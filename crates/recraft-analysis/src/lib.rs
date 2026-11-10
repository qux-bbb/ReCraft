// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Automatic analysis (M1).
//!
//! - **Function discovery**: seeds from `.pdata`, exports, the entry point and TLS
//!   callbacks; recursive descent follows direct branches, direct calls become new
//!   function seeds, and calls/jumps leaving the function are treated as tail calls.
//! - **Cross-references**: direct call edges, branch edges and data references from
//!   absolute/RIP-relative memory operands.
//! - **Strings**: ASCII and UTF-16LE extraction with IDA-compatible defaults (see
//!   [`strings`]).
//!
//! Evaluation lives in the CLI (`recraft eval`) and uses exports + `.pdata` as ground
//! truth — see `corpus/` for the CRT-free benchmark binary.

#![forbid(unsafe_code)]

mod api;
mod boundaries;
mod cfg;
mod golang;
mod strings;

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

use recraft_common::{Address, AddressRange, Result};
use recraft_core::Session;
use recraft_decode::FlowKind;

pub use api::{categorize, categorize_path};
pub use boundaries::merge_pdata_functions;
pub use cfg::{BasicBlock, Cfg, CfgInsn, EdgeKind, build_cfg, layers};
pub use golang::{GoFunction, parse_pclntab};
pub use strings::{
    DEFAULT_MIN_STRING_LEN, ExtractedString, MIN_STRING_LEN_CEILING, MIN_STRING_LEN_FLOOR,
    MIN_STRING_LEN_META_KEY, STRING_ENC_ASCII, STRING_ENC_UTF16LE, STRING_KIND_GENERIC,
    StringScanOptions, clamp_min_string_len, extract_strings, extract_strings_with,
    persist_strings, set_string_min_len, string_min_len,
};

/// `xref.type` for a direct call.
pub const XREF_CALL: i64 = 1;
/// `xref.type` for a branch (conditional or unconditional).
pub const XREF_JUMP: i64 = 2;
/// `xref.type` for a data reference (absolute / RIP-relative memory operand).
pub const XREF_DATA: i64 = 3;

/// Where a discovered function start came from (for stats / UI).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionSource {
    Entry,
    Export,
    Pdata,
    Tls,
    CallTarget,
    /// Recovered from a Go `pclntab` (exact start and real name).
    GoPclntab,
    /// Function pointer found in a readable data segment (dispatch tables, callbacks).
    DataPointer,
    /// Seed for an incremental re-analysis of a user-edited code range.
    UserRange,
}

/// One function recovered by analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnalyzedFunction {
    pub start: Address,
    pub end: Address,
    pub source: FunctionSource,
}

/// Everything the analysis pass produced.
#[derive(Debug, Clone, Default)]
pub struct AnalysisResult {
    pub functions: Vec<AnalyzedFunction>,
    /// Direct call edges observed while walking function bodies.
    pub calls: Vec<(Address, Address)>,
    /// Branch edges observed while walking function bodies.
    pub jumps: Vec<(Address, Address)>,
    /// Data references (absolute / RIP-relative memory operands into the image).
    pub data_refs: Vec<(Address, Address)>,
    /// API tags: (instruction address, qualified import name, category).
    pub api_refs: Vec<(Address, String, String)>,
    /// Names recovered from a Go `pclntab` (function start -> name).
    pub go_names: BTreeMap<u64, String>,
    /// Number of functions recovered from the Go `pclntab`.
    pub go_functions: usize,
    pub instructions: u64,
    pub seeds: u64,
    pub discovered_by_call: u64,
}

struct Known {
    end: Option<Address>,
    source: FunctionSource,
    scanned: bool,
}

struct ScanOutput {
    max_end: Address,
    calls: Vec<(Address, Address)>,
    jumps: Vec<(Address, Address)>,
    data_refs: Vec<(Address, Address)>,
    api_refs: Vec<(Address, String, String)>,
    leaving: Vec<Address>,
}

const MAX_INSTRUCTIONS_PER_FUNCTION: usize = 20_000;
const MAX_TOTAL_INSTRUCTIONS: u64 = 5_000_000;
const MAX_DATA_POINTERS: usize = 65_536;

/// Options for a (possibly long-running) analysis pass.
#[derive(Clone, Default)]
pub struct AnalysisOptions {
    /// Cooperative cancellation flag, checked between functions and periodically while
    /// walking a function body. The caller may set it from another thread.
    pub cancel: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    /// Restrict the initial seeds to this range (incremental re-analysis, DESIGN §6.4).
    pub seed_range: Option<AddressRange>,
}

impl AnalysisOptions {
    fn cancelled(&self) -> bool {
        self.cancel
            .as_ref()
            .is_some_and(|flag| flag.load(std::sync::atomic::Ordering::Relaxed))
    }
}

