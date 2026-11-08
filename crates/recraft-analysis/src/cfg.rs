// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Basic-block / control-flow graph construction (M1).
//!
//! Linear decode inside a known function range, split at branch targets and after
//! branch terminators, then connect blocks with typed edges. Jump tables (absolute
//! pointer tables and MSVC's 32-bit image-relative tables) are expanded into edges.

use std::collections::{BTreeMap, BTreeSet};

use iced_x86::Register;
use recraft_common::{Address, Result};
#[cfg(test)]
use recraft_core::Session;
use recraft_core::SessionView;
use recraft_decode::{FlowKind, Formatter, Insn};
use recraft_ir::{RegView, from_iced};

const MAX_FUNCTION_INSTRUCTIONS: usize = 20_000;
const MAX_TABLE_ENTRIES: usize = 256;

/// One instruction inside a basic block, with rendered text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CfgInsn {
    pub addr: Address,
    pub text: String,
}

/// A basic block: contiguous instructions, half-open `[start, end)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicBlock {
    pub index: usize,
    pub start: Address,
    pub end: Address,
    pub insns: Vec<CfgInsn>,
    /// Outgoing edges as (target block index, kind).
    pub edges: Vec<(usize, EdgeKind)>,
}

/// Why control flows from one block to another.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeKind {
    Fallthrough,
    Taken,
    NotTaken,
    Unconditional,
    JumpTable,
}

/// One recognised jump table: targets in table order plus the selector register when it
/// could be traced back to a simple register move.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JumpTable {
    pub targets: Vec<Address>,
    /// Register used as the table index after tracing back (`mov eax, ecx` -> ecx).
    pub selector: Option<RegView>,
}

/// Control-flow graph for one function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cfg {
    pub function: Address,
    pub entry: usize,
    pub blocks: Vec<BasicBlock>,
    /// Branches leaving the function range (tail calls, external jumps).
    pub external: Vec<(Address, Address)>,
    /// Jump tables: dispatch instruction address -> table.
    pub jump_tables: BTreeMap<Address, JumpTable>,
}

struct RawInsn {
    addr: Address,
    next: Address,
    flow: FlowKind,
    target: Option<Address>,
    insn: Insn,
}

