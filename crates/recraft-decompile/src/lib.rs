// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Decompiler front-end.
//!
//! - **L0**: flat, instruction-faithful pseudo-C (straight-line + branches).
//! - **L1**: control-structure recovery (`if`/`else`, `while`, `switch`).
//! - **L2 (first cut)**: incoming argument registers (`arg1..arg4`) and named stack
//!   slots (`var_XX`), still without types or SSA.
//!
//! Every output line keeps its source address so the UI can cross-highlight and the AI
//! layer can cite evidence.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use recraft_common::{Address, Error, Result};
use recraft_core::{Session, StructDef};
use recraft_ir::{
    BinOp, Expr, ExprId, Flow, LiftedInsn, Lifter, Reg, RegView, Stmt, UnOp, cond_name,
};

mod l2;
mod quality;
mod structuring;

pub use l2::{
    ArgNames, CallingConvention, L2Info, ProtoParam, ProtoSig, ReturnKind, parse_prototype,
    recover_args, recover_l2,
};
pub use quality::{
    FunctionQuality, GoldenReport, QualityReport, check_golden, default_golden_dir, golden_text,
    measure, write_golden,
};
pub use structuring::{pseudocode_l1, pseudocode_l2};

/// One block's lifted statements plus its terminating flow (used by L1/L2).
pub(crate) struct BlockLift {
    /// Statements without the trailing semicolon or indentation.
    pub statements: Vec<PseudoLine>,
    /// Condition text for a conditional branch terminating the block.
    pub condition: Option<String>,
    pub flow: Flow,
    /// Registers written anywhere in the block (for cross-block argument tracking).
    pub written: BTreeSet<Reg>,
    /// Direct call targets seen in the block (for one-pass signature inference).
    pub calls: BTreeSet<u64>,
}

/// Lift one basic block (`[start, end)`) into statements + terminating flow.
///
/// `seed` carries registers already overwritten on every path reaching this block, so
/// argument names are not shown for clobbered values. `arg_structs` maps argument
/// registers to user-defined struct layouts (`apply_type`).
pub(crate) fn lift_block(
    session: &mut Session,
    start: Address,
    end: Address,
    args: Option<&ArgNames>,
    arg_structs: &BTreeMap<Reg, StructDef>,
    seed: Option<&BTreeSet<Reg>>,
    call_arg_counts: &HashMap<u64, usize>,
) -> Result<BlockLift> {
    if end <= start {
        return Ok(BlockLift {
            statements: Vec::new(),
            condition: None,
            flow: Flow::Fallthrough,
            written: BTreeSet::new(),
            calls: BTreeSet::new(),
        });
    }
    let mut lifter = Lifter::new();
    let mut lifted: Vec<LiftedInsn> = Vec::new();
    let mut addr = start;
    while addr < end {
        let Some(insn) = session.insn_at(addr)? else {
            break;
        };
        if insn.is_invalid() {
            break;
        }
        let item = lifter.lift(&insn);
        let next = insn.next_addr();
        lifted.push(item);
        if next >= end || next <= addr {
            break;
        }
        addr = next;
    }
    if lifted.is_empty() {
        return Ok(BlockLift {
            statements: Vec::new(),
            condition: None,
            flow: Flow::Fallthrough,
            written: BTreeSet::new(),
            calls: BTreeSet::new(),
        });
    }

    let exprs = lifter.into_exprs();
    let literals = collect_literals(session, &exprs)?;
    let imports = import_names(session);
    let mut names: HashMap<u64, String> = HashMap::new();
    for item in &lifted {
        for stmt in &item.stmts {
            if let Stmt::Call { target, .. } = stmt {
                names
                    .entry(target.get())
                    .or_insert(resolve_name(session, *target)?);
            }
        }
        if let Flow::Goto(target) | Flow::CondGoto { target, .. } = item.flow {
            names
                .entry(target.get())
                .or_insert(resolve_name(session, target)?);
        }
    }
    let mut call_protos: HashMap<u64, crate::l2::ProtoSig> = HashMap::new();
    for item in &lifted {
        for stmt in &item.stmts {
            if let Stmt::Call { target, .. } = stmt
                && let Some(text) = session.prototype_at(*target)?
                && let Some(proto) = crate::l2::parse_prototype(&text)
            {
                call_protos.insert(target.get(), proto);
            }
        }
    }
    let mut calls = BTreeSet::new();
    for item in &lifted {
        for stmt in &item.stmts {
            if let Stmt::Call { target, .. } = stmt {
                calls.insert(target.get());
            }
        }
    }
    let mut printer = Printer {
        exprs: &exprs,
        names: &names,
        literals: &literals,
        start,
        end,
        labels: &BTreeSet::new(),
        args,
        arg_structs,
        call_protos: &call_protos,
        call_arg_counts,
        imports: &imports,
        written: seed
            .map(|seed| seed.iter().copied().collect())
            .unwrap_or_default(),
        frame_reg: None,
        arg_values: [None, None, None, None],
        arg_aliases: BTreeMap::new(),
    };

    let mut statements: Vec<PseudoLine> = Vec::new();
    let mut written = BTreeSet::new();
    // Argument-register writes consumed by a call are redundant: the call already renders
    // its captured argument expressions.
    let mut flat: Vec<(usize, usize)> = Vec::new();
    for (item_index, item) in lifted.iter().enumerate() {
        for stmt_index in 0..item.stmts.len() {
            flat.push((item_index, stmt_index));
        }
    }
    let mut skip: HashSet<(usize, usize)> = HashSet::new();
    if session.image().is_64 {
        // A direct call captures the argument expressions of the callee's declared
        // parameters. Remove the preceding register setup for each captured argument, even
        // when unrelated instructions sit between the setup and the call, as long as the
        // register is not read in between.
        for (position, &(call_item, call_stmt)) in flat.iter().enumerate() {
            let Stmt::Call { target, args } = &lifted[call_item].stmts[call_stmt] else {
                continue;
            };
            for (slot, arg) in args.iter().enumerate().take(crate::l2::X64_ARG_REGS.len()) {
                let arg_reg = match exprs[*arg as usize] {
                    Expr::Reg(view) => Some(view.base),
                    _ => None,
                };
                if arg_reg == Some(crate::l2::X64_ARG_REGS[slot]) {
                    // The call renders the register itself; its writer cannot be
                    // removed without changing the value read by the call.
                    continue;
                }
                let limit =
                    call_arg_limit(target.get(), args, &exprs, &call_protos, call_arg_counts);
                if slot >= limit {
                    continue;
                }
                let reg = crate::l2::X64_ARG_REGS[slot];
                for previous in (0..position).rev() {
                    let (item_index, stmt_index) = flat[previous];
                    let stmt = &lifted[item_index].stmts[stmt_index];
                    // The defining write also reads the register (`xor edx, edx` is a
                    // self-cancelling write), so it must be recognised before the
                    // read check stops the scan.
                    if let Stmt::WriteReg {
                        reg: destination,
                        value,
                    } = stmt
                        && destination.base == reg
                    {
                        if value == arg
                            && setup_is_stable(
                                &exprs, &lifted, &flat, previous, position, *value, reg,
                            )
                        {
                            skip.insert((item_index, stmt_index));
                        }
                        break;
                    }
                    if stmt_reads_reg(stmt, &exprs, reg) {
                        break;
                    }
                }
            }
        }
        // An indirect import call (`call [IAT]`) renders its arguments from the values last
        // written to the ABI registers, so the same setup lines are redundant.
        for (position, &(call_item, call_stmt)) in flat.iter().enumerate() {
            let Stmt::CallIndirect { target } = &lifted[call_item].stmts[call_stmt] else {
                continue;
            };
            if !is_import_call(&exprs, *target, &imports) {
                continue;
            }
            for reg in crate::l2::X64_ARG_REGS {
                for previous in (0..position).rev() {
                    let (item_index, stmt_index) = flat[previous];
                    let stmt = &lifted[item_index].stmts[stmt_index];
                    if let Stmt::WriteReg {
                        reg: destination,
                        value,
                    } = stmt
                        && destination.base == reg
                    {
                        if setup_is_stable(&exprs, &lifted, &flat, previous, position, *value, reg)
                        {
                            skip.insert((item_index, stmt_index));
                        }
                        break;
                    }
                    if stmt_reads_reg(stmt, &exprs, reg) {
                        break;
                    }
                }
            }
        }
    }
    for (item_index, item) in lifted.iter().enumerate() {
        for (stmt_index, stmt) in item.stmts.iter().enumerate() {
            let skipped = skip.contains(&(item_index, stmt_index));
            if let Stmt::WriteReg { reg, .. } = stmt {
                written.insert(reg.base);
            }
            printer.retire_frame_alias(stmt);
            if printer.is_noop_write(stmt) || is_padding_intrinsic(stmt) {
                continue;
            }
            let text = printer.stmt(stmt);
            // Consecutive identical pure writes are idempotent and common after
            // compiler-generated copy chains. Calls/intrinsics must never be folded.
            let pure = !matches!(
                stmt,
                Stmt::Call { .. } | Stmt::CallIndirect { .. } | Stmt::Intrinsic { .. }
            );
            if !skipped && !(pure && statements.last().is_some_and(|line| line.text == text)) {
                statements.push(PseudoLine {
                    addr: item.addr,
                    text: format!("{text};"),
                });
            }
            // A skipped setup still happened on the machine: later reads must not show the
            // old argument name, and an indirect call still prints the value it received.
            printer.note_write(stmt);
            printer.note_arg_alias(stmt);
            if let Stmt::WriteReg { reg, value } = stmt
                && let Some(slot) = arg_slot(reg.base)
            {
                let rendered = if reg.width >= 4 {
                    printer.arg_expr(*value)
                } else {
                    reg.name().to_owned()
                };
                printer.arg_values[slot] = Some(rendered);
            }
            if matches!(
                stmt,
                Stmt::Call { .. } | Stmt::CallIndirect { .. } | Stmt::Intrinsic { .. }
            ) {
                printer.arg_values = [None, None, None, None];
            }
        }
    }
    let last = lifted.last().unwrap();
    let condition = match last.flow {
        Flow::CondGoto { cond, .. } => Some(printer.expr(cond)),
        _ => None,
    };
    Ok(BlockLift {
        statements,
        condition,
        flow: last.flow,
        written,
        calls,
    })
}