/// Run function discovery with default options.
pub fn analyze(session: &mut Session) -> Result<AnalysisResult> {
    analyze_with_options(session, &AnalysisOptions::default())
}

/// Run function discovery (plus call/jump/data xref collection) over an opened session.
pub fn analyze_with_options(
    session: &mut Session,
    options: &AnalysisOptions,
) -> Result<AnalysisResult> {
    let entry = session.entry_point();
    let segments = session.segments().to_vec();
    let image = session.image().clone();

    let mut known: BTreeMap<u64, Known> = BTreeMap::new();
    let mut queue: VecDeque<Address> = VecDeque::new();

    // 1. Exact seeds first: .pdata (x64 unwind info gives start and end), with cold
    //    fragments merged back into their owning function.
    let raw_pdata: Vec<AddressRange> = image
        .pdata_functions
        .iter()
        .map(|range| AddressRange::new(range.start, range.end))
        .collect();
    // Cold-fragment merging is a PE `.pdata` concept: it re-attaches fragments that
    // fall through into the next recorded range. ELF ranges come from `.symtab` /
    // `.eh_frame` and are already exact, so they are taken verbatim.
    let pdata_ranges = if session.format() == recraft_core::Format::Pe {
        merge_pdata_functions(session, &raw_pdata)?
    } else {
        raw_pdata
            .iter()
            .map(|range| (range.start, range.end))
            .collect()
    };
    for (start, end) in &pdata_ranges {
        seed(
            &segments,
            *start,
            Some(*end),
            FunctionSource::Pdata,
            &mut known,
            &mut queue,
        );
    }
    // 2. Entry point and exports.
    for export in &image.exports {
        let va = image.image_base.get().saturating_add(u64::from(export.rva));
        seed(
            &segments,
            Address::new(va),
            None,
            FunctionSource::Export,
            &mut known,
            &mut queue,
        );
    }
    seed(
        &segments,
        entry,
        None,
        FunctionSource::Entry,
        &mut known,
        &mut queue,
    );
    // 3. TLS callbacks.
    for callback in &image.tls_callbacks {
        seed(
            &segments,
            *callback,
            None,
            FunctionSource::Tls,
            &mut known,
            &mut queue,
        );
    }

    // 4. Go runtime symbol table: exact function starts plus real names.
    let mut go_names: BTreeMap<u64, String> = BTreeMap::new();
    let go_functions = match parse_pclntab(session)? {
        Some(functions) => {
            for function in &functions {
                go_names.insert(function.start.get(), function.name.clone());
                seed(
                    &segments,
                    function.start,
                    None,
                    FunctionSource::GoPclntab,
                    &mut known,
                    &mut queue,
                );
            }
            functions.len()
        }
        None => 0,
    };

    // 5. Function-pointer tables: aligned pointers in readable data segments. This
    // catches dispatch tables and callbacks that are never called directly.
    let pointer_size = if session.mode().is_64() { 8usize } else { 4 };
    let mut data_candidates: BTreeSet<u64> = BTreeSet::new();
    let mut chunk = vec![0u8; 64 * 1024];
    for segment in &segments {
        if !segment.perm.is_readable() || segment.perm.is_executable() || segment.file_size == 0 {
            continue;
        }
        let size = segment.file_size.min(segment.len());
        let mut offset = 0u64;
        while offset + pointer_size as u64 <= size && data_candidates.len() < MAX_DATA_POINTERS {
            let want = (((size - offset) as usize).min(chunk.len())) & !(pointer_size - 1);
            if want < pointer_size {
                break;
            }
            let read = session.read_at(
                Address::new(segment.range.start.get() + offset),
                &mut chunk[..want],
            )?;
            let read = read & !(pointer_size - 1);
            if read < pointer_size {
                break;
            }
            for index in (0..=read - pointer_size).step_by(pointer_size) {
                let mut raw = [0u8; 8];
                raw[..pointer_size].copy_from_slice(&chunk[index..index + pointer_size]);
                let value = u64::from_le_bytes(raw);
                if is_code(&segments, Address::new(value)) {
                    data_candidates.insert(value);
                }
            }
            offset += read as u64;
        }
    }
    for candidate in data_candidates {
        seed(
            &segments,
            Address::new(candidate),
            None,
            FunctionSource::DataPointer,
            &mut known,
            &mut queue,
        );
    }

    // Incremental mode: keep only seeds inside the requested range, and make the range
    // start itself a seed so user-marked code is scanned even without a known prologue.
    if let Some(range) = options.seed_range {
        known.retain(|start, _| range.contains(Address::new(*start)));
        queue.clear();
        for start in known.keys() {
            queue.push_back(Address::new(*start));
        }
        if range.start < range.end
            && is_code(&segments, range.start)
            && !known.contains_key(&range.start.get())
        {
            known.insert(
                range.start.get(),
                Known {
                    end: None,
                    source: FunctionSource::UserRange,
                    scanned: false,
                },
            );
            queue.push_back(range.start);
        }
    }

    let initial_seeds = known.len() as u64;

    // Imported APIs by IAT slot: address -> (qualified name, category).
    let mut iat: HashMap<u64, (String, String)> = HashMap::new();
    for module in &image.imports {
        for function in &module.functions {
            if function.iat_va.get() == 0 {
                continue;
            }
            let api = match (&function.name, function.ordinal) {
                (Some(name), _) if !name.is_empty() => name.clone(),
                (_, Some(ordinal)) => format!("#{ordinal}"),
                _ => continue,
            };
            let qualified = format!("{}!{api}", module.name);
            iat.insert(
                function.iat_va.get(),
                (qualified, categorize(&api).to_owned()),
            );
        }
    }

    // ELF calls imported functions through 16-byte PLT stubs (`jmp [rip+got]`), so the
    // IAT slot is referenced by the stub, not by the caller. Without this map an ELF
    // sample only sees the handful of imports it loads directly (`-fno-plt` style);
    // with it the tag lands on the call site, exactly like PE's `call [IAT]`.
    let plt = if session.format() == recraft_core::Format::Elf {
        plt_stubs(session, &segments, &iat)?
    } else {
        HashMap::new()
    };

    let mut calls: Vec<(Address, Address)> = Vec::new();
    let mut jumps: Vec<(Address, Address)> = Vec::new();
    let mut data_refs: Vec<(Address, Address)> = Vec::new();
    let mut api_refs: Vec<(Address, String, String)> = Vec::new();
    let mut instruction_total: u64 = 0;
    let mut discovered_by_call: u64 = 0;

    while let Some(start) = queue.pop_front() {
        if options.cancelled() {
            return Err(recraft_common::Error::Cancelled);
        }
        let Some(record) = known.get(&start.get()) else {
            continue;
        };
        if record.scanned {
            continue;
        }
        let limit = record.end;
        if let Some(record) = known.get_mut(&start.get()) {
            record.scanned = true;
        }

        let output = scan_function(
            session,
            start,
            limit,
            &segments,
            &ImportTags {
                iat: &iat,
                plt: &plt,
            },
            &mut instruction_total,
            options,
        )?;

        calls.extend(output.calls);
        jumps.extend(output.jumps);
        data_refs.extend(output.data_refs);
        api_refs.extend(output.api_refs);

        if let Some(record) = known.get_mut(&start.get())
            && record.end.is_none()
            && output.max_end > start
        {
            record.end = Some(output.max_end);
        }

        for target in output.leaving {
            if target <= start || known.contains_key(&target.get()) {
                continue;
            }
            if !is_code(&segments, target) {
                continue;
            }
            // Never invent a function in the middle of an already known one.
            let inside_known = known.iter().any(|(&other_start, record)| {
                other_start != target.get()
                    && record
                        .end
                        .is_some_and(|end| target >= Address::new(other_start) && target < end)
            });
            if inside_known {
                continue;
            }
            known.insert(
                target.get(),
                Known {
                    end: None,
                    source: FunctionSource::CallTarget,
                    scanned: false,
                },
            );
            discovered_by_call += 1;
            queue.push_back(target);
        }

        if instruction_total > MAX_TOTAL_INSTRUCTIONS {
            break;
        }
    }

    let mut functions: Vec<AnalyzedFunction> = known
        .into_iter()
        .filter_map(|(start, record)| {
            let end = record.end?;
            let start = Address::new(start);
            if end <= start {
                return None;
            }
            Some(AnalyzedFunction {
                start,
                end,
                source: record.source,
            })
        })
        .collect();
    functions.sort_by_key(|function| function.start);

    // Function ranges must not overlap: a scan can follow a tail jump or fallthrough into
    // the next function's code, producing a bogus multi-function range. Clamp each end to
    // the next known start (IDA-style non-overlapping ranges) and drop empty results.
    for index in 0..functions.len().saturating_sub(1) {
        if functions[index].end > functions[index + 1].start {
            functions[index].end = functions[index + 1].start;
        }
    }
    functions.retain(|function| function.end > function.start);

    // Recovered ranges often include the alignment padding that follows the last real
    // instruction (MSVC pads with `int3`, occasionally `nop`). Trim it so decompilation,
    // metrics and xrefs never treat padding as code. Keep at least one byte per function.
    for function in &mut functions {
        let mut end = function.end;
        while end.get() > function.start.get() + 1 {
            let Ok(byte) = session.read_bytes(Address::new(end.get() - 1), 1) else {
                break;
            };
            match byte[0] {
                0xCC | 0x90 => end = Address::new(end.get() - 1),
                _ => break,
            }
        }
        function.end = end;
    }

    Ok(AnalysisResult {
        functions,
        calls,
        jumps,
        data_refs,
        api_refs,
        go_names,
        go_functions,
        instructions: instruction_total,
        seeds: initial_seeds,
        discovered_by_call,
    })
}