/// Build the CFG for `[start, end)`.
pub fn build_cfg(session: &dyn SessionView, start: Address, end: Address) -> Result<Cfg> {
    let mut raw: Vec<RawInsn> = Vec::new();
    let mut addr = start;
    while addr < end && raw.len() < MAX_FUNCTION_INSTRUCTIONS {
        let Some(insn) = session.insn_at(addr)? else {
            break;
        };
        let next = insn.next_addr();
        if next <= addr {
            break;
        }
        raw.push(RawInsn {
            addr,
            next,
            flow: insn.flow(),
            target: insn.branch_target(),
            insn,
        });
        addr = next;
    }
    if raw.is_empty() {
        return Ok(Cfg {
            function: start,
            entry: 0,
            blocks: Vec::new(),
            external: Vec::new(),
            jump_tables: BTreeMap::new(),
        });
    }

    // Jump tables: recognize indirect jumps fed by an indexed table load.
    let image_base = session.image().image_base.get();
    let mut tables: BTreeMap<u64, JumpTable> = BTreeMap::new();
    for index in 0..raw.len() {
        if raw[index].flow != FlowKind::IndirectBranch {
            continue;
        }
        if let Some(table) = detect_jump_table(session, &raw, index, start, end, image_base) {
            tables.insert(raw[index].addr.get(), table);
        }
    }

    // Leaders: function entry, branch targets inside the range, the instruction after
    // a branch, and every jump-table target.
    let mut leaders: BTreeSet<u64> = BTreeSet::new();
    leaders.insert(start.get());
    for insn in &raw {
        if matches!(
            insn.flow,
            FlowKind::ConditionalBranch | FlowKind::UnconditionalBranch
        ) {
            if let Some(target) = insn.target
                && target >= start
                && target < end
            {
                leaders.insert(target.get());
            }
            if insn.next >= start && insn.next < end {
                leaders.insert(insn.next.get());
            }
        }
    }
    for table in tables.values() {
        for target in &table.targets {
            if *target >= start && *target < end {
                leaders.insert(target.get());
            }
        }
    }

    // Split into blocks.
    let mut blocks: Vec<BasicBlock> = Vec::new();
    let mut current: Vec<usize> = Vec::new();
    let flush = |current: &mut Vec<usize>, blocks: &mut Vec<BasicBlock>, raw: &[RawInsn]| {
        if current.is_empty() {
            return;
        }
        let first = current[0];
        let last = *current.last().unwrap();
        blocks.push(BasicBlock {
            index: blocks.len(),
            start: raw[first].addr,
            end: raw[last].next,
            insns: Vec::new(),
            edges: Vec::new(),
        });
        current.clear();
    };

    for index in 0..raw.len() {
        let insn = &raw[index];
        if !current.is_empty() && leaders.contains(&insn.addr.get()) {
            flush(&mut current, &mut blocks, &raw);
        }
        current.push(index);
        let terminates = matches!(
            insn.flow,
            FlowKind::ConditionalBranch
                | FlowKind::UnconditionalBranch
                | FlowKind::IndirectBranch
                | FlowKind::Return
                | FlowKind::Interrupt
                | FlowKind::Trap
                | FlowKind::Invalid
        );
        let next_is_leader = index + 1 < raw.len() && leaders.contains(&raw[index + 1].addr.get());
        if terminates || next_is_leader {
            flush(&mut current, &mut blocks, &raw);
        }
    }
    flush(&mut current, &mut blocks, &raw);

    let mut by_start: BTreeMap<u64, usize> = BTreeMap::new();
    for block in &blocks {
        by_start.insert(block.start.get(), block.index);
    }

    // Fill instruction text and edges.
    let instruction_index: BTreeMap<u64, usize> = raw
        .iter()
        .enumerate()
        .map(|(index, insn)| (insn.addr.get(), index))
        .collect();
    let mut external: Vec<(Address, Address)> = Vec::new();

    let mut new_blocks: Vec<BasicBlock> = Vec::with_capacity(blocks.len());
    let mut formatter = Formatter::new();
    for block in &blocks {
        let (block_start, block_end) = (block.start, block.end);
        let first = *instruction_index.get(&block_start.get()).unwrap_or(&0);
        let mut cursor = first;
        let mut count = 0usize;
        while cursor < raw.len() && raw[cursor].addr < block_end {
            cursor += 1;
            count += 1;
        }
        // Format from the instructions decoded above instead of asking the session for a
        // second linear disassembly: the text is the same, but rendering a function no
        // longer costs three project queries per instruction.
        let mut insns: Vec<CfgInsn> = Vec::with_capacity(count);
        for raw_insn in raw.iter().skip(first).take(count) {
            let insn = &raw_insn.insn;
            let (mnemonic, mut operands) = {
                let (mnemonic, operands) = formatter.format_parts(insn);
                (mnemonic.to_owned(), operands.to_owned())
            };
            let referenced = if insn.memory_is_direct() {
                Some(Address::new(insn.memory_displacement64()))
            } else if matches!(insn.flow(), FlowKind::Call | FlowKind::UnconditionalBranch) {
                insn.branch_target()
            } else {
                None
            };
            if let Some(target) = referenced
                && let Some(target_name) = session.name_at(target)?
            {
                operands.push_str("  ; ");
                operands.push_str(&target_name);
            }
            insns.push(CfgInsn {
                addr: raw_insn.addr,
                text: format!("{mnemonic} {operands}").trim_end().to_owned(),
            });
        }

        let last = raw
            .iter()
            .rev()
            .find(|insn| insn.addr >= block_start && insn.addr < block_end);
        let mut edges: Vec<(usize, EdgeKind)> = Vec::new();
        if let Some(last) = last {
            match last.flow {
                FlowKind::ConditionalBranch => {
                    if let Some(target) = last.target {
                        if let Some(&index) = by_start.get(&target.get()) {
                            edges.push((index, EdgeKind::Taken));
                        } else {
                            external.push((last.addr, target));
                        }
                    }
                    if let Some(&index) = by_start.get(&last.next.get()) {
                        edges.push((index, EdgeKind::NotTaken));
                    }
                }
                FlowKind::UnconditionalBranch => {
                    if let Some(target) = last.target {
                        if let Some(&index) = by_start.get(&target.get()) {
                            edges.push((index, EdgeKind::Unconditional));
                        } else {
                            external.push((last.addr, target));
                        }
                    }
                }
                FlowKind::IndirectBranch => {
                    if let Some(table) = tables.get(&last.addr.get()) {
                        for target in &table.targets {
                            if let Some(&index) = by_start.get(&target.get()) {
                                edges.push((index, EdgeKind::JumpTable));
                            } else {
                                external.push((last.addr, *target));
                            }
                        }
                    }
                }
                FlowKind::Return | FlowKind::Interrupt | FlowKind::Trap | FlowKind::Invalid => {}
                _ => {
                    if let Some(&index) = by_start.get(&last.next.get()) {
                        edges.push((index, EdgeKind::Fallthrough));
                    }
                }
            }
        }
        edges.dedup();
        new_blocks.push(BasicBlock {
            index: block.index,
            start: block_start,
            end: block_end,
            insns,
            edges,
        });
    }
    blocks = new_blocks;

    let entry = blocks
        .iter()
        .position(|block| block.start == start)
        .unwrap_or(0);

    // The builder sweeps the whole `[start, end)` range linearly, so gaps that are not
    // reachable from the entry (alignment padding, jump tables, data between blocks)
    // show up as blocks too. Drop everything unreachable and renumber so the CFG only
    // contains real code (this also keeps metrics/pseudocode from counting `int3`).
    let mut reachable = vec![false; blocks.len()];
    reachable[entry] = true;
    let mut queue = vec![entry];
    while let Some(index) = queue.pop() {
        for (target, _) in &blocks[index].edges {
            if !reachable[*target] {
                reachable[*target] = true;
                queue.push(*target);
            }
        }
    }
    let mut remap = vec![usize::MAX; blocks.len()];
    let mut kept: Vec<BasicBlock> = Vec::new();
    for (old, block) in blocks.iter().enumerate() {
        if reachable[old] {
            remap[old] = kept.len();
            kept.push(block.clone());
        }
    }
    for block in &mut kept {
        block.index = remap[block.index];
        let mut edges = Vec::with_capacity(block.edges.len());
        for (target, kind) in &block.edges {
            let target = remap[*target];
            if target != usize::MAX {
                edges.push((target, *kind));
            }
        }
        block.edges = edges;
    }
    blocks = kept;
    let entry = remap[entry];

    Ok(Cfg {
        function: start,
        entry,
        blocks,
        external,
        jump_tables: tables
            .into_iter()
            .map(|(dispatch, table)| (Address::new(dispatch), table))
            .collect(),
    })
}