/// Number of leading call arguments to print for a call without a user prototype.
///
/// All x64 argument slots are captured by the lifter, even when they still hold the
/// incoming register. Print through the last *computed* slot so calls stop looking
/// argument-less whenever the caller actually prepared a value; trailing untouched
/// registers stay implicit instead of producing `(rcx, rdx, r8, r9)` noise.
fn rendered_arg_count(args: &[ExprId], exprs: &[Expr]) -> usize {
    args.iter()
        .rposition(|arg| !matches!(exprs[*arg as usize], Expr::Reg(_)))
        .map_or(0, |index| index + 1)
}

fn call_arg_limit(
    target: u64,
    args: &[ExprId],
    exprs: &[Expr],
    protos: &HashMap<u64, crate::l2::ProtoSig>,
    counts: &HashMap<u64, usize>,
) -> usize {
    if let Some(proto) = protos.get(&target) {
        proto.params.len().min(args.len())
    } else if let Some(count) = counts.get(&target) {
        (*count).min(args.len())
    } else {
        rendered_arg_count(args, exprs)
    }
}

fn expr_reads_reg(exprs: &[Expr], expr: ExprId, reg: Reg) -> bool {
    match &exprs[expr as usize] {
        Expr::Const(_) | Expr::Temp(_) => false,
        Expr::Reg(view) => view.base == reg,
        Expr::Load { addr, .. } | Expr::LoadFloat { addr, .. } => expr_reads_reg(exprs, *addr, reg),
        Expr::Bin { lhs, rhs, .. } | Expr::Cmp { lhs, rhs, .. } => {
            expr_reads_reg(exprs, *lhs, reg) || expr_reads_reg(exprs, *rhs, reg)
        }
        Expr::Un { arg, .. } | Expr::Cast { arg, .. } => expr_reads_reg(exprs, *arg, reg),
        Expr::Select {
            cond,
            then,
            otherwise,
        } => {
            expr_reads_reg(exprs, *cond, reg)
                || expr_reads_reg(exprs, *then, reg)
                || expr_reads_reg(exprs, *otherwise, reg)
        }
    }
}

fn stmt_reads_reg(stmt: &Stmt, exprs: &[Expr], reg: Reg) -> bool {
    match stmt {
        Stmt::WriteReg { value, .. } => expr_reads_reg(exprs, *value, reg),
        Stmt::Store { addr, value, .. } | Stmt::StoreFloat { addr, value, .. } => {
            expr_reads_reg(exprs, *addr, reg) || expr_reads_reg(exprs, *value, reg)
        }
        Stmt::Call { args, .. } => args.iter().any(|arg| expr_reads_reg(exprs, *arg, reg)),
        Stmt::CallIndirect { target } => expr_reads_reg(exprs, *target, reg),
        Stmt::Intrinsic { .. } => false,
    }
}