fn seed(
    segments: &[recraft_core::Segment],
    addr: Address,
    end: Option<Address>,
    source: FunctionSource,
    known: &mut BTreeMap<u64, Known>,
    queue: &mut VecDeque<Address>,
) {
    if !is_code(segments, addr) {
        return;
    }
    match known.get_mut(&addr.get()) {
        Some(existing) => {
            if existing.end.is_none() {
                existing.end = end;
            }
            if source_rank(source) > source_rank(existing.source) {
                existing.source = source;
            }
        }
        None => {
            known.insert(
                addr.get(),
                Known {
                    end,
                    source,
                    scanned: false,
                },
            );
            queue.push_back(addr);
        }
    }
}

fn source_rank(source: FunctionSource) -> u8 {
    match source {
        FunctionSource::Pdata => 5,
        FunctionSource::GoPclntab => 4,
        FunctionSource::Entry => 3,
        FunctionSource::Export => 2,
        FunctionSource::Tls => 1,
        FunctionSource::DataPointer => 1,
        FunctionSource::CallTarget => 0,
        FunctionSource::UserRange => 3,
    }
}

fn is_code(segments: &[recraft_core::Segment], addr: Address) -> bool {
    segments
        .iter()
        .any(|segment| segment.perm.is_executable() && segment.contains(addr))
}