/// Trace the table-index register back through simple `mov dst, src` moves so the
/// emitted `switch` can name the original selector (often an argument).
fn trace_selector(raw: &[RawInsn], at: usize, index: Register) -> Option<RegView> {
    let current = from_iced(index)?;
    for earlier in (at.saturating_sub(12)..at).rev() {
        let insn = raw[earlier].insn;
        let destination = insn.op_register(0);
        if from_iced(destination).map(|view| view.base) != Some(current.base) {
            continue;
        }
        let source = insn.op_register(1);
        let source_view = from_iced(source)?;
        if source_view.base != current.base {
            return Some(source_view);
        }
    }
    Some(current)
}

fn detect_jump_table(
    session: &dyn SessionView,
    raw: &[RawInsn],
    jmp_index: usize,
    start: Address,
    end: Address,
    image_base: u64,
) -> Option<JumpTable> {
    let jmp = raw[jmp_index].insn;

    // Pattern A: `jmp qword ptr [index*scale + table]` with absolute pointers.
    if let Some((table, _scale)) = jmp.memory_indexed_table()
        && table != 0
    {
        let entry_size = if session.mode().is_64() { 8 } else { 4 };
        if let Some(targets) = read_table(session, Address::new(table), entry_size, 0, start, end) {
            return Some(JumpTable {
                targets,
                selector: trace_selector(raw, jmp_index, jmp.memory_index()),
            });
        }
    }

    // Pattern B: MSVC x64 image-relative table.
    //   lea base, [image_base]
    //   mov dst32, [base + index*4 + disp]
    //   add dst, base
    //   jmp dst
    let lookback_start = jmp_index.saturating_sub(10);
    for index in (lookback_start..jmp_index).rev() {
        let candidate = raw[index].insn;
        if !candidate.has_memory_operand() {
            continue;
        }
        let base = candidate.memory_base();
        if base == Register::None || base == Register::RIP {
            continue;
        }
        if candidate.memory_index() == Register::None {
            continue;
        }
        let scale = candidate.memory_index_scale();
        if scale != 4 && scale != 8 {
            continue;
        }
        for earlier in (jmp_index.saturating_sub(14)..index).rev() {
            let lea = raw[earlier].insn;
            if !lea.memory_is_direct() || lea.op_register(0) != base {
                continue;
            }
            let base_value = lea.memory_displacement64();
            if base_value != image_base {
                continue;
            }
            let table = base_value.wrapping_add(candidate.memory_displacement64());
            let (entry_size, relative_base) = if scale == 4 {
                (4usize, base_value)
            } else {
                (8usize, 0u64)
            };
            if let Some(targets) = read_table(
                session,
                Address::new(table),
                entry_size,
                relative_base,
                start,
                end,
            ) {
                return Some(JumpTable {
                    targets,
                    selector: trace_selector(raw, index, candidate.memory_index()),
                });
            }
        }
    }
    None
}