/// Registers an expression really depends on, ignoring self-cancelling forms the printer
/// folds to a constant (`xor eax, eax`, `sub eax, eax`, `and eax, 0`). A register setup
/// whose inputs are untouched can be evaluated at the call site instead.
fn expr_live_regs(exprs: &[Expr], expr: ExprId, out: &mut BTreeSet<Reg>) {
    match &exprs[expr as usize] {
        Expr::Const(_) | Expr::Temp(_) => {}
        Expr::Reg(view) => {
            out.insert(view.base);
        }
        Expr::Bin { op, lhs, rhs } => {
            if same_value(exprs, *lhs, *rhs) && matches!(op, BinOp::Xor | BinOp::Sub) {
                return;
            }
            if *op == BinOp::And
                && (matches!(exprs[*lhs as usize], Expr::Const(0))
                    || matches!(exprs[*rhs as usize], Expr::Const(0)))
            {
                return;
            }
            expr_live_regs(exprs, *lhs, out);
            expr_live_regs(exprs, *rhs, out);
        }
        Expr::Cmp { lhs, rhs, .. } => {
            expr_live_regs(exprs, *lhs, out);
            expr_live_regs(exprs, *rhs, out);
        }
        Expr::Load { addr, .. } | Expr::LoadFloat { addr, .. } => {
            expr_live_regs(exprs, *addr, out);
        }
        Expr::Un { arg, .. } | Expr::Cast { arg, .. } => expr_live_regs(exprs, *arg, out),
        Expr::Select {
            cond,
            then,
            otherwise,
        } => {
            expr_live_regs(exprs, *cond, out);
            expr_live_regs(exprs, *then, out);
            expr_live_regs(exprs, *otherwise, out);
        }
    }
}

/// True when two expressions are the same value even if they are distinct arena entries
/// (the lifter materialises a fresh register read for each operand).
fn same_value(exprs: &[Expr], left: ExprId, right: ExprId) -> bool {
    match (&exprs[left as usize], &exprs[right as usize]) {
        (Expr::Reg(a), Expr::Reg(b)) => a == b,
        (Expr::Const(a), Expr::Const(b)) => a == b,
        (Expr::Temp(a), Expr::Temp(b)) => a == b,
        _ => left == right,
    }
}

/// True when an indirect call target is a load of an import slot, which the printer renders
/// as a named call with an argument list.
fn is_import_call(exprs: &[Expr], target: ExprId, imports: &HashMap<u64, String>) -> bool {
    let Expr::Load { addr, .. } = exprs[target as usize] else {
        return false;
    };
    matches!(exprs[addr as usize], Expr::Const(value) if imports.contains_key(&value))
}

/// True when a value expression reads memory, so a store before the call could change it.
fn expr_has_load(exprs: &[Expr], expr: ExprId) -> bool {
    match &exprs[expr as usize] {
        Expr::Load { .. } | Expr::LoadFloat { .. } => true,
        Expr::Const(_) | Expr::Temp(_) | Expr::Reg(_) => false,
        Expr::Bin { lhs, rhs, .. } | Expr::Cmp { lhs, rhs, .. } => {
            expr_has_load(exprs, *lhs) || expr_has_load(exprs, *rhs)
        }
        Expr::Un { arg, .. } | Expr::Cast { arg, .. } => expr_has_load(exprs, *arg),
        Expr::Select {
            cond,
            then,
            otherwise,
        } => {
            expr_has_load(exprs, *cond)
                || expr_has_load(exprs, *then)
                || expr_has_load(exprs, *otherwise)
        }
    }
}

/// Whether a register setup can be deferred to the call site: the value must not read the
/// destination register itself, none of its inputs may change between the write and the
/// call, and a memory read must not be invalidated by a store in between.
fn setup_is_stable(
    exprs: &[Expr],
    lifted: &[recraft_ir::LiftedInsn],
    flat: &[(usize, usize)],
    previous: usize,
    position: usize,
    value: ExprId,
    register: Reg,
) -> bool {
    let mut sources = BTreeSet::new();
    expr_live_regs(exprs, value, &mut sources);
    if sources.contains(&register) {
        return false;
    }
    let reads_memory = expr_has_load(exprs, value);
    for &(item_index, stmt_index) in &flat[previous + 1..position] {
        let stmt = &lifted[item_index].stmts[stmt_index];
        if matches!(
            stmt,
            Stmt::Call { .. } | Stmt::CallIndirect { .. } | Stmt::Intrinsic { .. }
        ) {
            return false;
        }
        if reads_memory && matches!(stmt, Stmt::Store { .. } | Stmt::StoreFloat { .. }) {
            return false;
        }
        if let Stmt::WriteReg { reg, .. } = stmt
            && sources.contains(&reg.base)
        {
            return false;
        }
    }
    true
}

/// Alignment padding (`int3`) and `nop` are compiler artifacts, not source code.
fn is_padding_intrinsic(stmt: &Stmt) -> bool {
    matches!(stmt, Stmt::Intrinsic { text } if matches!(text.trim(), "int3" | "nop"))
}

/// One line of pseudo-C, with its source address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PseudoLine {
    pub addr: Address,
    pub text: String,
}

/// Rendered function.
#[derive(Debug, Clone)]
pub struct Pseudocode {
    pub func_start: Address,
    pub lines: Vec<PseudoLine>,
}

impl Pseudocode {
    /// Plain text, one line per entry (without addresses).
    pub fn text(&self) -> String {
        let mut out = String::new();
        for line in &self.lines {
            out.push_str(&line.text);
            out.push('\n');
        }
        out
    }
}

/// Render L0 pseudo-C for the function starting at `start`.
pub fn pseudocode_l0(
    session: &mut Session,
    start: Address,
    max_insns: usize,
) -> Result<Pseudocode> {
    let mut lifter = Lifter::new();
    let mut lifted: Vec<LiftedInsn> = Vec::new();
    let mut targets: BTreeSet<u64> = BTreeSet::new();
    let mut calls: BTreeSet<u64> = BTreeSet::new();
    let mut addr = start;
    let mut end = start;

    for _ in 0..max_insns {
        let Some(insn) = session.insn_at(addr)? else {
            break;
        };
        if insn.is_invalid() {
            break;
        }
        let item = lifter.lift(&insn);
        for stmt in &item.stmts {
            if let Stmt::Call { target, .. } = stmt {
                calls.insert(target.get());
            }
        }
        if let Flow::Goto(target) | Flow::CondGoto { target, .. } = item.flow {
            targets.insert(target.get());
        }
        let stop = matches!(item.flow, Flow::Return | Flow::Goto(_));
        end = insn.next_addr();
        lifted.push(item);
        if stop {
            break;
        }
        addr = end;
    }

    if lifted.is_empty() {
        return Err(Error::internal(format!(
            "nothing to decompile at {:#x}",
            start.get()
        )));
    }

    let exprs = lifter.into_exprs();
    let literals = collect_literals(session, &exprs)?;
    let imports = import_names(session);
    let mut names: HashMap<u64, String> = HashMap::new();
    for target in targets.iter().chain(calls.iter()) {
        let target = Address::new(*target);
        names.insert(target.get(), resolve_name(session, target)?);
    }

    let no_structs: BTreeMap<Reg, StructDef> = BTreeMap::new();
    let no_protos: HashMap<u64, crate::l2::ProtoSig> = HashMap::new();
    let no_counts: HashMap<u64, usize> = HashMap::new();
    let mut printer = Printer {
        exprs: &exprs,
        names: &names,
        literals: &literals,
        start,
        end,
        labels: &targets,
        args: None,
        arg_structs: &no_structs,
        call_protos: &no_protos,
        call_arg_counts: &no_counts,
        imports: &imports,
        written: HashSet::new(),
        frame_reg: None,
        arg_values: [None, None, None, None],
        arg_aliases: BTreeMap::new(),
    };
    let lines = printer.render(&lifted);
    Ok(Pseudocode {
        func_start: start,
        lines,
    })
}

