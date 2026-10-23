// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! L1 structure recovery: basic blocks → `if`/`else` and `while` skeletons.
//!
//! Conservative by design. Dominators and post-dominators drive if/else diamonds and
//! natural loops; anything the structurer cannot prove (switch tables, irreducible
//! flow, indirect jumps) falls back to labelled `goto` output. Unvisited blocks are
//! appended as labelled regions so no code is silently dropped.
//!
//! Known approximation: statements executed by a loop header before its condition are
//! emitted once before the `while` (compilers normally put only a compare there).

use std::collections::{BTreeMap, BTreeSet};

use recraft_analysis::{Cfg, EdgeKind, build_cfg};
use recraft_common::{Address, Error, Result};
use recraft_core::Session;
use recraft_ir::{Flow, Reg};

use crate::l2::{X64_ARG_REGS, parse_prototype};
use crate::{ArgNames, BlockLift, L2Info, PseudoLine, Pseudocode, lift_block};

/// Render structured pseudo-C (L1) for a function produced by automatic analysis.
pub fn pseudocode_l1(session: &mut Session, start: Address) -> Result<Pseudocode> {
    structure(session, start, None)
}

/// Render structured pseudo-C with L2 signature, argument naming and stack slots.
pub fn pseudocode_l2(session: &mut Session, start: Address) -> Result<Pseudocode> {
    let functions = session.functions()?;
    let Some(function) = functions.iter().find(|function| function.start == start) else {
        return pseudocode_l1(session, start);
    };
    let info = crate::l2::recover_l2(session, function.start, function.end)?;
    structure(session, start, Some(info))
}

fn structure(session: &mut Session, start: Address, info: Option<L2Info>) -> Result<Pseudocode> {
    let mut info = info;
    let functions = session.functions()?;
    let Some(function) = functions.iter().find(|function| function.start == start) else {
        let mut fallback = crate::pseudocode_l0(session, start, 512)?;
        fallback.lines.insert(
            0,
            PseudoLine {
                addr: start,
                text: "// L1 unavailable: run analysis first".to_owned(),
            },
        );
        return Ok(fallback);
    };
    let cfg = build_cfg(session, function.start, function.end)?;
    if cfg.blocks.is_empty() {
        return Err(Error::internal(format!(
            "no basic blocks at {:#x}",
            start.get()
        )));
    }

    // A user prototype overrides the inferred signature: return type, calling
    // convention and (on x64) the register arguments with their declared names/types.
    let prototype_text = session.prototype_at(start)?;
    let parsed_proto = prototype_text.as_deref().and_then(parse_prototype);
    if let Some(parsed) = parsed_proto.as_ref() {
        if let Some(info) = info.as_mut() {
            info.ret = parsed.ret;
            if let Some(cc) = parsed.cc {
                info.abi = cc;
            }
            if session.image().is_64 {
                info.args = ArgNames::from_prototype(&parsed.params);
            } else {
                info.args.apply_prototype_x86(&parsed.params);
            }
        }
    }

    let args = info.as_ref().map(|info| &info.args);
    // `apply_type`: struct layouts bound to argument registers for this function.
    let mut arg_structs: BTreeMap<Reg, recraft_core::StructDef> = BTreeMap::new();
    if let Some(args) = args {
        for reg in X64_ARG_REGS {
            if let Some(slot) = args.slot_of(reg)
                && let Some(def) = session.struct_for_arg(start, slot)?
            {
                arg_structs.insert(reg, def);
            }
        }
    }
    let mut bodies = Vec::with_capacity(cfg.blocks.len());
    for block in &cfg.blocks {
        bodies.push(lift_block(
            session,
            block.start,
            block.end,
            args,
            &arg_structs,
            None,
        )?);
    }
    // SSA-lite: which registers are already overwritten on every path reaching a block.
    // Re-lifting with that seed keeps `argN` off clobbered values at join points.
    if args.is_some() {
        let written: Vec<BTreeSet<Reg>> = bodies.iter().map(|body| body.written.clone()).collect();
        let seeds = entry_written(&cfg, &written);
        let mut seeded = Vec::with_capacity(cfg.blocks.len());
        for (index, block) in cfg.blocks.iter().enumerate() {
            seeded.push(lift_block(
                session,
                block.start,
                block.end,
                args,
                &arg_structs,
                Some(&seeds[index]),
            )?);
        }
        bodies = seeded;
    }
    let mut structurer = Structurer::new(&cfg, bodies, info.as_ref().map(|info| info.args.clone()));
    structurer.compute_rpo();
    structurer.compute_dominators();
    structurer.compute_post_dominators();
    structurer.compute_loops();

    let mut name = session
        .name_at(start)?
        .unwrap_or_else(|| format!("sub_{:x}", start.get()));
    if let Some(parsed_name) = parsed_proto.as_ref().and_then(|parsed| parsed.name.clone()) {
        name = parsed_name;
    }
    let mut lines = Vec::with_capacity(64);
    lines.push(PseudoLine {
        addr: start,
        text: format!("// L1 structure recovery ({} blocks)", cfg.blocks.len()),
    });
    match &info {
        Some(info) => {
            if !info.args.is_empty() {
                lines.push(PseudoLine {
                    addr: start,
                    text: format!("// L2 params: {}", info.args.comment()),
                });
            }
            lines.push(PseudoLine {
                addr: start,
                text: format!(
                    "{} {}{}({}) {{",
                    info.ret.c_name(),
                    info.abi.keyword(),
                    name,
                    info.args.signature()
                ),
            });
        }
        None => {
            lines.push(PseudoLine {
                addr: start,
                text: format!("void {name}_l1(void) {{"),
            });
        }
    }
    structurer.emit_all(&mut lines);
    lines.push(PseudoLine {
        addr: start,
        text: "}".to_owned(),
    });
    // A user-set prototype is authoritative for naming/definitions; show it verbatim
    // above the generated signature (which already reflects its parsed types/names).
    if let Some(prototype) = session.prototype_at(start)? {
        lines.insert(
            0,
            PseudoLine {
                addr: start,
                text: format!("// prototype: {prototype}"),
            },
        );
    }
    Ok(Pseudocode {
        func_start: start,
        lines,
    })
}

