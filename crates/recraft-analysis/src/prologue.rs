// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Fallback function-start discovery for images without an exact function table.
//!
//! PE32 has no `.pdata`, and a stripped image may have neither symbols nor unwind
//! data. In that case the only evidence left for a function that is never called
//! directly (a thread start routine passed as an immediate, a callback stored in
//! code, dead code kept by the linker) is its prologue. This pass walks the code
//! regions no seed covers, and accepts a start only when
//!
//! 1. a plausible prologue begins there (`push` of a callee-saved register, a
//!    frameless `sub esp, imm`, or the hotpatch `mov edi, edi`),
//! 2. it follows a `ret`/`jmp` or alignment padding (never the middle of an
//!    instruction run), and
//! 3. a trial walk of its control-flow graph decodes into a real instruction run —
//!    either it reaches a `ret`, or (packed stubs and thread routines can never
//!    return) it is long enough that a desynchronised data blob is implausible.
//!
//! The third condition is what keeps jump tables and other data inside `.text`
//! from turning into phantom functions: random bytes decode to a `ret` only when
//! the walk actually forms a straight line, not just when one byte equals `0xC3`.

use std::collections::{BTreeMap, HashMap};

use recraft_common::Address;
use recraft_core::Session;
use recraft_decode::iced_x86::{Mnemonic, OpKind, Register};
use recraft_decode::{FlowKind, Insn};

use super::{AnalysisOptions, ImportTags, Known, scan_function};

/// Instructions a trial walk may decode before it rejects a candidate. Real
/// functions in a prologue-less binary are far below this; the cap only bounds
/// the work for a candidate inside a data blob.
const TRIAL_INSTRUCTION_BUDGET: usize = 4096;

/// Shortest accepted trial that reaches a `ret`.
const MIN_TRIAL_INSTRUCTIONS: usize = 4;
/// Shortest accepted trial that does not: a routine whose only exits are tail
/// jumps or calls that never return still leaves plenty of instructions behind.
const MIN_NORETURN_INSTRUCTIONS: usize = 8;

/// Max prologue-recovered functions per image. A pathological binary (a huge
/// `.text` full of padding-like data) must not turn analysis into a delay.
const MAX_PROLOGUE_FUNCTIONS: usize = 4096;

/// Bytes one discovery pass may sweep. Every byte of a gap is decoded at most
/// once, so this bounds the pass at a few seconds of debug-build work even for a
/// 1 GiB image with a stripped, enormous `.text`.
const MAX_PROLOGUE_SWEEP_BYTES: u64 = 8 * 1024 * 1024;

/// A function start recovered by the prologue fallback.
pub(super) struct PrologueCandidate {
    pub start: Address,
    pub end: Address,
}

/// Find uncovered code ranges in executable segments and try to start a function
/// in each. `known` is the coverage collected by the main scan (every seed there
/// has already been walked).
pub(super) fn discover_prologue_starts(
    session: &mut Session,
    segments: &[recraft_core::Segment],
    known: &BTreeMap<u64, Known>,
) -> recraft_common::Result<Vec<PrologueCandidate>> {
    let covered = covered_ranges(known);
    let mut found = Vec::new();
    let mut budget = MAX_PROLOGUE_SWEEP_BYTES;

    for segment in segments {
        if budget == 0 {
            break;
        }
        if !segment.perm.is_executable() || segment.file_size == 0 {
            continue;
        }
        let start = segment.range.start.get();
        let end = start.saturating_add(segment.file_size.min(segment.len()));
        if end <= start {
            continue;
        }
        let mut cursor = start;
        for &(covered_start, covered_end) in &covered {
            if covered_end <= start || covered_start >= end {
                continue;
            }
            let gap_end = covered_start.min(end);
            if gap_end > cursor {
                scan_gap(session, segments, cursor, gap_end, &mut budget, &mut found)?;
            }
            cursor = cursor.max(covered_end.min(end));
            if cursor >= end {
                break;
            }
        }
        if cursor < end {
            scan_gap(session, segments, cursor, end, &mut budget, &mut found)?;
        }
        if found.len() >= MAX_PROLOGUE_FUNCTIONS {
            found.truncate(MAX_PROLOGUE_FUNCTIONS);
            break;
        }
    }
    Ok(found)
}

/// Merge the known function ranges into a sorted, non-overlapping coverage list.
fn covered_ranges(known: &BTreeMap<u64, Known>) -> Vec<(u64, u64)> {
    let mut ranges: Vec<(u64, u64)> = known
        .iter()
        .filter_map(|(&start, record)| {
            let end = record.end?;
            (end.get() > start).then_some((start, end.get()))
        })
        .collect();
    ranges.sort_unstable();
    let mut merged: Vec<(u64, u64)> = Vec::with_capacity(ranges.len());
    for (start, end) in ranges {
        match merged.last_mut() {
            Some((_, last_end)) if start <= *last_end => *last_end = (*last_end).max(end),
            _ => merged.push((start, end)),
        }
    }
    merged
}