fn is_mapped(segments: &[recraft_core::Segment], addr: Address) -> bool {
    segments.iter().any(|segment| segment.contains(addr))
}

/// Map every PLT stub entry address to the import it jumps through.
///
/// On x86-64 a PLT entry is a short stub ending in `ff 25 <disp32>` (`jmp [rip+disp]`);
/// the jump target is the import's GOT slot, i.e. a key of `iat`. CET builds put
/// `endbr64` (optionally with a `bnd` prefix) in front of the jump, so the entry is
/// recorded under each plausible prefix length. A literal pattern scan over the
/// executable segments is enough — `.plt` does not have to be parsed.
fn plt_stubs(
    session: &mut Session,
    segments: &[recraft_core::Segment],
    iat: &HashMap<u64, (String, String)>,
) -> Result<HashMap<u64, (String, String)>> {
    let mut stubs: HashMap<u64, (String, String)> = HashMap::new();
    let mut chunk = vec![0u8; 64 * 1024];
    for segment in segments {
        if !segment.perm.is_executable() || segment.file_size == 0 {
            continue;
        }
        let size = segment.file_size.min(segment.len());
        let mut offset = 0u64;
        while offset < size {
            let want = ((size - offset) as usize).min(chunk.len());
            if want < 6 {
                break;
            }
            let read = session.read_at(
                Address::new(segment.range.start.get() + offset),
                &mut chunk[..want],
            )?;
            if read < 6 {
                break;
            }
            scan_plt_bytes(
                &chunk[..read],
                segment.range.start.get() + offset,
                iat,
                &mut stubs,
            );
            // Overlap by five bytes so a `jmp [rip+disp]` cut by the chunk edge is seen.
            offset += read as u64 - 5;
        }
    }
    Ok(stubs)
}

/// Record the `jmp [rip+disp]` thunks in one window of executable bytes whose target is a
/// known import slot, together with the entry address a `call` would land on.
fn scan_plt_bytes(
    bytes: &[u8],
    base: u64,
    iat: &HashMap<u64, (String, String)>,
    out: &mut HashMap<u64, (String, String)>,
) {
    if bytes.len() < 6 {
        return;
    }
    for index in 0..=bytes.len() - 6 {
        if bytes[index] != 0xFF || bytes[index + 1] != 0x25 {
            continue;
        }
        let disp = i32::from_le_bytes([
            bytes[index + 2],
            bytes[index + 3],
            bytes[index + 4],
            bytes[index + 5],
        ]);
        let insn = base + index as u64;
        let target = insn.wrapping_add(6).wrapping_add_signed(i64::from(disp));
        let Some(tag) = iat.get(&target) else {
            continue;
        };
        out.insert(insn, tag.clone());
        // A call lands on the entry start, which may carry prefixes in front of the
        // jump (`endbr64`, `bnd`, `notrack`). Everything within five bytes before the
        // `ff 25` belongs to the stub, so alias those addresses to the same import.
        for back in 1..=5usize {
            if index >= back {
                out.insert(insn - back as u64, tag.clone());
            }
        }
    }
}

