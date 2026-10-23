// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Basic-block / control-flow graph construction (M1).
//!
//! Linear decode inside a known function range, split at branch targets and after
//! branch terminators, then connect blocks with typed edges. Deliberately simple:
//! no jump-table expansion yet, indirect branches terminate a block without edges.

use std::collections::{BTreeMap, BTreeSet};

use recraft_common::{Address, Result};
use recraft_core::Session;
use recraft_decode::FlowKind;

const MAX_FUNCTION_INSTRUCTIONS: usize = 20_000;

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
}

/// Control-flow graph for one function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cfg {
    pub function: Address,
    pub entry: usize,
    pub blocks: Vec<BasicBlock>,
    /// Branches leaving the function range (tail calls, external jumps).
    pub external: Vec<(Address, Address)>,
}

struct RawInsn {
    addr: Address,
    next: Address,
    flow: FlowKind,
    target: Option<Address>,
}

/// Build the CFG for `[start, end)`.
pub fn build_cfg(session: &mut Session, start: Address, end: Address) -> Result<Cfg> {
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
        });
        addr = next;
    }
    if raw.is_empty() {
        return Ok(Cfg {
            function: start,
            entry: 0,
            blocks: Vec::new(),
            external: Vec::new(),
        });
    }

    // Leaders: function entry, branch targets inside the range, and the instruction
    // after a branch (fallthrough of a conditional, or the next block).
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
    for block in &blocks {
        let (block_start, block_end) = (block.start, block.end);
        let first = *instruction_index.get(&block_start.get()).unwrap_or(&0);
        let mut cursor = first;
        let mut count = 0usize;
        while cursor < raw.len() && raw[cursor].addr < block_end {
            cursor += 1;
            count += 1;
        }
        let lines = session.disasm(block_start, count)?;
        let insns: Vec<CfgInsn> = lines
            .iter()
            .map(|line| CfgInsn {
                addr: line.addr,
                text: format!("{} {}", line.mnemonic, line.operands)
                    .trim_end()
                    .to_owned(),
            })
            .collect();

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
                FlowKind::IndirectBranch
                | FlowKind::Return
                | FlowKind::Interrupt
                | FlowKind::Trap
                | FlowKind::Invalid => {}
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

    Ok(Cfg {
        function: start,
        entry,
        blocks,
        external,
    })
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

    #[test]
    fn branchy_has_three_blocks() {
        let Some(path) = corpus_path() else {
            return;
        };
        let bytes = std::fs::read(path).unwrap();
        let (mut session, _dir) = open(&bytes);
        let (start, end) = function_bounds(&mut session, "branchy");
        let cfg = build_cfg(&mut session, start, end).unwrap();
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
        let cfg = build_cfg(&mut session, start, end).unwrap();
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
        let cfg = build_cfg(&mut session, function.start, function.end).unwrap();
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