/// Which registers may already have been overwritten whenever a block is entered.
///
/// Forward may-write dataflow: `in[b] = ∪ out[pred]`, `out[b] = in[b] ∪ written[b]`.
/// The function entry starts empty (incoming argument registers are intact there).
fn entry_written(cfg: &Cfg, written: &[BTreeSet<Reg>]) -> Vec<BTreeSet<Reg>> {
    let count = written.len();
    let mut predecessors = vec![Vec::new(); count];
    for (index, block) in cfg.blocks.iter().enumerate() {
        for (target, _) in &block.edges {
            if *target < count {
                predecessors[*target].push(index);
            }
        }
    }
    let mut entering: Vec<BTreeSet<Reg>> = vec![BTreeSet::new(); count];
    let mut leaving = written.to_vec();
    for _ in 0..(count + 4) {
        let mut changed = false;
        for block in 0..count {
            let mut set = BTreeSet::new();
            if block != cfg.entry {
                for &pred in &predecessors[block] {
                    set.extend(leaving[pred].iter().copied());
                }
            }
            if set != entering[block] {
                entering[block] = set;
                changed = true;
            }
            let mut out = entering[block].clone();
            out.extend(written[block].iter().copied());
            if out != leaving[block] {
                leaving[block] = out;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    entering
}

struct Block {
    entry: Address,
    flow: Flow,
    condition: Option<String>,
    taken: Option<usize>,
    not_taken: Option<usize>,
    goto_target: Option<usize>,
    has_table: bool,
    /// Jump-table targets in table (case) order.
    table_targets: Vec<usize>,
    /// Traced selector register of the jump table, if any.
    table_selector: Option<recraft_ir::RegView>,
}

struct Structurer {
    blocks: Vec<Block>,
    entry: usize,
    successors: Vec<Vec<usize>>,
    predecessors: Vec<Vec<usize>>,
    rpo: Vec<usize>,
    idom: Vec<Option<usize>>,
    ipdom: Vec<Option<usize>>,
    loop_headers: BTreeSet<usize>,
    loop_of: Vec<Option<usize>>,
    loop_blocks: BTreeMap<usize, BTreeSet<usize>>,
    /// Active loops while emitting bodies: `(header, exit)` for break/continue.
    loop_stack: Vec<(usize, usize)>,
    /// Loop headers currently emitted as an unstructured `while (1)` body (avoids
    /// recursing back into `emit_loop` for the same header).
    fallback_headers: BTreeSet<usize>,
    emitted: Vec<bool>,
    bodies: Vec<BlockLift>,
    args: Option<ArgNames>,
}

impl Structurer {
    fn new(cfg: &Cfg, bodies: Vec<BlockLift>, args: Option<ArgNames>) -> Self {
        let index_of =
            |address: Address| cfg.blocks.iter().position(|block| block.start == address);
        let mut blocks = Vec::with_capacity(cfg.blocks.len());
        let mut successors = vec![Vec::new(); cfg.blocks.len()];
        for (index, block) in cfg.blocks.iter().enumerate() {
            let mut taken = None;
            let mut not_taken = None;
            let mut goto_target = None;
            let mut has_table = false;
            for &(target, kind) in &block.edges {
                match kind {
                    EdgeKind::Taken => taken = Some(target),
                    EdgeKind::NotTaken | EdgeKind::Fallthrough => not_taken = Some(target),
                    EdgeKind::Unconditional => goto_target = Some(target),
                    EdgeKind::JumpTable => has_table = true,
                }
                if !successors[index].contains(&target) {
                    successors[index].push(target);
                }
            }
            let lifted = &bodies[index];
            // Prefer the CFG's own table (ordered targets + traced selector); the
            // deduplicated JumpTable edges are only a fallback.
            let dispatch = cfg
                .jump_tables
                .iter()
                .find(|(addr, _)| **addr >= block.start && **addr < block.end);
            let (table_targets, table_selector) = if let Some((_, table)) = dispatch {
                (
                    table
                        .targets
                        .iter()
                        .filter_map(|target| index_of(*target))
                        .collect(),
                    table.selector,
                )
            } else {
                (
                    block
                        .edges
                        .iter()
                        .filter(|(_, kind)| *kind == EdgeKind::JumpTable)
                        .map(|(target, _)| *target)
                        .collect(),
                    None,
                )
            };
            blocks.push(Block {
                entry: block.start,
                flow: lifted.flow,
                condition: lifted.condition.clone(),
                taken,
                not_taken,
                goto_target: match lifted.flow {
                    Flow::Goto(address) => index_of(address),
                    _ => goto_target,
                },
                has_table,
                table_targets,
                table_selector,
            });
        }
        let mut predecessors = vec![Vec::new(); cfg.blocks.len()];
        for (index, nexts) in successors.iter().enumerate() {
            for &next in nexts {
                predecessors[next].push(index);
            }
        }
        let entry = cfg.entry;
        Structurer {
            blocks,
            entry,
            successors,
            predecessors,
            rpo: Vec::new(),
            idom: vec![None; bodies.len()],
            ipdom: vec![None; bodies.len()],
            loop_headers: BTreeSet::new(),
            loop_of: vec![None; bodies.len()],
            loop_blocks: BTreeMap::new(),
            loop_stack: Vec::new(),
            fallback_headers: BTreeSet::new(),
            emitted: vec![false; bodies.len()],
            bodies,
            args,
        }
    }

    // ------------------------------------------------------------ graph math --

    fn compute_rpo(&mut self) {
        let count = self.blocks.len();
        let mut visited = vec![false; count];
        let mut postorder = Vec::with_capacity(count);
        let mut stack = vec![(self.entry, 0usize)];
        visited[self.entry] = true;
        while let Some((node, next_index)) = stack.pop() {
            if next_index < self.successors[node].len() {
                stack.push((node, next_index + 1));
                let next = self.successors[node][next_index];
                if !visited[next] {
                    visited[next] = true;
                    stack.push((next, 0));
                }
            } else {
                postorder.push(node);
            }
        }
        self.rpo = postorder.into_iter().rev().collect();
    }

    fn compute_dominators(&mut self) {
        let count = self.blocks.len();
        let mut idom: Vec<Option<usize>> = vec![None; count];
        idom[self.entry] = Some(self.entry);
        let order: BTreeMap<usize, usize> = self
            .rpo
            .iter()
            .enumerate()
            .map(|(index, node)| (*node, index))
            .collect();
        let mut changed = true;
        while changed {
            changed = false;
            for &node in self.rpo.iter().skip(1) {
                let mut new_idom = None;
                for &pred in &self.predecessors[node] {
                    if idom[pred].is_some() {
                        new_idom = Some(match new_idom {
                            None => pred,
                            Some(current) => intersect(&order, &idom, current, pred),
                        });
                    }
                }
                if let Some(new_idom) = new_idom
                    && idom[node] != Some(new_idom)
                {
                    idom[node] = Some(new_idom);
                    changed = true;
                }
            }
        }
        self.idom = idom;
    }

    /// Post-dominators with a virtual exit node; `None` means "can reach function end
    /// without reconverging", which disables if/else packaging for that header.
    fn compute_post_dominators(&mut self) {
        let count = self.blocks.len();
        let virtual_exit = count;
        let mut predecessors = vec![Vec::new(); count + 1];
        for node in 0..count {
            if self.successors[node].is_empty() {
                predecessors[virtual_exit].push(node);
            }
            for &next in &self.successors[node] {
                predecessors[next].push(node);
            }
        }
        let mut visited = vec![false; count + 1];
        let mut postorder = Vec::new();
        let mut stack = vec![(virtual_exit, 0usize)];
        visited[virtual_exit] = true;
        while let Some((node, next_index)) = stack.pop() {
            if next_index < predecessors[node].len() {
                stack.push((node, next_index + 1));
                let next = predecessors[node][next_index];
                if !visited[next] {
                    visited[next] = true;
                    stack.push((next, 0));
                }
            } else {
                postorder.push(node);
            }
        }
        let rpo: Vec<usize> = postorder.into_iter().rev().collect();
        let order: BTreeMap<usize, usize> = rpo
            .iter()
            .enumerate()
            .map(|(index, node)| (*node, index))
            .collect();
        let mut ipdom: Vec<Option<usize>> = vec![None; count + 1];
        ipdom[virtual_exit] = Some(virtual_exit);
        // Exit blocks post-dominate directly from the virtual exit.
        for (node, successors) in self.successors.iter().enumerate() {
            if successors.is_empty() {
                ipdom[node] = Some(virtual_exit);
            }
        }
        let mut changed = true;
        while changed {
            changed = false;
            for &node in rpo.iter().skip(1) {
                if node == virtual_exit {
                    continue;
                }
                let mut new_ipdom = None;
                for &next in &self.successors[node] {
                    if ipdom[next].is_some() {
                        new_ipdom = Some(match new_ipdom {
                            None => next,
                            Some(current) => intersect(&order, &ipdom, current, next),
                        });
                    }
                }
                if let Some(new_ipdom) = new_ipdom
                    && ipdom[node] != Some(new_ipdom)
                {
                    ipdom[node] = Some(new_ipdom);
                    changed = true;
                }
            }
        }
        self.ipdom = (0..count)
            .map(|node| ipdom[node].filter(|&value| value != virtual_exit))
            .collect();
    }

    fn compute_loops(&mut self) {
        let count = self.blocks.len();
        let mut loops: Vec<(usize, BTreeSet<usize>)> = Vec::new();
        for node in 0..count {
            for &next in &self.successors[node] {
                if self.dominates(next, node) {
                    let mut body = BTreeSet::new();
                    body.insert(next);
                    // Natural loop: the back-edge source and everything reaching it
                    // without passing through the header (a self-loop is just {header}).
                    if node != next {
                        let mut work = vec![node];
                        while let Some(block) = work.pop() {
                            if body.insert(block) {
                                for &pred in &self.predecessors[block] {
                                    if !body.contains(&pred) {
                                        work.push(pred);
                                    }
                                }
                            }
                        }
                    }
                    loops.push((next, body));
                }
            }
        }
        // Innermost loop wins for shared blocks (sort by size, smallest first).
        loops.sort_by_key(|(_, body)| body.len());
        for (header, body) in loops {
            self.loop_headers.insert(header);
            self.loop_of[header] = Some(header);
            for &block in &body {
                if block != header && self.loop_of[block].is_none() {
                    self.loop_of[block] = Some(header);
                }
            }
            self.loop_blocks.entry(header).or_insert(body);
        }
    }

    fn dominates(&self, candidate: usize, node: usize) -> bool {
        let mut current = node;
        for _ in 0..=self.blocks.len() {
            if current == candidate {
                return true;
            }
            match self.idom[current] {
                Some(parent) if parent != current => current = parent,
                _ => return false,
            }
        }
        false
    }

    // -------------------------------------------------------------- emission --

    fn emit_all(&mut self, lines: &mut Vec<PseudoLine>) {
        self.emit_linear(lines, self.entry, 1, &BTreeSet::new());
        for index in 0..self.blocks.len() {
            if self.emitted[index] {
                continue;
            }
            self.emitted[index] = true;
            lines.push(PseudoLine {
                addr: self.blocks[index].entry,
                text: format!("{}loc_{:x}:", indent(1), self.blocks[index].entry.get()),
            });
            self.emit_simple(lines, index, 1);
            self.emit_raw_flow(lines, index, 1);
        }
    }

    fn emit_linear(
        &mut self,
        lines: &mut Vec<PseudoLine>,
        start: usize,
        level: usize,
        stop: &BTreeSet<usize>,
    ) {
        let mut current = start;
        let mut guard = 0usize;
        while guard <= self.blocks.len() + 8 {
            guard += 1;
            if current >= self.blocks.len() || self.emitted[current] || stop.contains(&current) {
                return;
            }
            if let Some(header) = self.loop_of[current]
                && header != current
                && !self.emitted[header]
            {
                // Entered a loop body without its header first: keep output safe.
                self.emitted[current] = true;
                self.emit_simple(lines, current, level);
                self.emit_raw_flow(lines, current, level);
                return;
            }
            if self.loop_headers.contains(&current) && !self.fallback_headers.contains(&current) {
                match self.emit_loop(lines, current, level) {
                    Some(exit) => {
                        current = exit;
                        continue;
                    }
                    None => {
                        self.emitted[current] = true;
                        self.emit_simple(lines, current, level);
                        self.emit_raw_flow(lines, current, level);
                        return;
                    }
                }
            }
            self.emitted[current] = true;
            self.emit_simple(lines, current, level);
            if self.blocks[current].has_table {
                self.emit_switch(lines, current, level);
                return;
            }

            match self.blocks[current].flow {
                Flow::Return => {
                    lines.push(PseudoLine {
                        addr: self.blocks[current].entry,
                        text: format!("{}return;", indent(level)),
                    });
                    return;
                }
                Flow::Goto(_) => {
                    let Some(target) = self.blocks[current].goto_target else {
                        self.emit_raw_flow(lines, current, level);
                        return;
                    };
                    if let Some(&(loop_header, loop_exit)) = self.loop_stack.last() {
                        if target == loop_exit {
                            lines.push(PseudoLine {
                                addr: self.blocks[current].entry,
                                text: format!("{}break;", indent(level)),
                            });
                            return;
                        }
                        if target == loop_header {
                            lines.push(PseudoLine {
                                addr: self.blocks[current].entry,
                                text: format!("{}continue;", indent(level)),
                            });
                            return;
                        }
                    }
                    if target == current {
                        self.emit_raw_flow(lines, current, level);
                        return;
                    }
                    if stop.contains(&target) {
                        // Leaving the current arm/region: the enclosing construct resumes
                        // at the join, so no explicit jump is needed. Inside a loop body
                        // only targets that stay in the body may fall out; anything else
                        // that is not break/continue must stay an explicit jump.
                        if let Some(&(loop_header, _)) = self.loop_stack.last() {
                            let inside_body = self
                                .loop_blocks
                                .get(&loop_header)
                                .is_some_and(|body| body.contains(&target));
                            if !inside_body {
                                self.emit_raw_flow(lines, current, level);
                            }
                            return;
                        }
                        return;
                    }
                    if self.emitted[target] {
                        if self.duplicate_leaf(lines, target, level) {
                            return;
                        }
                        self.emit_raw_flow(lines, current, level);
                        return;
                    }
                    current = target;
                }
                Flow::CondGoto { .. } => {
                    let (Some(condition), Some(taken), Some(not_taken)) = (
                        self.blocks[current].condition.clone(),
                        self.blocks[current].taken,
                        self.blocks[current].not_taken,
                    ) else {
                        self.emit_raw_flow(lines, current, level);
                        return;
                    };
                    if taken == not_taken {
                        current = taken;
                        continue;
                    }
                    // Inside a loop body: a branch to the loop exit is `break`, a branch
                    // back to the header is `continue`.
                    if let Some(&(loop_header, loop_exit)) = self.loop_stack.last() {
                        if taken == loop_exit {
                            lines.push(PseudoLine {
                                addr: self.blocks[current].entry,
                                text: format!("{}if ({condition}) break;", indent(level)),
                            });
                            current = not_taken;
                            continue;
                        }
                        if not_taken == loop_exit {
                            lines.push(PseudoLine {
                                addr: self.blocks[current].entry,
                                text: format!(
                                    "{}if ({}) break;",
                                    indent(level),
                                    negate(&condition)
                                ),
                            });
                            current = taken;
                            continue;
                        }
                        if taken == loop_header {
                            lines.push(PseudoLine {
                                addr: self.blocks[current].entry,
                                text: format!("{}if ({condition}) continue;", indent(level)),
                            });
                            current = not_taken;
                            continue;
                        }
                        if not_taken == loop_header {
                            lines.push(PseudoLine {
                                addr: self.blocks[current].entry,
                                text: format!(
                                    "{}if ({}) continue;",
                                    indent(level),
                                    negate(&condition)
                                ),
                            });
                            current = taken;
                            continue;
                        }
                    }
                    let join = self.ipdom[current];
                    if let Some(join) = join {
                        // A join inside the enclosing region's stop set is fine: the arms
                        // stop at it and the region ends there (early-exit chains).
                        if join >= self.blocks.len() || self.emitted[join] {
                            self.emit_raw_flow(lines, current, level);
                            return;
                        }
                        let mut arm_stop = stop.clone();
                        arm_stop.insert(join);
                        // Emit both arms into buffers so empty arms do not produce empty
                        // `if` blocks (common for early-exit joins).
                        let mut then_lines = Vec::new();
                        if !self.emitted[taken] {
                            self.emit_linear(&mut then_lines, taken, level + 1, &arm_stop);
                        } else {
                            self.duplicate_leaf(&mut then_lines, taken, level + 1);
                        }
                        let mut else_lines = Vec::new();
                        if !self.emitted[not_taken] {
                            self.emit_linear(&mut else_lines, not_taken, level + 1, &arm_stop);
                        } else {
                            self.duplicate_leaf(&mut else_lines, not_taken, level + 1);
                        }
                        let entry = self.blocks[current].entry;
                        match (then_lines.is_empty(), else_lines.is_empty()) {
                            (true, true) => {}
                            (true, false) => {
                                lines.push(PseudoLine {
                                    addr: entry,
                                    text: format!("{}if (!{condition}) {{", indent(level)),
                                });
                                lines.extend(else_lines);
                                lines.push(PseudoLine {
                                    addr: entry,
                                    text: format!("{}}}", indent(level)),
                                });
                            }
                            (false, true) => {
                                lines.push(PseudoLine {
                                    addr: entry,
                                    text: format!("{}if ({condition}) {{", indent(level)),
                                });
                                lines.extend(then_lines);
                                lines.push(PseudoLine {
                                    addr: entry,
                                    text: format!("{}}}", indent(level)),
                                });
                            }
                            (false, false) => {
                                lines.push(PseudoLine {
                                    addr: entry,
                                    text: format!("{}if ({condition}) {{", indent(level)),
                                });
                                lines.extend(then_lines);
                                lines.push(PseudoLine {
                                    addr: entry,
                                    text: format!("{}}} else {{", indent(level)),
                                });
                                lines.extend(else_lines);
                                lines.push(PseudoLine {
                                    addr: entry,
                                    text: format!("{}}}", indent(level)),
                                });
                            }
                        }
                        current = join;
                        continue;
                    }
                    // No reconvergence (both arms terminate): emit the taken arm as an
                    // early-exit `if` and continue with the fallthrough path.
                    if !self.emitted[taken] && taken != not_taken {
                        lines.push(PseudoLine {
                            addr: self.blocks[current].entry,
                            text: format!("{}if ({condition}) {{", indent(level)),
                        });
                        self.emit_linear(lines, taken, level + 1, stop);
                        lines.push(PseudoLine {
                            addr: self.blocks[current].entry,
                            text: format!("{}}}", indent(level)),
                        });
                        current = not_taken;
                        continue;
                    }
                    if self.emitted[taken] && taken != not_taken {
                        let mut arm_lines = Vec::new();
                        if self.duplicate_leaf(&mut arm_lines, taken, level + 1) {
                            lines.push(PseudoLine {
                                addr: self.blocks[current].entry,
                                text: format!("{}if ({condition}) {{", indent(level)),
                            });
                            lines.extend(arm_lines);
                            lines.push(PseudoLine {
                                addr: self.blocks[current].entry,
                                text: format!("{}}}", indent(level)),
                            });
                            current = not_taken;
                            continue;
                        }
                    }
                    self.emit_raw_flow(lines, current, level);
                    return;
                }
                _ => {
                    let Some(next) = self.blocks[current].not_taken else {
                        return;
                    };
                    if next == current {
                        return;
                    }
                    current = next;
                }
            }
        }
    }

    fn emit_loop(
        &mut self,
        lines: &mut Vec<PseudoLine>,
        header: usize,
        level: usize,
    ) -> Option<usize> {
        let body = self.loop_blocks.get(&header)?.clone();
        if self.blocks[header].condition.is_none() {
            // No condition at the header: try a do-while whose latch tests the condition
            // at the bottom of the body.
            return self.emit_do_while(lines, header, level, &body);
        }
        let condition = self.blocks[header].condition.clone()?;
        let taken = self.blocks[header].taken?;
        let not_taken = self.blocks[header].not_taken?;
        // One successor stays in the loop, the other leaves it; otherwise this is not a
        // structured pre-test loop and the caller falls back to gotos.
        let (inside, exit, condition) = if body.contains(&taken) && !body.contains(&not_taken) {
            (taken, not_taken, condition)
        } else if body.contains(&not_taken) && !body.contains(&taken) {
            (not_taken, taken, negate(&condition))
        } else {
            return self.emit_unstructured_loop(lines, header, level, &body);
        };

        // A single-block loop whose header is also the latch is a do-while: emit the
        // statements then test the condition at the bottom.
        if body.len() == 1 {
            self.loop_stack.push((header, exit));
            lines.push(PseudoLine {
                addr: self.blocks[header].entry,
                text: format!("{}do {{", indent(level)),
            });
            self.emitted[header] = true;
            self.emit_simple(lines, header, level + 1);
            lines.push(PseudoLine {
                addr: self.blocks[header].entry,
                text: format!("{}}} while ({condition});", indent(level)),
            });
            self.loop_stack.pop();
            return Some(exit);
        }

        // Header statements run once per iteration: keep them inside the loop and
        // break when the condition says to leave.
        self.loop_stack.push((header, exit));
        lines.push(PseudoLine {
            addr: self.blocks[header].entry,
            text: format!("{}while (1) {{", indent(level)),
        });
        self.emitted[header] = true;
        self.emit_simple(lines, header, level + 1);
        let break_condition = if exit == taken {
            condition.clone()
        } else {
            negate(&condition)
        };
        lines.push(PseudoLine {
            addr: self.blocks[header].entry,
            text: format!("{}if ({break_condition}) break;", indent(level + 1)),
        });

        let mut stop = BTreeSet::new();
        for block in 0..self.blocks.len() {
            if !body.contains(&block) {
                stop.insert(block);
            }
        }
        if inside != header {
            self.emit_linear(lines, inside, level + 1, &stop);
        }
        lines.push(PseudoLine {
            addr: self.blocks[header].entry,
            text: format!("{}}}", indent(level)),
        });
        for block in &body {
            if !self.emitted[*block] {
                self.emitted[*block] = true;
                lines.push(PseudoLine {
                    addr: self.blocks[*block].entry,
                    text: format!(
                        "{}loc_{:x}:",
                        indent(level),
                        self.blocks[*block].entry.get()
                    ),
                });
                self.emit_simple(lines, *block, level);
                self.emit_raw_flow(lines, *block, level);
            }
        }
        self.loop_stack.pop();
        Some(exit)
    }

    /// The header condition does not split into "stay in loop" vs "leave loop" (e.g. a
    /// short-circuit guard at the top of the body). Emit `while (1) { <header structured>
    /// ... }` and let break/continue carry the exits.
    fn emit_unstructured_loop(
        &mut self,
        lines: &mut Vec<PseudoLine>,
        header: usize,
        level: usize,
        body: &BTreeSet<usize>,
    ) -> Option<usize> {
        if self.emitted[header] || self.fallback_headers.contains(&header) {
            return None;
        }
        let mut exit = None;
        for &block in body {
            for &next in &self.successors[block] {
                if !body.contains(&next) {
                    exit = Some(next);
                    break;
                }
            }
            if exit.is_some() {
                break;
            }
        }
        let exit = exit?;
        let mut stop = BTreeSet::new();
        for block in 0..self.blocks.len() {
            if !body.contains(&block) {
                stop.insert(block);
            }
        }
        lines.push(PseudoLine {
            addr: self.blocks[header].entry,
            text: format!("{}while (1) {{", indent(level)),
        });
        self.loop_stack.push((header, exit));
        self.fallback_headers.insert(header);
        self.emit_linear(lines, header, level + 1, &stop);
        self.fallback_headers.remove(&header);
        self.loop_stack.pop();
        lines.push(PseudoLine {
            addr: self.blocks[header].entry,
            text: format!("{}}}", indent(level)),
        });
        for block in body {
            if !self.emitted[*block] {
                self.emitted[*block] = true;
                lines.push(PseudoLine {
                    addr: self.blocks[*block].entry,
                    text: format!(
                        "{}loc_{:x}:",
                        indent(level + 1),
                        self.blocks[*block].entry.get()
                    ),
                });
                self.emit_simple(lines, *block, level + 1);
                self.emit_raw_flow(lines, *block, level + 1);
            }
        }
        Some(exit)
    }

    /// Emit `do { ... } while (cond);` when the header has no condition and a latch block
    /// inside the body branches back to the header (or leaves the loop).
    fn emit_do_while(
        &mut self,
        lines: &mut Vec<PseudoLine>,
        header: usize,
        level: usize,
        body: &BTreeSet<usize>,
    ) -> Option<usize> {
        let mut latch = None;
        for &block in body {
            if block == header {
                continue;
            }
            let Some(condition) = self.blocks[block].condition.clone() else {
                continue;
            };
            let (Some(taken), Some(not_taken)) =
                (self.blocks[block].taken, self.blocks[block].not_taken)
            else {
                continue;
            };
            let (exit, continue_condition) = if taken == header && !body.contains(&not_taken) {
                (not_taken, condition)
            } else if not_taken == header && !body.contains(&taken) {
                (taken, negate(&condition))
            } else {
                continue;
            };
            latch = Some((block, exit, continue_condition));
            break;
        }
        let (latch, exit, continue_condition) = latch?;

        let mut stop = BTreeSet::new();
        for block in 0..self.blocks.len() {
            if !body.contains(&block) {
                stop.insert(block);
            }
        }
        let mut body_stop = stop.clone();
        body_stop.insert(latch);

        lines.push(PseudoLine {
            addr: self.blocks[header].entry,
            text: format!("{}do {{", indent(level)),
        });
        self.emitted[header] = true;
        self.loop_stack.push((header, exit));
        self.emit_simple(lines, header, level + 1);
        // Follow the header's flow into the body (stop before the latch).
        if let Some(next) = self.blocks[header].not_taken
            && next != latch
        {
            self.emit_linear(lines, next, level + 1, &body_stop);
        } else if let Some(target) = self.blocks[header].goto_target
            && target != latch
        {
            self.emit_linear(lines, target, level + 1, &body_stop);
        }
        if !self.emitted[latch] {
            self.emitted[latch] = true;
            self.emit_simple(lines, latch, level + 1);
        }
        lines.push(PseudoLine {
            addr: self.blocks[latch].entry,
            text: format!("{}}} while ({continue_condition});", indent(level)),
        });
        // Any body blocks that were not reached linearly stay as labelled leftovers.
        for block in body {
            if !self.emitted[*block] {
                self.emitted[*block] = true;
                lines.push(PseudoLine {
                    addr: self.blocks[*block].entry,
                    text: format!(
                        "{}loc_{:x}:",
                        indent(level + 1),
                        self.blocks[*block].entry.get()
                    ),
                });
                self.emit_simple(lines, *block, level + 1);
                self.emit_raw_flow(lines, *block, level + 1);
            }
        }
        self.loop_stack.pop();
        Some(exit)
    }

    fn emit_switch(&mut self, lines: &mut Vec<PseudoLine>, block: usize, level: usize) {
        let targets = self.blocks[block].table_targets.clone();
        if targets.is_empty() {
            self.emit_raw_flow(lines, block, level);
            return;
        }
        let entry = self.blocks[block].entry;
        // Prefer a traced selector: map argument registers back to `argN` when intact.
        let selector = self.blocks[block]
            .table_selector
            .map(|view| {
                if let Some(args) = self.args.as_ref()
                    && let Some(name) = args.name_for(view.base)
                {
                    return name.to_owned();
                }
                view.name().to_owned()
            })
            .unwrap_or_else(|| "index".to_owned());
        lines.push(PseudoLine {
            addr: entry,
            text: format!(
                "{}/* jump table: {} targets, selector={selector} */",
                indent(level),
                targets.len()
            ),
        });
        lines.push(PseudoLine {
            addr: entry,
            text: format!("{}switch ({selector}) {{", indent(level)),
        });
        let join = self.ipdom[block];
        if self.switch_inline_safe(&targets, join) {
            let mut previous = None;
            for (case, &target) in targets.iter().enumerate() {
                if previous == Some(target) {
                    lines.push(PseudoLine {
                        addr: entry,
                        text: format!("{}case {case}:", indent(level + 1)),
                    });
                    continue;
                }
                previous = Some(target);
                lines.push(PseudoLine {
                    addr: self.blocks[target].entry,
                    text: format!("{}case {case}: {{", indent(level + 1)),
                });
                self.emitted[target] = true;
                self.emit_simple(lines, target, level + 2);
                match &self.blocks[target].flow {
                    Flow::Return => lines.push(PseudoLine {
                        addr: self.blocks[target].entry,
                        text: format!("{}return;", indent(level + 2)),
                    }),
                    Flow::Goto(_) => lines.push(PseudoLine {
                        addr: self.blocks[target].entry,
                        text: format!("{}break;", indent(level + 2)),
                    }),
                    _ => {}
                }
                lines.push(PseudoLine {
                    addr: self.blocks[target].entry,
                    text: format!("{}}}", indent(level + 1)),
                });
            }
            lines.push(PseudoLine {
                addr: entry,
                text: format!("{}}}", indent(level)),
            });
            return;
        }
        for (case, target) in targets.iter().enumerate() {
            lines.push(PseudoLine {
                addr: self.blocks[*target].entry,
                text: format!(
                    "{}case {case}: goto loc_{:x};",
                    indent(level + 1),
                    self.blocks[*target].entry.get()
                ),
            });
        }
        lines.push(PseudoLine {
            addr: entry,
            text: format!("{}}}", indent(level)),
        });
        // Case bodies are emitted as labelled regions by the unvisited-block pass.
    }

    /// Inline a jump table only when every target is a distinct, already-unemitted leaf
    /// that returns or jumps to the switch join; otherwise keep the label/goto form.
    fn switch_inline_safe(&self, targets: &[usize], join: Option<usize>) -> bool {
        let mut seen = BTreeSet::new();
        let mut previous = None;
        for &target in targets {
            if let Some(previous) = previous {
                if previous == target {
                    continue;
                }
                if seen.contains(&target) {
                    return false; // non-adjacent duplicate case target
                }
            }
            previous = Some(target);
            if !seen.insert(target) {
                continue;
            }
            if self.emitted[target] {
                return false;
            }
            match &self.blocks[target].flow {
                Flow::Return => {}
                Flow::Goto(goto) if join.is_some_and(|index| self.blocks[index].entry == *goto) => {
                }
                _ => return false,
            }
        }
        true
    }

    /// Re-emit an already-emitted straight-line chain that ends in `return`/tail call
    /// instead of emitting a `goto` to it. The chain is duplicated only inside the one
    /// branch that reaches it, so extra copies never both execute; this removes most
    /// shared-epilogue and shared-tail `goto`s.
    fn duplicate_leaf(&mut self, lines: &mut Vec<PseudoLine>, block: usize, level: usize) -> bool {
        let mut chain: Vec<usize> = Vec::new();
        let mut seen: BTreeSet<usize> = BTreeSet::new();
        let mut current = block;
        let mut budget = 24usize;
        while budget > 0 {
            if current >= self.blocks.len() || !seen.insert(current) {
                return false;
            }
            if self.blocks[current].has_table || self.loop_headers.contains(&current) {
                return false;
            }
            chain.push(current);
            budget -= 1;
            match self.blocks[current].flow {
                Flow::Return => break,
                Flow::Goto(_) if self.blocks[current].goto_target.is_none() => break,
                Flow::Goto(_) => {
                    let Some(target) = self.blocks[current].goto_target else {
                        return false;
                    };
                    current = target;
                }
                Flow::Fallthrough => {
                    let Some(next) = self.blocks[current].not_taken else {
                        return false;
                    };
                    if next == current {
                        return false;
                    }
                    current = next;
                }
                Flow::CondGoto { .. } => return false,
            }
        }
        if budget == 0 {
            return false;
        }
        let Some(&last) = chain.last() else {
            return false;
        };
        let ends_leaf = matches!(self.blocks[last].flow, Flow::Return)
            || (self.blocks[last].goto_target.is_none()
                && matches!(self.blocks[last].flow, Flow::Goto(_)));
        if !ends_leaf {
            return false;
        }
        let statements: usize = chain
            .iter()
            .map(|index| self.bodies[*index].statements.len())
            .sum();
        if statements > 24 {
            return false;
        }
        for &index in &chain {
            self.emit_simple(lines, index, level);
        }
        if matches!(self.blocks[last].flow, Flow::Return) {
            lines.push(PseudoLine {
                addr: self.blocks[last].entry,
                text: format!("{}return;", indent(level)),
            });
        } else {
            self.emit_raw_flow(lines, last, level);
        }
        true
    }

    fn emit_simple(&self, lines: &mut Vec<PseudoLine>, block: usize, level: usize) {
        for statement in &self.bodies[block].statements {
            lines.push(PseudoLine {
                addr: statement.addr,
                text: format!("{}{}", indent(level), statement.text),
            });
        }
    }

    fn emit_raw_flow(&mut self, lines: &mut Vec<PseudoLine>, block: usize, level: usize) {
        let entry = self.blocks[block].entry;
        match &self.blocks[block].flow {
            Flow::Return => lines.push(PseudoLine {
                addr: entry,
                text: format!("{}return;", indent(level)),
            }),
            Flow::Goto(target) => {
                if self.blocks[block].goto_target.is_some() {
                    lines.push(PseudoLine {
                        addr: entry,
                        text: format!("{}goto loc_{:x};", indent(level), target.get()),
                    });
                } else {
                    lines.push(PseudoLine {
                        addr: entry,
                        text: format!(
                            "{}/* tail call -> {:#x} */ return;",
                            indent(level),
                            target.get()
                        ),
                    });
                }
            }
            Flow::CondGoto { target, .. } => {
                let condition = self.blocks[block]
                    .condition
                    .clone()
                    .unwrap_or_else(|| "/* cond */".to_owned());
                lines.push(PseudoLine {
                    addr: entry,
                    text: format!(
                        "{}if ({condition}) goto loc_{:x};",
                        indent(level),
                        target.get()
                    ),
                });
            }
            _ => {}
        }
    }
}

fn indent(level: usize) -> String {
    "    ".repeat(level)
}

fn intersect(
    order: &BTreeMap<usize, usize>,
    dom: &[Option<usize>],
    mut left: usize,
    mut right: usize,
) -> usize {
    while left != right {
        while order.get(&left).copied().unwrap_or(0) > order.get(&right).copied().unwrap_or(0) {
            left = dom[left].unwrap_or(left);
        }
        while order.get(&right).copied().unwrap_or(0) > order.get(&left).copied().unwrap_or(0) {
            right = dom[right].unwrap_or(right);
        }
    }
    left
}

fn negate(condition: &str) -> String {
    let trimmed = condition.trim();
    let flipped = trimmed
        .replace(" == ", " != ")
        .replace(" != ", " == ")
        .replace(" <= ", " > ")
        .replace(" >= ", " < ")
        .replace(" < ", " >= ")
        .replace(" > ", " <= ");
    if flipped != trimmed {
        flipped
    } else {
        format!("!({trimmed})")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use recraft_analysis::{BasicBlock, analyze, persist};
    use tempfile::TempDir;

    fn analyzed_corpus() -> Option<(Session, TempDir)> {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../corpus/build/x64/corpus.exe");
        let bytes = std::fs::read(path).ok()?;
        let dir = TempDir::new().unwrap();
        let project = dir.path().join("l1.rcraft");
        let mut session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let result = analyze(&mut session).unwrap();
        persist(&mut session, &result).unwrap();
        Some((session, dir))
    }

    fn analyzed_x86_corpus() -> Option<(Session, TempDir)> {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../corpus/build/x86/corpus.exe");
        let bytes = std::fs::read(path).ok()?;
        let dir = TempDir::new().unwrap();
        let project = dir.path().join("l1-x86.rcraft");
        let mut session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let result = analyze(&mut session).unwrap();
        persist(&mut session, &result).unwrap();
        Some((session, dir))
    }

    fn function_start(session: &Session, name: &str) -> Option<Address> {
        let address = session.addr_of_name(name).ok().flatten()?;
        session
            .functions()
            .ok()?
            .iter()
            .find(|function| function.start == address)
            .map(|function| function.start)
    }

    fn render(session: &mut Session, name: &str) -> String {
        let start = function_start(session, name).unwrap_or_else(|| panic!("{name} missing"));
        pseudocode_l1(session, start).unwrap().text()
    }

    #[test]
    fn branchy_becomes_if() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let text = render(&mut session, "branchy");
        assert!(text.contains("if ("), "{text}");
        assert!(text.contains('{') && text.contains('}'), "{text}");
    }

    #[test]
    fn loopy_becomes_while() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let text = render(&mut session, "loopy");
        assert!(text.contains("while ("), "{text}");
    }

    #[test]
    fn switch_is_structured_from_jump_table() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let text = render(&mut session, "switcher_big");
        // The selector register is traced back through the index move.
        assert!(text.contains("switch (ecx)"), "{text}");
        assert!(text.contains("case 0:"), "{text}");
        assert!(text.contains("case 19:"), "{text}");
        // Simple leaf cases are inlined; no goto fallback remains.
        assert!(text.contains("case 0: {"), "{text}");
        assert!(!text.contains("goto loc_"), "{text}");
    }

    #[test]
    fn l2_switch_selector_becomes_argument() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let start = function_start(&session, "switcher_big").unwrap();
        let text = pseudocode_l2(&mut session, start).unwrap().text();
        assert!(text.contains("switch (arg1)"), "{text}");
    }

    #[test]
    fn recursive_has_if_and_self_reference() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let text = render(&mut session, "recursive");
        assert!(text.contains("if ("), "{text}");
        assert!(text.contains("recursive"), "{text}");
    }

    #[test]
    fn l2_names_arguments_and_stack_slots() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let start = function_start(&session, "add_leaf").unwrap();
        let text = pseudocode_l2(&mut session, start).unwrap().text();
        assert!(text.contains("int64 arg1"), "{text}");
        assert!(text.contains("int64 arg2"), "{text}");
        assert!(
            text.contains("(arg1 +") && text.contains("arg2"),
            "argument reads were not renamed:\n{text}"
        );

        let nested = function_start(&session, "nested_loops").unwrap();
        let text = pseudocode_l2(&mut session, nested).unwrap().text();
        assert!(text.contains("var_"), "stack slots were not named:\n{text}");
    }

    #[test]
    fn l2_signature_has_return_type_and_abi() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let start = function_start(&session, "add_leaf").unwrap();
        let text = pseudocode_l2(&mut session, start).unwrap().text();
        assert!(
            text.contains("int32 add_leaf(int64 arg1, int64 arg2)"),
            "{text}"
        );
    }

    #[test]
    fn dataflow_marks_arguments_written_on_any_path() {
        let address = |value: u64| Address::new(value);
        let block = |index: usize, start: u64, edges: Vec<(usize, EdgeKind)>| BasicBlock {
            index,
            start: address(start),
            end: address(start + 0x10),
            insns: Vec::new(),
            edges,
        };
        let cfg = Cfg {
            function: address(0x1000),
            entry: 0,
            blocks: vec![
                block(
                    0,
                    0x1000,
                    vec![(1, EdgeKind::Taken), (2, EdgeKind::Fallthrough)],
                ),
                block(1, 0x1010, vec![(3, EdgeKind::Unconditional)]),
                block(2, 0x1020, vec![(3, EdgeKind::Unconditional)]),
                block(3, 0x1030, Vec::new()),
            ],
            external: Vec::new(),
            jump_tables: BTreeMap::new(),
        };
        let mut written = vec![BTreeSet::new(); 4];
        written[1].insert(Reg::Rcx);
        let seeds = entry_written(&cfg, &written);
        // Entering the writing block still holds the original argument.
        assert!(!seeds[1].contains(&Reg::Rcx));
        // The other arm is untouched.
        assert!(!seeds[2].contains(&Reg::Rcx));
        // At the join one path has overwritten rcx: may-write unions it.
        assert!(seeds[3].contains(&Reg::Rcx));
    }

    #[test]
    fn l2_pointer_parameters_show_pointer_types() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let start = function_start(&session, "str_probe").unwrap();
        let text = pseudocode_l2(&mut session, start).unwrap().text();
        assert!(text.contains("int32 str_probe(char* arg1)"), "{text}");
    }

    #[test]
    fn l2_renders_struct_field_access_through_pointer_arguments() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let start = function_start(&session, "field_probe").unwrap();
        let text = pseudocode_l2(&mut session, start).unwrap().text();
        assert!(text.contains("int32 field_probe(int32* arg1)"), "{text}");
        assert!(text.contains("arg1->field_10"), "{text}");
    }

    #[test]
    fn user_prototype_is_shown_above_the_signature_and_undoes() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let start = function_start(&session, "field_probe").unwrap();
        session
            .execute(recraft_core::Command::SetPrototype {
                addr: start,
                text: Some("int field_probe(const int* values)".to_owned()),
            })
            .unwrap();
        let text = pseudocode_l2(&mut session, start).unwrap().text();
        assert!(
            text.contains("// prototype: int field_probe(const int* values)"),
            "{text}"
        );
        session.undo().unwrap();
        let text = pseudocode_l2(&mut session, start).unwrap().text();
        assert!(!text.contains("// prototype:"), "{text}");
    }

    #[test]
    fn applied_struct_type_names_fields_in_pseudocode() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let start = function_start(&session, "field_probe").unwrap();
        session
            .execute(recraft_core::Command::DefineStruct {
                def: recraft_core::StructDef {
                    name: "Probe".to_owned(),
                    size: 0x18,
                    fields: vec![recraft_core::StructField {
                        offset: 0x10,
                        name: "value".to_owned(),
                        kind: "i32".to_owned(),
                    }],
                },
            })
            .unwrap();
        session
            .execute(recraft_core::Command::ApplyType {
                func: start,
                slot: 0,
                struct_name: Some("Probe".to_owned()),
            })
            .unwrap();
        let text = pseudocode_l2(&mut session, start).unwrap().text();
        assert!(text.contains("arg1->value"), "{text}");
        assert!(!text.contains("field_10"), "{text}");

        // Clearing the binding restores the offset-style fallback.
        session
            .execute(recraft_core::Command::ApplyType {
                func: start,
                slot: 0,
                struct_name: None,
            })
            .unwrap();
        let text = pseudocode_l2(&mut session, start).unwrap().text();
        assert!(text.contains("arg1->field_10"), "{text}");
    }

    #[test]
    fn user_prototype_overrides_signature_and_argument_names() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let start = function_start(&session, "field_probe").unwrap();
        session
            .execute(recraft_core::Command::SetPrototype {
                addr: start,
                text: Some("int __stdcall field_probe(const int* values, int count)".to_owned()),
            })
            .unwrap();
        let text = pseudocode_l2(&mut session, start).unwrap().text();
        assert!(
            text.contains("int32 __stdcall field_probe(int32* values, int32 count)"),
            "{text}"
        );
        assert!(text.contains("values->field_10"), "{text}");
        session.undo().unwrap();
        let text = pseudocode_l2(&mut session, start).unwrap().text();
        assert!(!text.contains("__stdcall"), "{text}");
    }

    #[test]
    fn scaled_index_addressing_renders_array_subscript() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let start = function_start(&session, "array_probe").unwrap();
        let text = pseudocode_l2(&mut session, start).unwrap().text();
        assert!(
            text.contains("int32 array_probe(int32* arg1, int64 arg2)"),
            "{text}"
        );
        assert!(text.contains("arg1["), "{text}");
        assert!(!text.contains("*(u32*)((arg1 +"), "{text}");
    }

    #[test]
    fn x86_prototype_maps_stack_arguments() {
        let Some((mut session, _dir)) = analyzed_x86_corpus() else {
            return;
        };
        let start = function_start(&session, "_stdcall_add@8").expect("stdcall_add");
        session
            .execute(recraft_core::Command::SetPrototype {
                addr: start,
                text: Some("int __stdcall stdcall_add(int a, int b)".to_owned()),
            })
            .unwrap();
        let text = pseudocode_l2(&mut session, start).unwrap().text();
        assert!(
            text.contains("int32 __stdcall stdcall_add(int32 a, int32 b)"),
            "{text}"
        );
        assert!(text.contains("eax = a;"), "{text}");
        assert!(text.contains("(eax + b)"), "{text}");
    }

    #[test]
    fn early_exit_chains_and_guard_then_loop_are_structured() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        for name in ["early_chain", "bytes_op"] {
            let start = function_start(&session, name).unwrap();
            let text = pseudocode_l1(&mut session, start).unwrap().text();
            assert!(
                !text.contains("goto loc_"),
                "{name} still has a goto: {text}"
            );
        }
        let start = function_start(&session, "bytes_op").unwrap();
        let text = pseudocode_l1(&mut session, start).unwrap().text();
        // Single-block loops become do-while; no goto fallback remains.
        assert!(text.contains("do {"), "{text}");
        assert!(text.contains("while ("), "{text}");
    }

    #[test]
    fn single_block_loops_emit_do_while() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let start = function_start(&session, "count_bits").unwrap();
        let text = pseudocode_l1(&mut session, start).unwrap().text();
        assert!(text.contains("do {"), "{text}");
        assert!(text.contains("} while ("), "{text}");
        assert!(!text.contains("goto loc_"), "{text}");
    }

    #[test]
    fn jump_table_cases_inline_simple_leaves() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let start = function_start(&session, "switcher_big").unwrap();
        let text = pseudocode_l1(&mut session, start).unwrap().text();
        assert!(text.contains("switch ("), "{text}");
        assert!(text.contains("case 0: {"), "{text}");
        assert!(text.contains("return;"), "{text}");
        assert!(!text.contains("goto loc_"), "{text}");
    }

    #[test]
    fn complex_loop_headers_stay_structured() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let start = function_start(&session, "nested_loops").unwrap();
        let text = pseudocode_l1(&mut session, start).unwrap().text();
        assert!(!text.contains("goto loc_"), "{text}");
        assert!(text.contains("while (1)"), "{text}");
        assert!(text.contains("do {"), "{text}");
    }

    #[test]
    fn call_sites_show_arguments_with_a_callee_prototype() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let start = function_start(&session, "recursive").unwrap();
        let before = pseudocode_l2(&mut session, start).unwrap().text();
        assert!(before.contains("recursive()"), "{before}");
        session
            .execute(recraft_core::Command::SetPrototype {
                addr: start,
                text: Some("int recursive(int n)".to_owned()),
            })
            .unwrap();
        let after = pseudocode_l2(&mut session, start).unwrap().text();
        assert!(after.contains("recursive(rcx)"), "{after}");
        assert!(!after.contains("recursive()"), "{after}");
    }

    #[test]
    fn x86_call_arguments_are_recovered_from_pushes() {
        let Some((mut session, _dir)) = analyzed_x86_corpus() else {
            return;
        };
        let start = function_start(&session, "recursive").expect("x86 recursive");
        session
            .execute(recraft_core::Command::SetPrototype {
                addr: start,
                text: Some("int __cdecl recursive(int n)".to_owned()),
            })
            .unwrap();
        let text = pseudocode_l2(&mut session, start).unwrap().text();
        assert!(text.contains("recursive("), "{text}");
        assert!(!text.contains("recursive()"), "{text}");
    }

    #[test]
    fn call_arguments_get_pointer_casts_from_callee_prototype() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let callee = function_start(&session, "str_probe").unwrap();
        session
            .execute(recraft_core::Command::SetPrototype {
                addr: callee,
                text: Some("int str_probe(const char* text)".to_owned()),
            })
            .unwrap();
        let caller = function_start(&session, "call_str").unwrap();
        let text = pseudocode_l2(&mut session, caller).unwrap().text();
        assert!(text.contains("str_probe((char*)"), "{text}");
        assert!(!text.contains("str_probe()"), "{text}");
    }

    #[test]
    fn output_is_deterministic() {
        let Some((mut session, _dir)) = analyzed_corpus() else {
            return;
        };
        let start = function_start(&session, "branchy").unwrap();
        let first = pseudocode_l1(&mut session, start).unwrap().text();
        let second = pseudocode_l1(&mut session, start).unwrap().text();
        assert_eq!(first, second);
    }
}