/// The two ways an instruction references an import: a direct IAT/GOT memory operand,
/// or a call to a PLT stub that jumps through one.
struct ImportTags<'a> {
    iat: &'a HashMap<u64, (String, String)>,
    plt: &'a HashMap<u64, (String, String)>,
}

/// Walk one function's basic blocks.
fn scan_function(
    session: &mut Session,
    start: Address,
    limit: Option<Address>,
    segments: &[recraft_core::Segment],
    tags: &ImportTags<'_>,
    instruction_total: &mut u64,
    options: &AnalysisOptions,
) -> Result<ScanOutput> {
    let mut visited: BTreeSet<u64> = BTreeSet::new();
    let mut stack: Vec<Address> = vec![start];
    let mut max_end = start;
    let mut calls = Vec::new();
    let mut jumps = Vec::new();
    let mut data_refs = Vec::new();
    let mut api_refs = Vec::new();
    let mut leaving: BTreeSet<u64> = BTreeSet::new();
    let mut local_instructions = 0usize;

    while let Some(addr) = stack.pop() {
        if !visited.insert(addr.get()) {
            continue;
        }

        if !is_code(segments, addr) {
            continue;
        }
        if limit.is_some_and(|end| addr >= end) {
            leaving.insert(addr.get());
            continue;
        }
        if local_instructions >= MAX_INSTRUCTIONS_PER_FUNCTION {
            break;
        }
        let Some(insn) = session.insn_at(addr)? else {
            continue;
        };
        local_instructions += 1;
        *instruction_total += 1;
        if local_instructions % 1024 == 0 && options.cancelled() {
            return Err(recraft_common::Error::Cancelled);
        }
        let next = insn.next_addr();
        if next > max_end {
            max_end = next;
        }

        if insn.memory_is_direct() {
            let target = Address::new(insn.memory_displacement64());
            if target.get() != 0 && is_mapped(segments, target) {
                data_refs.push((addr, target));
            }
            if let Some((qualified, category)) = tags.iat.get(&target.get()) {
                api_refs.push((addr, qualified.clone(), category.clone()));
            }
        }

        match insn.flow() {
            FlowKind::Next => stack.push(next),
            FlowKind::Call => {
                if let Some(target) = insn.branch_target() {
                    // A call to a PLT stub is this sample's import reference.
                    if let Some((qualified, category)) = tags.plt.get(&target.get()) {
                        api_refs.push((addr, qualified.clone(), category.clone()));
                    }
                    calls.push((addr, target));
                    // Direct call targets are functions even when we do not know this
                    // function's end yet; the caller filters out known-interior targets.
                    if target != start {
                        leaving.insert(target.get());
                    }
                }
                stack.push(next);
            }
            FlowKind::ConditionalBranch => {
                if let Some(target) = insn.branch_target() {
                    jumps.push((addr, target));
                    if is_inside(limit, start, target) {
                        stack.push(target);
                    } else {
                        leaving.insert(target.get());
                    }
                }
                stack.push(next);
            }
            FlowKind::UnconditionalBranch => {
                if let Some(target) = insn.branch_target() {
                    jumps.push((addr, target));
                    if is_inside(limit, start, target) {
                        stack.push(target);
                    } else {
                        leaving.insert(target.get());
                    }
                }
            }
            FlowKind::IndirectCall => {
                // A call returns to the next instruction, even when its target is only
                // known at run time. Treating `call [IAT]` as a dead end stopped every
                // function at its first imported-API call, so all later API tags (and
                // the blocks after them) were missing from the database.
                stack.push(next);
            }
            FlowKind::IndirectBranch
            | FlowKind::Return
            | FlowKind::Interrupt
            | FlowKind::Trap
            | FlowKind::Invalid => {}
        }
    }

    Ok(ScanOutput {
        max_end,
        calls,
        jumps,
        data_refs,
        api_refs,
        leaving: leaving.into_iter().map(Address::new).collect(),
    })
}

fn is_inside(limit: Option<Address>, start: Address, target: Address) -> bool {
    match limit {
        Some(end) => target >= start && target < end,
        None => target >= start,
    }
}

/// Persist discovered functions, names, xrefs and API tags into the session
/// (all derived data).
/// Analyze only `range`: incremental re-analysis (DESIGN §6.4). Seeds outside the range
/// are ignored and the range start is used as an extra code seed.
pub fn analyze_range(session: &mut Session, range: AddressRange) -> Result<AnalysisResult> {
    let options = AnalysisOptions {
        seed_range: Some(range),
        ..AnalysisOptions::default()
    };
    analyze_with_options(session, &options)
}