fn resolve_name(session: &Session, addr: Address) -> Result<String> {
    if let Some(name) = session.name_at(addr)? {
        return Ok(name);
    }
    Ok(format!("sub_{:x}", addr.get()))
}

/// Position of a register in the x64 integer-argument order (`rcx`, `rdx`, `r8`, `r9`).
fn arg_slot(reg: Reg) -> Option<usize> {
    crate::l2::X64_ARG_REGS
        .iter()
        .position(|candidate| *candidate == reg)
}

/// Imported function names keyed by their IAT slot address. Indirect calls through the
/// IAT are otherwise rendered as `(**(u64*)(0x...))()`, which hides which API is called.
fn import_names(session: &Session) -> HashMap<u64, String> {
    let mut names = HashMap::new();
    for module in &session.image().imports {
        for function in &module.functions {
            if let Some(name) = &function.name {
                // Keep the rendered call valid C even for decorated/ordinal names.
                let sanitized: String = name
                    .chars()
                    .map(|ch| {
                        if ch.is_ascii_alphanumeric() || ch == '_' {
                            ch
                        } else {
                            '_'
                        }
                    })
                    .collect();
                names.insert(function.iat_va.get(), sanitized);
            }
        }
    }
    names
}

/// Quote a decoded string as a C literal, escaping quotes, backslashes and control bytes.
fn quote_c_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => {
                for byte in ch.to_string().as_bytes() {
                    out.push_str(&format!("\\x{byte:02x}"));
                }
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

/// Resolve address constants that name a stored string to quoted C literals.
fn collect_literals(session: &mut Session, exprs: &[Expr]) -> Result<HashMap<u64, String>> {
    let mut literals = HashMap::new();
    let mut values: BTreeSet<u64> = exprs
        .iter()
        .filter_map(|expr| match expr {
            Expr::Const(value) => Some(*value),
            _ => None,
        })
        .collect();
    values.retain(|value| *value > 0x10000);
    for value in values {
        let addr = Address::new(value);
        if let Some(text) = session.string_at(addr)? {
            if !text.is_empty() {
                literals.insert(value, quote_c_string(&text));
            }
            continue;
        }
        // Short literals such as `"\r\n"` are not always in the string table; read them
        // directly when the address lies in read-only data.
        let in_rdata = session
            .segment_containing(addr)
            .is_some_and(|segment| segment.name == ".rdata");
        if in_rdata && let Some(text) = read_c_literal(session, addr) {
            literals.insert(value, quote_c_string(&text));
        }
    }
    Ok(literals)
}

/// Read a NUL-terminated printable ASCII literal for short data not in the string table.
fn read_c_literal(session: &mut Session, addr: Address) -> Option<String> {
    let mut bytes = [0u8; 64];
    let read = session.read_at(addr, &mut bytes).ok()?;
    let bytes = &bytes[..read];
    let end = bytes.iter().position(|byte| *byte == 0)?;
    if end == 0 {
        return None;
    }
    let text = &bytes[..end];
    let printable = text
        .iter()
        .all(|byte| matches!(byte, b'\r' | b'\n' | b'\t' | 0x20..=0x7e));
    printable.then(|| String::from_utf8_lossy(text).into_owned())
}

struct Printer<'a> {
    exprs: &'a [Expr],
    names: &'a HashMap<u64, String>,
    start: Address,
    end: Address,
    labels: &'a BTreeSet<u64>,
    args: Option<&'a ArgNames>,
    /// Struct layouts bound to argument registers (`apply_type`).
    arg_structs: &'a BTreeMap<Reg, StructDef>,
    /// User prototypes of direct call targets (renders call arguments/types).
    call_protos: &'a HashMap<u64, crate::l2::ProtoSig>,
    /// Inferred argument counts for direct call targets without a user prototype.
    call_arg_counts: &'a HashMap<u64, usize>,
    /// Quoted C literals for address constants that name a stored string.
    literals: &'a HashMap<u64, String>,
    /// Imported function names keyed by the address of their IAT slot.
    imports: &'a HashMap<u64, String>,
    /// Registers written so far (for argument-register naming).
    written: HashSet<Reg>,
    /// Register that aliases `rsp` for frame-relative locals (e.g. `mov r11, rsp`).
    frame_reg: Option<Reg>,
    /// Rendered values last written to the x64 argument registers, for naming the
    /// arguments of an indirect IAT call (`CreateFileW(path, access, ...)`).
    arg_values: [Option<String>; 4],
    /// Registers that still hold an incoming argument copied from its ABI register
    /// (`rsi = arg1`), so later reads print the parameter name instead of the register.
    arg_aliases: BTreeMap<Reg, String>,
}