/// Read consecutive table entries that stay inside `[start, end)`; at least two valid
/// entries are required so a stray indirect jump is not mistaken for a table.
fn read_table(
    session: &dyn SessionView,
    table: Address,
    entry_size: usize,
    relative_base: u64,
    start: Address,
    end: Address,
) -> Option<Vec<Address>> {
    let mut targets = Vec::new();
    for index in 0..MAX_TABLE_ENTRIES {
        let addr = Address::new(table.get() + (index * entry_size) as u64);
        let mut buffer = [0u8; 8];
        let read = session.read_at(addr, &mut buffer[..entry_size]).ok()?;
        if read < entry_size {
            break;
        }
        let raw_value = u64::from_le_bytes(buffer);
        let target = if relative_base != 0 {
            let value = if entry_size == 4 {
                i64::from(raw_value as u32 as i32)
            } else {
                raw_value as i64
            };
            relative_base.wrapping_add_signed(value)
        } else {
            raw_value
        };
        let target = Address::new(target);
        if target < start || target >= end {
            break;
        }
        targets.push(target);
    }
    (targets.len() >= 2).then_some(targets)
}

/// Assign each block a layer using breadth-first distance from the entry.
pub fn layers(cfg: &Cfg) -> Vec<usize> {
    let mut layer = vec![usize::MAX; cfg.blocks.len()];
    if cfg.blocks.is_empty() {
        return layer;
    }
    layer[cfg.entry] = 0;
    let mut queue = std::collections::VecDeque::from([cfg.entry]);
    while let Some(index) = queue.pop_front() {
        let next_layer = layer[index] + 1;
        for &(target, _) in &cfg.blocks[index].edges {
            if target < layer.len() && layer[target] == usize::MAX {
                layer[target] = next_layer;
                queue.push_back(target);
            }
        }
    }
    // Unreachable blocks go to a final row rather than off-canvas.
    let max_layer = layer
        .iter()
        .filter(|&&value| value != usize::MAX)
        .max()
        .copied()
        .unwrap_or(0);
    for value in &mut layer {
        if *value == usize::MAX {
            *value = max_layer + 1;
        }
    }
    layer
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

    fn open(bytes: &[u8]) -> (Session, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("cfg.rcraft");
        let session = Session::create(&project, bytes, &mut |_, _| {}, &|| false).unwrap();
        (session, dir)
    }

    fn function_bounds(session: &mut Session, name: &str) -> (Address, Address) {
        let analysis = crate::analyze(session).unwrap();
        let image = session.image().clone();
        let export = image
            .exports
            .iter()
            .find(|export| export.name.as_deref() == Some(name))
            .unwrap_or_else(|| panic!("export {name} missing"));
        let start = Address::new(image.image_base.get() + u64::from(export.rva));
        let function = analysis
            .functions
            .iter()
            .find(|function| function.start == start)
            .unwrap_or_else(|| panic!("function {name} not discovered"));
        (function.start, function.end)
    }

    /// `build_cfg` formats block text from the instructions it already decoded. That must
    /// stay byte-identical to what the linear-disassembly view prints for the same block.
    #[test]
    fn block_text_matches_the_disassembly_view() {
        let Some(path) = corpus_path() else {
            return;
        };
        let bytes = std::fs::read(path).unwrap();
        let (mut session, _dir) = open(&bytes);
        let (start, end) = function_bounds(&mut session, "branchy");
        let cfg = build_cfg(&session, start, end).unwrap();
        assert!(!cfg.blocks.is_empty());
        for block in &cfg.blocks {
            let lines = session.disasm(block.start, block.insns.len()).unwrap();
            assert_eq!(lines.len(), block.insns.len());
            for (insn, line) in block.insns.iter().zip(&lines) {
                assert_eq!(insn.addr, line.addr);
                let expected = format!("{} {}", line.mnemonic, line.operands)
                    .trim_end()
                    .to_owned();
                assert_eq!(insn.text, expected);
            }
        }
    }

    #[test]
    fn branchy_has_three_blocks() {
        let Some(path) = corpus_path() else {
            return;
        };
        let bytes = std::fs::read(path).unwrap();
        let (mut session, _dir) = open(&bytes);
        let (start, end) = function_bounds(&mut session, "branchy");
        let cfg = build_cfg(&session, start, end).unwrap();
        assert!(
            cfg.blocks.len() >= 3,
            "expected >= 3 blocks, got {}",
            cfg.blocks.len()
        );
        for block in &cfg.blocks {
            for &(target, _) in &block.edges {
                assert!(target < cfg.blocks.len(), "edge target out of range");
            }
        }
        assert_eq!(cfg.entry, 0);
    }

    #[test]
    fn loopy_has_a_back_edge() {
        let Some(path) = corpus_path() else {
            return;
        };
        let bytes = std::fs::read(path).unwrap();
        let (mut session, _dir) = open(&bytes);
        let (start, end) = function_bounds(&mut session, "loopy");
        let cfg = build_cfg(&session, start, end).unwrap();
        assert!(cfg.blocks.len() >= 3);
        let has_back_edge = cfg
            .blocks
            .iter()
            .any(|block| block.edges.iter().any(|&(target, _)| target <= block.index));
        assert!(has_back_edge, "loop should produce a back edge");
        let layer = layers(&cfg);
        assert!(layer.iter().any(|&value| value > 0));
    }

    #[test]
    fn switcher_big_expands_jump_table() {
        let Some(path) = corpus_path() else {
            return;
        };
        let bytes = std::fs::read(path).unwrap();
        let (mut session, _dir) = open(&bytes);
        let (start, end) = function_bounds(&mut session, "switcher_big");
        let cfg = build_cfg(&session, start, end).unwrap();
        let table_edges: usize = cfg
            .blocks
            .iter()
            .map(|block| {
                block
                    .edges
                    .iter()
                    .filter(|(_, kind)| *kind == EdgeKind::JumpTable)
                    .count()
            })
            .sum();
        assert!(
            table_edges >= 5,
            "expected jump-table edges, got {table_edges} (blocks: {})",
            cfg.blocks.len()
        );
    }

    #[test]
    fn notepad_entry_cfg_is_connected() {
        let Ok(bytes) = std::fs::read("C:/Windows/System32/notepad.exe") else {
            return;
        };
        let (mut session, _dir) = open(&bytes);
        let analysis = crate::analyze(&mut session).unwrap();
        let function = analysis
            .functions
            .iter()
            .find(|function| function.start == session.entry_point())
            .expect("entry function missing");
        let cfg = build_cfg(&session, function.start, function.end).unwrap();
        assert!(!cfg.blocks.is_empty());
        for block in &cfg.blocks {
            assert!(
                !block.insns.is_empty(),
                "block {} at {:#x} has no instructions",
                block.index,
                block.start.get()
            );
        }
    }
}
