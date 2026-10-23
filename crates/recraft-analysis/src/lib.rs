// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Automatic analysis (M1).
//!
//! First deliverable: **function discovery** with recursive descent, seeded by the
//! entry point, exports, `.pdata` ranges and TLS callbacks. `.pdata` entries give exact
//! boundaries on x64 MSVC binaries; other functions are recovered by walking their
//! basic blocks until return/tail-call.
//!
//! Evaluation lives in the CLI (`recraft eval`) and uses exports + `.pdata` as ground
//! truth — see `corpus/` for the CRT-free benchmark binary.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use recraft_common::{Address, Result};
use recraft_core::Session;
use recraft_decode::FlowKind;

/// Where a discovered function start came from (for stats / UI).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionSource {
    Entry,
    Export,
    Pdata,
    Tls,
    CallTarget,
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
    pub instructions: u64,
    pub seeds: u64,
    pub discovered_by_call: u64,
}

struct Known {
    end: Option<Address>,
    source: FunctionSource,
    scanned: bool,
}

const MAX_INSTRUCTIONS_PER_FUNCTION: usize = 20_000;
const MAX_TOTAL_INSTRUCTIONS: u64 = 5_000_000;

/// Run function discovery over an opened session.
pub fn analyze(session: &mut Session) -> Result<AnalysisResult> {
    let entry = session.entry_point();
    let segments = session.segments().to_vec();
    let image = session.image().clone();

    let mut known: BTreeMap<u64, Known> = BTreeMap::new();
    let mut queue: VecDeque<Address> = VecDeque::new();

    // 1. Exact seeds first: .pdata (x64 unwind info gives start and end).
    for range in &image.pdata_functions {
        if range.start.get() < range.end.get() {
            seed(
                &segments,
                range.start,
                Some(range.end),
                FunctionSource::Pdata,
                &mut known,
                &mut queue,
            );
        }
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

    let initial_seeds = known.len() as u64;

    let mut calls: Vec<(Address, Address)> = Vec::new();
    let mut instruction_total: u64 = 0;
    let mut discovered_by_call: u64 = 0;

    while let Some(start) = queue.pop_front() {
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

        let (max_end, function_calls, new_starts) =
            scan_function(session, start, limit, &segments, &mut instruction_total)?;

        calls.extend(function_calls);

        if let Some(record) = known.get_mut(&start.get())
            && record.end.is_none()
            && max_end > start
        {
            record.end = Some(max_end);
        }

        for target in new_starts {
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

    Ok(AnalysisResult {
        functions,
        calls,
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
        FunctionSource::Pdata => 4,
        FunctionSource::Entry => 3,
        FunctionSource::Export => 2,
        FunctionSource::Tls => 1,
        FunctionSource::CallTarget => 0,
    }
}

fn is_code(segments: &[recraft_core::Segment], addr: Address) -> bool {
    segments
        .iter()
        .any(|segment| segment.perm.is_executable() && segment.contains(addr))
}

/// Walk one function's basic blocks. Returns the furthest instruction end, the direct
/// calls seen, and branch targets that leave the function (tail calls / callbacks).
#[allow(clippy::type_complexity)]
fn scan_function(
    session: &mut Session,
    start: Address,
    limit: Option<Address>,
    segments: &[recraft_core::Segment],
    instruction_total: &mut u64,
) -> Result<(Address, Vec<(Address, Address)>, Vec<Address>)> {
    let mut visited: BTreeSet<u64> = BTreeSet::new();
    let mut stack: Vec<Address> = vec![start];
    let mut max_end = start;
    let mut calls = Vec::new();
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
        let next = insn.next_addr();
        if next > max_end {
            max_end = next;
        }

        match insn.flow() {
            FlowKind::Next => stack.push(next),
            FlowKind::Call => {
                if let Some(target) = insn.branch_target() {
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
                    if is_inside(limit, start, target) {
                        stack.push(target);
                    } else {
                        leaving.insert(target.get());
                    }
                }
            }
            FlowKind::IndirectBranch
            | FlowKind::IndirectCall
            | FlowKind::Return
            | FlowKind::Interrupt
            | FlowKind::Trap
            | FlowKind::Invalid => {}
        }
    }

    Ok((
        max_end,
        calls,
        leaving.into_iter().map(Address::new).collect(),
    ))
}

fn is_inside(limit: Option<Address>, start: Address, target: Address) -> bool {
    match limit {
        Some(end) => target >= start && target < end,
        None => target >= start,
    }
}

/// Persist discovered functions into the session's function table.
pub fn persist(session: &mut Session, result: &AnalysisResult) -> Result<()> {
    let ranges: Vec<(Address, Address)> = result
        .functions
        .iter()
        .map(|function| (function.start, function.end))
        .collect();
    session.replace_auto_functions(&ranges)
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

    fn analyze_bytes(bytes: &[u8]) -> (AnalysisResult, Session, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("analysis.rcraft");
        let mut session = Session::create(&project, bytes, &mut |_, _| {}, &|| false).unwrap();
        let result = analyze(&mut session).unwrap();
        (result, session, dir)
    }

    #[test]
    fn corpus_recall_precision_and_exact_boundaries() {
        let Some(path) = corpus_path() else {
            return;
        };
        let bytes = std::fs::read(path).unwrap();
        let (result, session, _dir) = analyze_bytes(&bytes);
        let image = session.image();

        let mut ground_truth: BTreeSet<u64> = BTreeSet::new();
        ground_truth.insert(session.entry_point().get());
        for export in &image.exports {
            ground_truth.insert(image.image_base.get() + u64::from(export.rva));
        }
        for range in &image.pdata_functions {
            ground_truth.insert(range.start.get());
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

        // Every .pdata function must keep its exact end.
        for range in &image.pdata_functions {
            let found = result
                .functions
                .iter()
                .find(|function| function.start == range.start)
                .expect("pdata function missing");
            assert_eq!(
                found.end,
                range.end,
                "boundary mismatch at {:#x}",
                range.start.get()
            );
        }
    }
}