/// Recompute the functions/xrefs/names/API tags intersecting `range` and merge them back,
/// leaving everything outside the range untouched.
pub fn reanalyze_range(session: &mut Session, range: AddressRange) -> Result<AnalysisResult> {
    let result = analyze_range(session, range)?;
    let intersects = |start: Address, end: Address| start < range.end && end > range.start;
    let functions: Vec<(Address, Address)> = result
        .functions
        .iter()
        .filter(|function| intersects(function.start, function.end))
        .map(|function| (function.start, function.end))
        .collect();
    session.replace_auto_functions_in_range(range, &functions)?;
    let names: Vec<(Address, String)> = functions
        .iter()
        .map(|(start, _)| {
            let name = result
                .go_names
                .get(&start.get())
                .cloned()
                .unwrap_or_else(|| format!("sub_{:x}", start.get()));
            (*start, name)
        })
        .collect();
    session.replace_auto_names_in_range(range, &names)?;
    let mut xrefs: Vec<(Address, Address, i64)> = Vec::new();
    xrefs.extend(
        result
            .calls
            .iter()
            .filter(|(from, _)| range.contains(*from))
            .map(|(from, to)| (*from, *to, XREF_CALL)),
    );
    xrefs.extend(
        result
            .jumps
            .iter()
            .filter(|(from, _)| range.contains(*from))
            .map(|(from, to)| (*from, *to, XREF_JUMP)),
    );
    xrefs.extend(
        result
            .data_refs
            .iter()
            .filter(|(from, _)| range.contains(*from))
            .map(|(from, to)| (*from, *to, XREF_DATA)),
    );
    session.replace_auto_xrefs_in_range(range, &xrefs)?;
    let tags: Vec<(Address, String, String)> = result
        .api_refs
        .iter()
        .filter(|(addr, _, _)| range.contains(*addr))
        .cloned()
        .collect();
    session.replace_auto_api_tags_in_range(range, &tags)?;
    Ok(result)
}

