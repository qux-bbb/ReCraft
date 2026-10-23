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
use recraft_ir::{BinOp, Expr, ExprId, Flow, LiftedInsn, Lifter, Reg, Stmt, UnOp, cond_name};

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
) -> Result<BlockLift> {
    if end <= start {
        return Ok(BlockLift {
            statements: Vec::new(),
            condition: None,
            flow: Flow::Fallthrough,
            written: BTreeSet::new(),
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
        });
    }

    let exprs = lifter.into_exprs();
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
    let mut printer = Printer {
        exprs: &exprs,
        names: &names,
        start,
        end,
        labels: &BTreeSet::new(),
        args,
        arg_structs,
        call_protos: &call_protos,
        written: seed
            .map(|seed| seed.iter().copied().collect())
            .unwrap_or_default(),
    };

    let mut statements = Vec::new();
    let mut written = BTreeSet::new();
    for item in &lifted {
        for stmt in &item.stmts {
            if let Stmt::WriteReg { reg, .. } = stmt {
                written.insert(reg.base);
            }
            if printer.is_noop_write(stmt) {
                continue;
            }
            let text = printer.stmt(stmt);
            statements.push(PseudoLine {
                addr: item.addr,
                text: format!("{text};"),
            });
            printer.note_write(stmt);
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
    })
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
    let mut names: HashMap<u64, String> = HashMap::new();
    for target in targets.iter().chain(calls.iter()) {
        let target = Address::new(*target);
        names.insert(target.get(), resolve_name(session, target)?);
    }

    let no_structs: BTreeMap<Reg, StructDef> = BTreeMap::new();
    let no_protos: HashMap<u64, crate::l2::ProtoSig> = HashMap::new();
    let mut printer = Printer {
        exprs: &exprs,
        names: &names,
        start,
        end,
        labels: &targets,
        args: None,
        arg_structs: &no_structs,
        call_protos: &no_protos,
        written: HashSet::new(),
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
    /// Registers written so far (for argument-register naming).
    written: HashSet<Reg>,
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
            for stmt in &insn.stmts {
                if self.is_noop_write(stmt) {
                    continue;
                }
                let text = self.stmt(stmt);
                lines.push(PseudoLine {
                    addr: insn.addr,
                    text: format!("    {text};"),
                });
                self.note_write(stmt);
            }
            match insn.flow {
                Flow::Fallthrough => {}
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

    /// Records a register write so later reads no longer render as `argN`.
    fn note_write(&mut self, stmt: &Stmt) {
        if let Stmt::WriteReg { reg, .. } = stmt {
            self.written.insert(reg.base);
        }
    }

    fn is_noop_write(&self, stmt: &Stmt) -> bool {
        let Stmt::WriteReg { reg, value } = stmt else {
            return false;
        };
        matches!(self.exprs[*value as usize], Expr::Reg(view) if view == *reg)
    }

    fn is_local(&self, target: Address) -> bool {
        self.start.get() <= target.get() && target.get() < self.end.get()
    }

    fn name(&self, target: Address) -> String {
        self.names
            .get(&target.get())
            .cloned()
            .unwrap_or_else(|| format!("sub_{:x}", target.get()))
    }

    /// Register reads render as `argN` while the register still holds its incoming
    /// argument value.
    fn reg_name(&self, view: &recraft_ir::RegView) -> String {
        if let Some(args) = self.args
            && let Some(name) = args.name_for(view.base)
            && !self.written.contains(&view.base)
        {
            return name.to_owned();
        }
        view.name().to_owned()
    }

    /// `[rsp + const]`, `[rbp - const]`, `[ebp + const]` -> (base register, offset).
    fn stack_slot(&self, expr: ExprId) -> Option<(Reg, i64)> {
        let (op, lhs, rhs) = match &self.exprs[expr as usize] {
            Expr::Bin { op, lhs, rhs } => (op, *lhs, *rhs),
            _ => return None,
        };
        if *op != recraft_ir::BinOp::Add {
            return None;
        }
        let base_of = |id: ExprId, exprs: &[Expr]| match &exprs[id as usize] {
            Expr::Reg(view) if matches!(view.base, Reg::Rsp | Reg::Rbp) => Some(view.base),
            _ => None,
        };
        let const_of = |id: ExprId, exprs: &[Expr]| match exprs[id as usize] {
            Expr::Const(value) => Some(value as i64),
            _ => None,
        };
        if let Some(base) = base_of(lhs, self.exprs) {
            const_of(rhs, self.exprs).map(|offset| (base, offset))
        } else if let Some(base) = base_of(rhs, self.exprs) {
            const_of(lhs, self.exprs).map(|offset| (base, offset))
        } else {
            None
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
    /// rsp slots and frame-pointer locals. Positive `[rbp+...]` without a stack-argument
    /// match stays a raw pointer (it may be the return address / caller frame).
    fn stack_reference(&self, base: Reg, offset: i64) -> Option<String> {
        if let Some(name) = self.args.and_then(|args| args.name_for_stack(base, offset)) {
            return Some(name.to_owned());
        }
        if base == Reg::Rsp || offset < 0 {
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

    fn stmt(&self, stmt: &Stmt) -> String {
        match stmt {
            Stmt::WriteReg { reg, value } => {
                // Writes always use the concrete register view; argument names only
                // re-label reads of the still-intact incoming value.
                format!("{} = {}", reg.name(), self.expr(*value))
            }
            Stmt::Store { addr, value, size } => {
                // Writes never use incoming argument names: storing to a caller's
                // argument slot is rare and would read as reassigning a variable.
                if let Some((base, offset)) = self.stack_slot(*addr)
                    && (base == Reg::Rsp || offset < 0)
                {
                    format!("{} = {}", Self::stack_name(offset), self.expr(*value))
                } else if let Some(index) = self.array_access(*addr) {
                    format!("{index} = {}", self.expr(*value))
                } else if let Some(field) = self.field_access(*addr) {
                    format!("{field} = {}", self.expr(*value))
                } else {
                    format!(
                        "*({}*)({}) = {}",
                        Self::scalar_type(*size),
                        self.expr(*addr),
                        self.expr(*value)
                    )
                }
            }
            Stmt::StoreFloat { addr, value, size } => {
                if let Some((base, offset)) = self.stack_slot(*addr)
                    && (base == Reg::Rsp || offset < 0)
                {
                    format!("{} = {}", Self::stack_name(offset), self.expr(*value))
                } else if let Some(index) = self.array_access(*addr) {
                    format!("{index} = {}", self.expr(*value))
                } else if let Some(field) = self.field_access(*addr) {
                    format!("{field} = {}", self.expr(*value))
                } else {
                    format!(
                        "*({}*)({}) = {}",
                        Self::float_type(*size),
                        self.expr(*addr),
                        self.expr(*value)
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
                                let value = self.expr(*arg);
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
                    None => format!("{name}()"),
                }
            }
            Stmt::Intrinsic { text } => {
                // Unsupported instructions keep their assembly, but must stay inside
                // a string literal so the pseudo-C does not break.
                let sanitized = text.replace('"', "'");
                format!("__asm(\"{sanitized}\")")
            }
        }
    }

    fn expr(&self, id: ExprId) -> String {
        match &self.exprs[id as usize] {
            Expr::Const(value) => {
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
                if let Some(index) = self.array_access(*addr) {
                    index
                } else if let Some(field) = self.field_access(*addr) {
                    field
                } else {
                    format!("*({}*)({})", Self::float_type(*size), self.expr(*addr))
                }
            }
            Expr::Bin { op, lhs, rhs } => {
                format!("({} {} {})", self.expr(*lhs), op.symbol(), self.expr(*rhs))
            }
            Expr::Un { op, arg } => {
                let symbol = match op {
                    UnOp::Not => "~",
                    UnOp::Neg => "-",
                };
                format!("{symbol}({})", self.expr(*arg))
            }
            Expr::Cast { arg, width, signed } => {
                let kind = if *signed { "int" } else { "uint" };
                format!("({kind}{}*)({})", u16::from(*width) * 8, self.expr(*arg))
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
        assert!(text.contains('='), "expected assignments:\n{text}");
        assert!(
            text.contains("return") || text.contains("goto"),
            "expected control flow:\n{text}"
        );
        assert!(text.contains("rsp"), "expected stack handling:\n{text}");
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