impl Printer<'_> {
    fn render(&mut self, lifted: &[LiftedInsn]) -> Vec<PseudoLine> {
        let mut lines = Vec::new();
        for insn in lifted {
            if self.labels.contains(&insn.addr.get()) {
                lines.push(PseudoLine {
                    addr: insn.addr,
                    text: format!("loc_{:x}:", insn.addr.get()),
                });
            }
            // `jmp [IAT]` lifts to a call statement plus `Flow::Return`; keep the tail call
            // as `return Api();` instead of a call followed by a bare `return;`.
            let tail_call = matches!(insn.flow, Flow::Return)
                && matches!(insn.stmts.last(), Some(Stmt::CallIndirect { .. }));
            for (index, stmt) in insn.stmts.iter().enumerate() {
                self.retire_frame_alias(stmt);
                if self.is_noop_write(stmt) || is_padding_intrinsic(stmt) {
                    continue;
                }
                let text = self.stmt(stmt);
                if tail_call && index + 1 == insn.stmts.len() {
                    lines.push(PseudoLine {
                        addr: insn.addr,
                        text: format!("    return {text};"),
                    });
                } else {
                    lines.push(PseudoLine {
                        addr: insn.addr,
                        text: format!("    {text};"),
                    });
                }
                self.note_write(stmt);
                self.note_arg_alias(stmt);
                if let Stmt::WriteReg { reg, value } = stmt
                    && let Some(slot) = arg_slot(reg.base)
                {
                    let rendered = if reg.width >= 4 {
                        self.arg_expr(*value)
                    } else {
                        reg.name().to_owned()
                    };
                    self.arg_values[slot] = Some(rendered);
                }
                if matches!(
                    stmt,
                    Stmt::Call { .. } | Stmt::CallIndirect { .. } | Stmt::Intrinsic { .. }
                ) {
                    self.arg_values = [None, None, None, None];
                }
            }
            match insn.flow {
                Flow::Fallthrough => {}
                Flow::Return if tail_call => {}
                Flow::Return => lines.push(PseudoLine {
                    addr: insn.addr,
                    text: "    return;".to_owned(),
                }),
                Flow::Goto(target) => {
                    let text = if self.is_local(target) {
                        format!("    goto loc_{:x};", target.get())
                    } else {
                        format!("    return {}();", self.name(target))
                    };
                    lines.push(PseudoLine {
                        addr: insn.addr,
                        text,
                    });
                }
                Flow::CondGoto { cond, target } => {
                    let cond_expr = self.expr(cond);
                    let text = if self.is_local(target) {
                        format!("    if ({cond_expr}) goto loc_{:x};", target.get())
                    } else {
                        format!("    if ({cond_expr}) return {}();", self.name(target))
                    };
                    lines.push(PseudoLine {
                        addr: insn.addr,
                        text,
                    });
                }
            }
        }
        lines
    }

    /// Retire the `frame` name when the aliased register is overwritten with anything
    /// other than `rsp`; afterwards it is an ordinary scratch register again.
    fn retire_frame_alias(&mut self, stmt: &Stmt) {
        if let Stmt::WriteReg { reg, value } = stmt
            && self.frame_reg == Some(reg.base)
            && !matches!(self.exprs[*value as usize], Expr::Reg(view) if view.base == Reg::Rsp)
        {
            self.frame_reg = None;
        }
    }

    /// Records a register write so later reads no longer render as `argN`.
    fn note_write(&mut self, stmt: &Stmt) {
        if let Stmt::WriteReg { reg, value } = stmt {
            self.written.insert(reg.base);
            // `mov r11, rsp` (MSVC exception-handler frames) — keep the alias readable.
            if self.frame_reg.is_none()
                && reg.base != Reg::Rsp
                && matches!(self.exprs[*value as usize], Expr::Reg(view) if view.base == Reg::Rsp)
            {
                self.frame_reg = Some(reg.base);
            }
        }
    }

    fn is_noop_write(&mut self, stmt: &Stmt) -> bool {
        let Stmt::WriteReg { reg, value } = stmt else {
            return false;
        };
        // Stack-pointer bookkeeping (`sub rsp, N` / `add rsp, N`, push/pop) is already
        // reflected in stack-relative slot names, so it is noise in pseudo-C.
        if reg.base == Reg::Rsp {
            return true;
        }
        // `mov r11, rsp` (MSVC exception-handler frames): name the alias before printing
        // so the statement itself reads `frame = rsp`.
        if self.frame_reg.is_none()
            && matches!(self.exprs[*value as usize], Expr::Reg(view) if view.base == Reg::Rsp)
        {
            self.frame_reg = Some(reg.base);
        }
        matches!(self.exprs[*value as usize], Expr::Reg(view) if view == *reg)
    }

    fn is_local(&self, target: Address) -> bool {
        self.start.get() <= target.get() && target.get() < self.end.get()
    }

    fn name(&self, target: Address) -> String {
        if let Some(proto) = self.call_protos.get(&target.get())
            && let Some(name) = proto.name.as_deref()
        {
            return name.to_owned();
        }
        self.names
            .get(&target.get())
            .cloned()
            .unwrap_or_else(|| format!("sub_{:x}", target.get()))
    }

    /// Register reads render as `argN` while the register still holds its incoming
    /// argument value.
    fn reg_name(&self, view: &recraft_ir::RegView) -> String {
        if self.frame_reg == Some(view.base) {
            return "frame".to_owned();
        }
        if view.width == 8
            && let Some(name) = self.arg_aliases.get(&view.base)
        {
            return name.clone();
        }
        if let Some(args) = self.args
            && let Some(name) = args.name_for(view.base)
            && !self.written.contains(&view.base)
        {
            return name.to_owned();
        }
        view.name().to_owned()
    }

    /// Record or drop the argument alias carried by a register write. A full-width copy of
    /// an argument register (`rsi = arg1`) keeps naming the parameter until the register is
    /// overwritten; every other write clears the alias.
    fn note_arg_alias(&mut self, stmt: &Stmt) {
        if matches!(
            stmt,
            Stmt::Call { .. } | Stmt::CallIndirect { .. } | Stmt::Intrinsic { .. }
        ) {
            for reg in [
                Reg::Rcx,
                Reg::Rdx,
                Reg::R8,
                Reg::R9,
                Reg::Rax,
                Reg::R10,
                Reg::R11,
            ] {
                self.arg_aliases.remove(&reg);
            }
        }
        if let Stmt::WriteReg { reg, value } = stmt {
            let alias = match &self.exprs[*value as usize] {
                Expr::Reg(view) if view.width == 8 => {
                    self.arg_aliases.get(&view.base).cloned().or_else(|| {
                        self.args
                            .and_then(|args| args.name_for(view.base))
                            .filter(|_| !self.written.contains(&view.base))
                            .map(str::to_owned)
                    })
                }
                _ => None,
            };
            self.arg_aliases.remove(&reg.base);
            if let Some(alias) = alias {
                self.arg_aliases.insert(reg.base, alias);
            }
        }
    }

    /// `[rsp + const]`, `[rbp - const]`, `[ebp + const]` -> (base register, offset).
    fn stack_slot(&self, expr: ExprId) -> Option<(Reg, i64)> {
        let (op, lhs, rhs) = match &self.exprs[expr as usize] {
            Expr::Bin { op, lhs, rhs } => (op, *lhs, *rhs),
            // A bare `[rsp]` / `[rbp]` write is the frame slot at offset 0 (`push`).
            Expr::Reg(view)
                if matches!(view.base, Reg::Rsp | Reg::Rbp)
                    || self.frame_reg == Some(view.base) =>
            {
                return Some((view.base, 0));
            }
            _ => return None,
        };
        let base_of = |id: ExprId, exprs: &[Expr]| match &exprs[id as usize] {
            Expr::Reg(view)
                if matches!(view.base, Reg::Rsp | Reg::Rbp)
                    || self.frame_reg == Some(view.base) =>
            {
                Some(view.base)
            }
            _ => None,
        };
        let const_of = |id: ExprId, exprs: &[Expr]| match exprs[id as usize] {
            Expr::Const(value) => Some(value as i64),
            _ => None,
        };
        match op {
            // `[rsp + N]` / `[rbp + N]`
            recraft_ir::BinOp::Add => {
                if let Some(base) = base_of(lhs, self.exprs) {
                    const_of(rhs, self.exprs).map(|offset| (base, offset))
                } else if let Some(base) = base_of(rhs, self.exprs) {
                    const_of(lhs, self.exprs).map(|offset| (base, offset))
                } else {
                    None
                }
            }
            // `[rsp - N]` / `[rbp - N]` (e.g. the saved-register slot of a `push`)
            recraft_ir::BinOp::Sub => {
                let base = base_of(lhs, self.exprs)?;
                const_of(rhs, self.exprs).map(|offset| (base, -offset))
            }
            _ => None,
        }
    }

    fn stack_name(offset: i64) -> String {
        if offset < 0 {
            format!("var_n{:x}", -offset)
        } else {
            format!("var_{offset:x}")
        }
    }

    /// C spelling for a load/store width (16-byte operands are SSE vectors).
    fn scalar_type(size: u8) -> String {
        match size {
            16 => "__m128".to_owned(),
            other => format!("u{}", u16::from(other) * 8),
        }
    }

    fn float_type(size: u8) -> &'static str {
        if size == 8 { "f64" } else { "f32" }
    }

    /// Render a stack reference: a recovered x86 argument name, otherwise `var_XX` for
    /// rsp slots and frame-pointer locals. Positive `[rbp+...]` stays a raw pointer while
    /// rbp is the incoming frame (it may be the return address / caller frame); once the
    /// function assigns rbp itself, positive offsets are locals of the rebuilt frame.
    fn stack_reference(&self, base: Reg, offset: i64) -> Option<String> {
        if let Some(name) = self.args.and_then(|args| args.name_for_stack(base, offset)) {
            return Some(name.to_owned());
        }
        let rebuilt_frame = base == Reg::Rbp && self.written.contains(&Reg::Rbp);
        if base == Reg::Rsp || offset < 0 || rebuilt_frame || self.frame_reg == Some(base) {
            return Some(Self::stack_name(offset));
        }
        None
    }

    /// `argN->field_XX` for an offset access through a pointer-typed argument.
    fn field_access(&self, expr: ExprId) -> Option<String> {
        let Expr::Bin { op, lhs, rhs } = &self.exprs[expr as usize] else {
            return None;
        };
        if *op != BinOp::Add {
            return None;
        }
        let (lhs, rhs) = (*lhs, *rhs);
        let view = match &self.exprs[lhs as usize] {
            Expr::Reg(view) => *view,
            _ => return None,
        };
        let offset = match &self.exprs[rhs as usize] {
            Expr::Const(value) => *value as i64,
            _ => return None,
        };
        let args = self.args?;
        if !args.is_pointer_arg(view.base) || self.written.contains(&view.base) {
            return None;
        }
        let name = args.name_for(view.base)?;
        if let Some(def) = self.arg_structs.get(&view.base)
            && offset >= 0
            && let Some(field) = def.field_at(offset as u64)
        {
            return Some(format!("{name}->{}", field.name));
        }
        let field = if offset < 0 {
            format!("field_n{:x}", -offset)
        } else {
            format!("field_{offset:x}")
        };
        Some(format!("{name}->{field}"))
    }

    /// `argN[index]` for `base + index*scale` addressing through a typed pointer.
    fn array_access(&self, expr: ExprId) -> Option<String> {
        let Expr::Bin { op, lhs, rhs } = &self.exprs[expr as usize] else {
            return None;
        };
        if *op != BinOp::Add {
            return None;
        }
        let (lhs, rhs) = (*lhs, *rhs);
        let view = match &self.exprs[lhs as usize] {
            Expr::Reg(view) => *view,
            _ => return None,
        };
        if self.written.contains(&view.base) {
            return None;
        }
        let args = self.args?;
        let size = args.pointer_elem_size(view.base)?;
        let index = self.index_expr(rhs, size)?;
        let name = args.name_for(view.base)?;
        Some(format!("{name}[{index}]"))
    }

    /// Render the index of a scaled access (`idx * size`, `idx << log2(size)`); unscaled
    /// indices are only plausible for byte-sized pointers.
    fn index_expr(&self, expr: ExprId, size: u64) -> Option<String> {
        match &self.exprs[expr as usize] {
            Expr::Const(_) => None,
            Expr::Bin { op, lhs, rhs } if *op == BinOp::Mul => {
                if let Expr::Const(value) = &self.exprs[*lhs as usize]
                    && *value == size
                {
                    return Some(self.expr(*rhs));
                }
                if let Expr::Const(value) = &self.exprs[*rhs as usize]
                    && *value == size
                {
                    return Some(self.expr(*lhs));
                }
                None
            }
            Expr::Bin { op, lhs, rhs } if *op == BinOp::Shl => {
                if let Expr::Const(value) = &self.exprs[*rhs as usize]
                    && *value >= 1
                    && (1u64 << (*value as u32).min(63)) == size
                {
                    return Some(self.expr(*lhs));
                }
                None
            }
            _ if size == 1 => Some(self.expr(expr)),
            _ => None,
        }
    }

    /// Render a value that is stored or assigned. A stack-slot address becomes `&var_XX`,
    /// which is how the source spells a local passed or kept by pointer.
    fn value_expr(&self, id: ExprId) -> String {
        // A bare register (`mov r11, rsp`) is the frame pointer itself, not `&var_0`.
        if !matches!(self.exprs[id as usize], Expr::Reg(_))
            && let Some((base, offset)) = self.stack_slot(id)
            && let Some(name) = self.stack_reference(base, offset)
        {
            return format!("&{name}");
        }
        self.expr(id)
    }

    fn stmt(&self, stmt: &Stmt) -> String {
        match stmt {
            Stmt::WriteReg { reg, value } => {
                // Writes always use the concrete register view; argument names only
                // re-label reads of the still-intact incoming value. The frame alias is
                // the one exception so `mov r11, rsp` reads as `frame = rsp`.
                let name = if self.frame_reg == Some(reg.base) {
                    "frame".to_owned()
                } else {
                    reg.name().to_owned()
                };
                format!("{name} = {}", self.value_expr(*value))
            }
            Stmt::Store { addr, value, size } => {
                // Writes never use incoming argument names: storing to a caller's
                // argument slot is rare and would read as reassigning a variable.
                if let Some((base, offset)) = self.stack_slot(*addr)
                    && let Some(name) = self.stack_reference(base, offset)
                {
                    format!("{name} = {}", self.value_expr(*value))
                } else if let Some(index) = self.array_access(*addr) {
                    format!("{index} = {}", self.value_expr(*value))
                } else if let Some(field) = self.field_access(*addr) {
                    format!("{field} = {}", self.value_expr(*value))
                } else {
                    format!(
                        "*({}*)({}) = {}",
                        Self::scalar_type(*size),
                        self.expr(*addr),
                        self.value_expr(*value)
                    )
                }
            }
            Stmt::StoreFloat { addr, value, size } => {
                if let Some((base, offset)) = self.stack_slot(*addr)
                    && let Some(name) = self.stack_reference(base, offset)
                {
                    format!("{name} = {}", self.value_expr(*value))
                } else if let Some(index) = self.array_access(*addr) {
                    format!("{index} = {}", self.value_expr(*value))
                } else if let Some(field) = self.field_access(*addr) {
                    format!("{field} = {}", self.value_expr(*value))
                } else {
                    format!(
                        "*({}*)({}) = {}",
                        Self::float_type(*size),
                        self.expr(*addr),
                        self.value_expr(*value)
                    )
                }
            }
            Stmt::Call { target, args } => {
                let name = self.name(*target);
                match self.call_protos.get(&target.get()) {
                    Some(proto) => {
                        let count = proto.params.len().min(args.len());
                        let rendered: Vec<String> = args
                            .iter()
                            .take(count)
                            .enumerate()
                            .map(|(index, arg)| {
                                let value = self.arg_expr(*arg);
                                match proto.params.get(index) {
                                    // Pointer parameters make the argument's intended
                                    // type visible; scalars stay uncast for readability.
                                    Some(param) if param.ty.is_pointer() => {
                                        format!("({}){value}", param.ty.c_name())
                                    }
                                    _ => value,
                                }
                            })
                            .collect();
                        format!("{name}({})", rendered.join(", "))
                    }
                    None => {
                        let count = call_arg_limit(
                            target.get(),
                            args,
                            self.exprs,
                            self.call_protos,
                            self.call_arg_counts,
                        );
                        if count == 0 {
                            format!("{name}()")
                        } else {
                            let rendered: Vec<String> = args
                                .iter()
                                .take(count)
                                .map(|arg| self.arg_expr(*arg))
                                .collect();
                            format!("{name}({})", rendered.join(", "))
                        }
                    }
                }
            }
            Stmt::CallIndirect { target } => match self.import_target(*target) {
                Some(name) => {
                    let args = self.indirect_args();
                    if args.is_empty() {
                        format!("{name}()")
                    } else {
                        format!("{name}({args})")
                    }
                }
                None => format!("(*{})()", self.expr(*target)),
            },
            Stmt::Intrinsic { text } => {
                // Unsupported instructions keep their assembly, but must stay inside
                // a string literal so the pseudo-C does not break.
                let sanitized = text.replace('"', "'");
                format!("__asm(\"{sanitized}\")")
            }
        }
    }

    /// Resolve `call [IAT]` to the imported API name. Only a direct load of a constant
    /// address is resolved; register- and index-based calls stay as pointer expressions.
    fn import_target(&self, target: ExprId) -> Option<&str> {
        let Expr::Load { addr, .. } = self.exprs[target as usize] else {
            return None;
        };
        let Expr::Const(value) = self.exprs[addr as usize] else {
            return None;
        };
        self.imports.get(&value).map(String::as_str)
    }

    /// Render a value passed to a call. A stack-slot address becomes `&var_XX`, which is
    /// how the source would pass a local by pointer.
    fn arg_expr(&self, id: ExprId) -> String {
        if let Some((base, offset)) = self.stack_slot(id)
            && let Some(name) = self.stack_reference(base, offset)
        {
            return format!("&{name}");
        }
        self.expr(id)
    }

    /// Render the argument list of an IAT call from the values last written to the ABI
    /// argument registers. Slots that were not written keep their register name so the
    /// position stays truthful; trailing untouched slots are dropped.
    fn indirect_args(&self) -> String {
        // Without the first argument the list would start with a register placeholder;
        // keep the bare `Api()` form in that case.
        if self.arg_values[0].is_none() {
            return String::new();
        }
        let Some(last) = self.arg_values.iter().rposition(Option::is_some) else {
            return String::new();
        };
        (0..=last)
            .map(|slot| match &self.arg_values[slot] {
                Some(value) => value.clone(),
                None => RegView::full(crate::l2::X64_ARG_REGS[slot])
                    .name()
                    .to_owned(),
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn expr(&self, id: ExprId) -> String {
        match &self.exprs[id as usize] {
            Expr::Const(value) => {
                if let Some(literal) = self.literals.get(value) {
                    return literal.clone();
                }
                let signed = *value as i64;
                if (-0x1_0000..0).contains(&signed) {
                    // Sign-extended small negatives (`add eax, -0x30`, `or rax, -1`)
                    // read better as negative literals than as 16 hex digits.
                    return if signed == -1 {
                        "-1".to_owned()
                    } else {
                        format!("-{:#x}", -signed)
                    };
                }
                if *value < 10 {
                    value.to_string()
                } else {
                    format!("{value:#x}")
                }
            }
            Expr::Reg(view) => self.reg_name(view),
            Expr::Temp(id) => format!("t{id}"),
            Expr::Load { addr, size } => {
                if let Some((base, offset)) = self.stack_slot(*addr)
                    && let Some(name) = self.stack_reference(base, offset)
                {
                    name
                } else if let Some(index) = self.array_access(*addr) {
                    index
                } else if let Some(field) = self.field_access(*addr) {
                    field
                } else {
                    format!("*({}*)({})", Self::scalar_type(*size), self.expr(*addr))
                }
            }
            Expr::LoadFloat { addr, size } => {
                if let Some((base, offset)) = self.stack_slot(*addr)
                    && let Some(name) = self.stack_reference(base, offset)
                {
                    name
                } else if let Some(index) = self.array_access(*addr) {
                    index
                } else if let Some(field) = self.field_access(*addr) {
                    field
                } else {
                    format!("*({}*)({})", Self::float_type(*size), self.expr(*addr))
                }
            }
            Expr::Bin { op, lhs, rhs } => {
                let lhs_text = self.expr(*lhs);
                let rhs_text = self.expr(*rhs);
                // Algebraic simplifications that remove the noise emitted by flag setup
                // (`xor eax, eax`, `and ecx, 0`, shifts by zero).
                if lhs_text == rhs_text {
                    match op {
                        BinOp::Xor | BinOp::Sub => return "0".to_owned(),
                        BinOp::And | BinOp::Or => return lhs_text,
                        _ => {}
                    }
                }
                if rhs_text == "0" {
                    match op {
                        BinOp::Add | BinOp::Or | BinOp::Xor => return lhs_text,
                        BinOp::Mul | BinOp::And => return "0".to_owned(),
                        BinOp::Shl | BinOp::Shr | BinOp::Sar | BinOp::Ror | BinOp::Rol => {
                            return lhs_text;
                        }
                        _ => {}
                    }
                }
                if rhs_text == "1" && matches!(op, BinOp::Mul | BinOp::Div) {
                    // `lea eax, [rcx + rdx*1]` and scaled address arithmetic spell a
                    // multiply by one that the source never had.
                    return lhs_text;
                }
                if lhs_text == "1" && matches!(op, BinOp::Mul) {
                    return rhs_text;
                }
                if lhs_text == "0" {
                    match op {
                        BinOp::Add | BinOp::Or | BinOp::Xor => return rhs_text,
                        BinOp::Mul | BinOp::And => return "0".to_owned(),
                        _ => {}
                    }
                }
                match op {
                    // Rotates have no C operator; print them as intrinsic-style calls.
                    BinOp::Ror => format!("ror({lhs_text}, {rhs_text})"),
                    BinOp::Rol => format!("rol({lhs_text}, {rhs_text})"),
                    _ => format!("({lhs_text} {} {rhs_text})", op.symbol()),
                }
            }
            Expr::Un { op, arg } => {
                let symbol = match op {
                    UnOp::Not => "~",
                    UnOp::Neg => "-",
                };
                format!("{symbol}({})", self.expr(*arg))
            }
            Expr::Cast { arg, width, signed } => {
                if let Expr::Const(value) = &self.exprs[*arg as usize]
                    && let Some(literal) = self.literals.get(value)
                {
                    return literal.clone();
                }
                let kind = if *signed { "int" } else { "uint" };
                format!("({kind}{})({})", u16::from(*width) * 8, self.expr(*arg))
            }
            Expr::Cmp { kind, lhs, rhs } => match kind.comparison_symbol() {
                Some(symbol) => {
                    let lhs = self.expr(*lhs);
                    let rhs = self.expr(*rhs);
                    // Signed comparisons need a cast; keep the raw predicate comment
                    // instead of mis-typing the operands.
                    if kind.is_signed() {
                        format!("({lhs} {symbol} {rhs} /* signed */)")
                    } else {
                        format!("({lhs} {symbol} {rhs})")
                    }
                }
                None => format!("/* {} */ 0", cond_name(*kind)),
            },
            Expr::Select {
                cond,
                then,
                otherwise,
            } => format!(
                "({} ? {} : {})",
                self.expr(*cond),
                self.expr(*then),
                self.expr(*otherwise)
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn string_literals_are_c_escaped() {
        assert_eq!(quote_c_string("plain"), "\"plain\"");
        assert_eq!(
            quote_c_string("a\"b\\c\n\t"),
            "\"a\\\"b\\\\c\\n\\t\"",
            "quotes, backslashes and control characters must be escaped"
        );
    }

    fn sample_pe() -> Option<Vec<u8>> {
        for path in [
            "C:/Windows/System32/notepad.exe",
            "C:/Windows/System32/kernel32.dll",
        ] {
            if let Ok(bytes) = std::fs::read(path)
                && !bytes.is_empty()
            {
                return Some(bytes);
            }
        }
        None
    }

    fn session(dir: &TempDir) -> Option<Session> {
        let bytes = sample_pe()?;
        let project = dir.path().join("decompile.rcraft");
        Some(Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap())
    }

    #[test]
    fn import_slots_map_to_sanitised_c_names() {
        let dir = TempDir::new().unwrap();
        let Some(session) = session(&dir) else {
            return;
        };
        let names = import_names(&session);
        assert!(!names.is_empty(), "the sample PE has no named imports");
        assert!(
            names.values().all(|name| {
                !name.is_empty()
                    && name
                        .chars()
                        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
            }),
            "import names must be valid C identifiers: {:?}",
            names.values().take(8).collect::<Vec<_>>()
        );
    }

    #[test]
    fn self_cancelling_setup_values_are_deferred_to_the_call() {
        let decode = |bytes: &[u8]| {
            recraft_decode::decode(bytes, Address::new(0x1000), recraft_decode::Mode::X64)
                .expect("decoded")
        };
        let mut lifter = Lifter::new();
        let xor = lifter.lift(&decode(&[0x33, 0xd2])); // xor edx, edx
        let nop = lifter.lift(&decode(&[0x90])); // nop
        let exprs = lifter.into_exprs();
        let lifted = vec![xor, nop];
        let flat = vec![(0usize, 0usize), (1, 0)];
        let Stmt::WriteReg { value, .. } = &lifted[0].stmts[0] else {
            panic!("xor must write a register");
        };
        let mut live = BTreeSet::new();
        expr_live_regs(&exprs, *value, &mut live);
        assert!(live.is_empty(), "xor edx, edx depends on {live:?}");
        assert!(setup_is_stable(
            &exprs,
            &lifted,
            &flat,
            0,
            1,
            *value,
            Reg::Rdx
        ));

        // A plain copy keeps its source live.
        let mut lifter = Lifter::new();
        let mov = lifter.lift(&decode(&[0x8b, 0xc1])); // mov eax, ecx
        let exprs = lifter.into_exprs();
        let Stmt::WriteReg { value, .. } = &mov.stmts[0] else {
            panic!("mov must write a register");
        };
        let mut live = BTreeSet::new();
        expr_live_regs(&exprs, *value, &mut live);
        assert_eq!(live, BTreeSet::from([Reg::Rcx]));
    }

    #[test]
    fn renders_notepad_entry() {
        let dir = TempDir::new().unwrap();
        let Some(mut session) = session(&dir) else {
            return;
        };
        let entry = session.entry_point();
        let code = pseudocode_l0(&mut session, entry, 64).unwrap();

        assert!(!code.lines.is_empty());
        assert!(code.lines.iter().all(|line| line.addr.get() >= entry.get()));
        let text = code.text();
        assert!(
            text.contains('=') || text.contains("();"),
            "expected assignments or calls:\n{text}"
        );
        assert!(
            text.contains("return") || text.contains("goto"),
            "expected control flow:\n{text}"
        );
        // Stack-pointer bookkeeping is suppressed now; the epilogue tail call becomes a
        // real return call instead of a comment.
        assert!(
            !text.contains("rsp = "),
            "stack-pointer noise should be suppressed:\n{text}"
        );
        assert!(
            text.contains("return sub_") || text.contains("goto"),
            "expected a rendered tail call:\n{text}"
        );
    }

    /// Guard the visible-noise cleanup against regressions in golden snapshots.
    #[test]
    fn golden_snapshots_have_no_known_noise() {
        let dir = default_golden_dir();
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return;
        };
        let mut checked = 0usize;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("c") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            checked += 1;
            assert!(
                !text.contains("rsp = "),
                "rsp bookkeeping leaked into {}",
                path.display()
            );
            assert!(
                !text.contains("tail call ->"),
                "old tail-call comment leaked into {}",
                path.display()
            );
            assert!(
                !text.contains("(uint32*)(") && !text.contains("(uint64*)("),
                "value cast rendered as pointer cast in {}",
                path.display()
            );
            for line in text.lines() {
                if let Some(index) = line.find(" ^ ") {
                    let lhs = line[..index].split_whitespace().last();
                    let rhs = line[index + 3..]
                        .split(|character: char| {
                            !character.is_ascii_alphanumeric() && character != '_'
                        })
                        .next();
                    assert!(
                        lhs != rhs || lhs.is_none(),
                        "x ^ x not simplified in {}: {line}",
                        path.display()
                    );
                }
            }
        }
        assert!(
            checked > 0,
            "no golden snapshots found in {}",
            dir.display()
        );
    }

    #[test]
    fn output_is_deterministic() {
        let dir = TempDir::new().unwrap();
        let Some(mut session) = session(&dir) else {
            return;
        };
        let entry = session.entry_point();
        let first = pseudocode_l0(&mut session, entry, 32).unwrap().text();
        let second = pseudocode_l0(&mut session, entry, 32).unwrap().text();
        assert_eq!(first, second);
    }
}