/// Linear sweep over one uncovered range. A candidate is tried only at an
/// instruction boundary that follows a `ret`/`jmp` or alignment padding (or at
/// the very start of the range), so instruction operands that happen to begin
/// with `push ebp` are not mistaken for functions.
fn scan_gap(
    session: &mut Session,
    segments: &[recraft_core::Segment],
    mut addr: u64,
    gap_end: u64,
    budget: &mut u64,
    found: &mut Vec<PrologueCandidate>,
) -> recraft_common::Result<()> {
    let mut expect_start = true;
    while addr < gap_end && found.len() < MAX_PROLOGUE_FUNCTIONS && *budget > 0 {
        let Some(insn) = session.insn_at(Address::new(addr))? else {
            *budget = budget.saturating_sub(1);
            addr += 1;
            expect_start = false;
            continue;
        };
        let next = addr.saturating_add(u64::from(insn.len()).max(1));
        *budget = budget.saturating_sub(next - addr);
        if insn.is_invalid() {
            addr = next;
            expect_start = false;
            continue;
        }
        let padding = is_padding(&insn);
        if expect_start && !padding {
            if let Some(end) = try_candidate(session, segments, addr, gap_end)? {
                found.push(PrologueCandidate {
                    start: Address::new(addr),
                    end,
                });
                // The trial walk already covered the whole function; continue at
                // its end so alignment padding is walked as padding, not code.
                addr = end.get().max(next);
                expect_start = true;
                continue;
            }
            expect_start = false;
        }
        if matches!(
            insn.flow(),
            FlowKind::Return | FlowKind::UnconditionalBranch | FlowKind::IndirectBranch
        ) {
            expect_start = true;
        }
        addr = next;
    }
    Ok(())
}

fn try_candidate(
    session: &mut Session,
    segments: &[recraft_core::Segment],
    addr: u64,
    gap_end: u64,
) -> recraft_common::Result<Option<Address>> {
    let Some(insn) = session.insn_at(Address::new(addr))? else {
        return Ok(None);
    };
    if !looks_like_prologue(&insn) {
        return Ok(None);
    }
    let iat: HashMap<u64, (String, String)> = HashMap::new();
    let plt: HashMap<u64, (String, String)> = HashMap::new();
    let tags = ImportTags {
        iat: &iat,
        plt: &plt,
    };
    let mut instructions = 0u64;
    let output = scan_function(
        session,
        Address::new(addr),
        Some(Address::new(gap_end)),
        segments,
        &tags,
        &mut instructions,
        &AnalysisOptions::default(),
        TRIAL_INSTRUCTION_BUDGET,
    )?;
    if output.max_end <= Address::new(addr) {
        return Ok(None);
    }
    let minimum = if output.returns > 0 {
        MIN_TRIAL_INSTRUCTIONS
    } else {
        MIN_NORETURN_INSTRUCTIONS
    };
    if output.instructions < minimum {
        return Ok(None);
    }
    Ok(Some(output.max_end.min(Address::new(gap_end))))
}

/// Whether the instruction can open a function body. Deliberately narrow: a
/// random opcode inside an operand must not qualify.
pub(super) fn looks_like_prologue(insn: &Insn) -> bool {
    match insn.mnemonic() {
        Mnemonic::Push => is_callee_saved_push(insn),
        // Frameless function that only reserves locals (`sub esp, 0x28`).
        Mnemonic::Sub => {
            is_stack_pointer(insn.op_register(0))
                && insn.op_count() > 1
                && is_immediate(insn.op_kind(1))
        }
        // Stack realignment / VLA prologue (`lea esp, [esp-0x10]`).
        Mnemonic::Lea => {
            insn.op_count() > 1
                && is_stack_pointer(insn.op_register(0))
                && is_stack_pointer(insn.memory_base())
        }
        // MSVC hotpatch entry (`mov edi, edi`).
        Mnemonic::Mov => {
            insn.op_count() == 2
                && insn.op_register(0) == Register::EDI
                && insn.op_register(1) == Register::EDI
        }
        _ => false,
    }
}

fn is_callee_saved_push(insn: &Insn) -> bool {
    insn.op_count() > 0
        && insn.op_kind(0) == OpKind::Register
        && matches!(
            insn.op_register(0),
            Register::EBP
                | Register::EBX
                | Register::ESI
                | Register::EDI
                | Register::RBP
                | Register::RBX
                | Register::RSI
                | Register::RDI
                | Register::R12
                | Register::R13
                | Register::R14
                | Register::R15
        )
}

fn is_stack_pointer(register: Register) -> bool {
    matches!(register, Register::ESP | Register::RSP)
}

fn is_immediate(kind: OpKind) -> bool {
    matches!(
        kind,
        OpKind::Immediate8
            | OpKind::Immediate8to16
            | OpKind::Immediate8to32
            | OpKind::Immediate8to64
            | OpKind::Immediate16
            | OpKind::Immediate32
            | OpKind::Immediate32to64
            | OpKind::Immediate64
    )
}

/// Compiler padding between functions: no-op alignment and `int3` fill. MSVC/GCC
/// align with multi-byte nops that iced decodes as instructions rather than
/// `nop` (`lea esi, [esi]`, `xchg ax, ax`), so those count too.
pub(super) fn is_padding(insn: &Insn) -> bool {
    match insn.mnemonic() {
        Mnemonic::Nop | Mnemonic::Int3 => true,
        Mnemonic::Xchg => insn.op_count() == 2 && insn.op_register(0) == insn.op_register(1),
        Mnemonic::Lea => {
            insn.op_count() == 2
                && insn.op_kind(1) == OpKind::Memory
                && insn.memory_index() == Register::None
                && insn.memory_displacement64() == 0
                && insn.op_register(0) == insn.memory_base()
        }
        _ => false,
    }
}