pub fn persist(session: &mut Session, result: &AnalysisResult) -> Result<()> {
    let ranges: Vec<(Address, Address)> = result
        .functions
        .iter()
        .map(|function| (function.start, function.end))
        .collect();
    session.replace_auto_functions(&ranges)?;

    // Give every unnamed function an IDA-style `sub_...` label, preferring names
    // recovered from a Go pclntab. Loader/user names win because insertion uses
    // INSERT OR IGNORE.
    let names: Vec<(Address, String)> = result
        .functions
        .iter()
        .map(|function| {
            let name = result
                .go_names
                .get(&function.start.get())
                .cloned()
                .unwrap_or_else(|| format!("sub_{:x}", function.start.get()));
            (function.start, name)
        })
        .collect();
    session.replace_auto_names(&names)?;

    let mut xrefs: Vec<(Address, Address, i64)> =
        Vec::with_capacity(result.calls.len() + result.jumps.len() + result.data_refs.len());
    xrefs.extend(
        result
            .calls
            .iter()
            .map(|(from, to)| (*from, *to, XREF_CALL)),
    );
    xrefs.extend(
        result
            .jumps
            .iter()
            .map(|(from, to)| (*from, *to, XREF_JUMP)),
    );
    xrefs.extend(
        result
            .data_refs
            .iter()
            .map(|(from, to)| (*from, *to, XREF_DATA)),
    );
    session.replace_auto_xrefs(&xrefs)?;
    session.replace_auto_api_tags(&result.api_refs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn corpus_path() -> Option<PathBuf> {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../corpus/build/x64/corpus.exe");
        path.exists().then_some(path)
    }

    /// Every function symbol in the MSVC linker map (`/MAP`), independent of exports
    /// and `.pdata`; this is the ground truth for heuristic-only functions.
    fn map_ground_truth() -> Option<BTreeSet<u64>> {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../corpus/build/x64/corpus.map");
        let text = std::fs::read_to_string(path).ok()?;
        let mut starts = BTreeSet::new();
        for line in text.lines() {
            let tokens: Vec<&str> = line.split_whitespace().collect();
            if tokens.len() >= 4
                && tokens[3] == "f"
                && let Ok(va) = u64::from_str_radix(tokens[2], 16)
            {
                starts.insert(va);
            }
        }
        (!starts.is_empty()).then_some(starts)
    }

    fn analyze_bytes(bytes: &[u8]) -> (AnalysisResult, Session, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("analysis.rcraft");
        let mut session = Session::create(&project, bytes, &mut |_, _| {}, &|| false).unwrap();
        let result = analyze(&mut session).unwrap();
        (result, session, dir)
    }

    #[test]
    fn incremental_reanalysis_is_range_scoped() {
        let Some(path) = corpus_path() else {
            return;
        };
        let bytes = std::fs::read(path).unwrap();
        let (result, mut session, _dir) = analyze_bytes(&bytes);
        persist(&mut session, &result).unwrap();
        let before = session.functions().unwrap();
        // Pick a function with code on both sides so "unaffected" is meaningful.
        let target = result
            .functions
            .iter()
            .find(|function| {
                function.start > before.first().unwrap().start
                    && function.end < before.last().unwrap().start
            })
            .copied()
            .unwrap_or_else(|| {
                panic!(
                    "no interior function among {} (first={:?} last={:?})",
                    result.functions.len(),
                    before.first(),
                    before.last()
                )
            });
        let range = AddressRange::new(target.start, target.end);
        let rerun = reanalyze_range(&mut session, range).unwrap();
        assert!(
            rerun
                .functions
                .iter()
                .all(|function| function.start < range.end && function.end > range.start),
            "reanalysis leaked functions outside the range"
        );
        let after = session.functions().unwrap();
        assert!(
            after.iter().all(|function| function.end > function.start),
            "invalid function range after merge"
        );
        let unaffected = before
            .iter()
            .find(|function| function.start < range.start)
            .expect("function before the range");
        assert!(
            after.contains(unaffected),
            "function before the range changed: {unaffected:?}"
        );
        let unaffected_after = before
            .iter()
            .find(|function| function.start > range.end)
            .expect("function after the range");
        assert!(
            after.contains(unaffected_after),
            "function after the range changed: {unaffected_after:?}"
        );
    }

    #[test]
    fn corpus_recall_precision_and_exact_boundaries() {
        let Some(path) = corpus_path() else {
            return;
        };
        let bytes = std::fs::read(path).unwrap();
        let (result, mut session, _dir) = analyze_bytes(&bytes);
        let image = session.image().clone();
        let raw_pdata: Vec<AddressRange> = image
            .pdata_functions
            .iter()
            .map(|range| AddressRange::new(range.start, range.end))
            .collect();
        let merged = merge_pdata_functions(&mut session, &raw_pdata).unwrap();

        let mut ground_truth: BTreeSet<u64> = BTreeSet::new();
        ground_truth.insert(session.entry_point().get());
        for export in &image.exports {
            ground_truth.insert(image.image_base.get() + u64::from(export.rva));
        }
        for (start, _) in &merged {
            ground_truth.insert(start.get());
        }
        // Prefer the linker map when present: it also covers leaf functions that have
        // neither an export nor a `.pdata` entry.
        if let Some(map_starts) = map_ground_truth() {
            ground_truth.extend(map_starts);
        }

        let detected: BTreeSet<u64> = result.functions.iter().map(|f| f.start.get()).collect();
        let matched = ground_truth.intersection(&detected).count();
        let recall = matched as f64 / ground_truth.len() as f64;
        let precision = matched as f64 / detected.len() as f64;

        assert!(
            recall > 0.95,
            "recall {recall:.3} ({matched}/{})",
            ground_truth.len()
        );
        assert!(
            precision > 0.95,
            "precision {precision:.3} ({matched}/{})",
            detected.len()
        );

        // Every merged .pdata function must keep its exact end.
        for (start, end) in &merged {
            let found = result
                .functions
                .iter()
                .find(|function| function.start == *start)
                .expect("pdata function missing");
            assert_eq!(found.end, *end, "boundary mismatch at {:#x}", start.get());
        }
    }

    #[test]
    fn corpus_has_string_xrefs() {
        let Some(path) = corpus_path() else {
            return;
        };
        let bytes = std::fs::read(path).unwrap();
        let (result, mut session, _dir) = analyze_bytes(&bytes);
        let strings = extract_strings(&mut session, 4, 10_000).unwrap();
        let banner = strings
            .iter()
            .find(|string| string.text == "ReCraft analysis corpus")
            .expect("banner string missing");

        assert!(
            result.data_refs.iter().any(|(_, to)| *to == banner.addr),
            "no data xref to the banner at {:#x}; refs: {:?}",
            banner.addr.get(),
            &result.data_refs[..result.data_refs.len().min(12)]
        );
        assert!(
            !result.calls.is_empty(),
            "expected direct call edges, got none"
        );
        assert!(
            !result.data_refs.is_empty(),
            "expected data references, got none"
        );
    }

    #[test]
    fn notepad_api_tags_and_auto_names() {
        let Ok(bytes) = std::fs::read("C:/Windows/System32/notepad.exe") else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("notepad-analyze.rcraft");
        let mut session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let result = analyze(&mut session).unwrap();

        assert!(
            result.api_refs.len() > 20,
            "only {} API references",
            result.api_refs.len()
        );
        let categories: BTreeSet<&str> = result
            .api_refs
            .iter()
            .map(|(_, _, category)| category.as_str())
            .collect();
        assert!(
            categories.contains("file")
                || categories.contains("process")
                || categories.contains("ui"),
            "unexpected categories: {categories:?}"
        );

        persist(&mut session, &result).unwrap();
        assert!(
            session.auto_name_count().unwrap() > 100,
            "auto names were not persisted"
        );

        let (site, rule, category) = &result.api_refs[0];
        let tags = session.api_tags_at(*site).unwrap();
        assert!(
            tags.iter()
                .any(|(tag_rule, tag_category)| tag_rule == rule && tag_category == category),
            "api tag missing at {:#x}",
            site.get()
        );
    }

    /// The PLT scan turns `jmp [rip+got]` stubs into call-site tags: a normal ELF build
    /// calls `open@plt`, so without this the import is never tagged in the function that
    /// uses it (only a stub or a `-fno-plt` load would carry the tag).
    #[test]
    fn plt_thunks_are_recognised_with_and_without_endbr() {
        let base = 0x1000u64;
        let got = 0x9000u64;
        // `endbr64; bnd jmp [rip+disp]` — the CET layout.
        let mut bytes = vec![0xF3, 0x0F, 0x1E, 0xFA, 0xF2, 0xFF, 0x25, 0, 0, 0, 0];
        let disp = (got as i64 - (base as i64 + 5 + 6)) as i32;
        bytes[7..11].copy_from_slice(&disp.to_le_bytes());
        let mut iat = HashMap::new();
        iat.insert(got, ("elf!open".to_owned(), "file".to_owned()));

        let mut stubs = HashMap::new();
        scan_plt_bytes(&bytes, base, &iat, &mut stubs);
        assert_eq!(
            stubs.get(&base).map(|(rule, _)| rule.as_str()),
            Some("elf!open")
        );

        // A plain `jmp [rip+disp]`, the layout without CET prefixes.
        let mut plain = vec![0xFF, 0x25, 0, 0, 0, 0];
        let disp = (got as i64 - (base as i64 + 6)) as i32;
        plain[2..6].copy_from_slice(&disp.to_le_bytes());
        let mut stubs = HashMap::new();
        scan_plt_bytes(&plain, base, &iat, &mut stubs);
        assert!(stubs.contains_key(&base));

        // A jump that does not land on an import slot is not a stub.
        let mut unknown = plain.clone();
        unknown[2..6].copy_from_slice(&0i32.to_le_bytes());
        let mut stubs = HashMap::new();
        scan_plt_bytes(&unknown, base, &iat, &mut stubs);
        assert!(stubs.is_empty());
    }

    #[test]
    fn analysis_can_be_cancelled() {
        let Some(path) = corpus_path() else {
            return;
        };
        let bytes = std::fs::read(path).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("cancel.rcraft");
        let mut session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        let options = AnalysisOptions {
            cancel: Some(flag),
            seed_range: None,
        };
        let error = analyze_with_options(&mut session, &options).unwrap_err();
        assert!(matches!(error, recraft_common::Error::Cancelled));
    }

    #[test]
    fn go_corpus_names_and_functions_are_recovered() {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../corpus/build/go/corpus-go.exe");
        let Ok(bytes) = std::fs::read(path) else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("go-analyze.rcraft");
        let mut session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let result = analyze(&mut session).unwrap();

        assert!(
            result.go_functions > 20,
            "only {} pclntab functions",
            result.go_functions
        );
        assert!(
            result.go_names.values().any(|name| name == "main.main"),
            "main.main was not recovered"
        );

        let detected: BTreeSet<u64> = result.functions.iter().map(|f| f.start.get()).collect();
        for start in result.go_names.keys() {
            assert!(
                detected.contains(start),
                "pclntab function at {start:#x} missing from the analysis result"
            );
        }

        persist(&mut session, &result).unwrap();
        let main_start = result
            .go_names
            .iter()
            .find(|(_, name)| *name == "main.main")
            .map(|(start, _)| *start)
            .unwrap();
        assert_eq!(
            session.addr_of_name("main.main").unwrap().map(|a| a.get()),
            Some(main_start)
        );
    }
    /// `call qword ptr [IAT]` returns to the next instruction, so the walk must continue:
    /// before this, every function stopped at its first imported API and the tag table
    /// only ever held one API per function (starving naming/triage evidence).
    #[test]
    fn indirect_calls_do_not_truncate_api_tags() {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../corpus/build/x64/naming.exe");
        if !path.exists() {
            return;
        }
        let bytes = std::fs::read(path).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("naming-tags.rcraft");
        let mut session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let result = analyze(&mut session).unwrap();
        let start = session
            .addr_of_name("open_config_file")
            .unwrap()
            .expect("exported benchmark function");
        let tags: Vec<String> = result
            .api_refs
            .iter()
            .filter(|(addr, _, _)| *addr >= start && addr.get() < start.get() + 0x200)
            .map(|(_, rule, _)| rule.clone())
            .collect();
        for api in ["CreateFileW", "ReadFile", "CloseHandle"] {
            assert!(
                tags.iter().any(|tag| tag.ends_with(api)),
                "{api} missing from {tags:?}"
            );
        }
    }
}
