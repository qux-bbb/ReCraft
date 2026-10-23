// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Decompiler front-end.
//!
//! M0 ships **L0**: a flat, instruction-faithful pseudo-C rendering of one function's
//! straight-line code and branches. It is deliberately simple — the point is to prove
//! the IR → printer → UI/CLI pipeline. L1 (SSA, structuring, variables, types) will
//! replace the printer, not the IR.
//!
//! Every output line keeps its source address so the UI can cross-highlight and the
//! AI layer can cite evidence.

#![forbid(unsafe_code)]

use std::collections::{BTreeSet, HashMap};

use recraft_common::{Address, Error, Result};
use recraft_core::Session;
use recraft_ir::{Expr, ExprId, Flow, LiftedInsn, Lifter, Stmt, UnOp, cond_name};

mod structuring;

pub use structuring::pseudocode_l1;

/// One block's lifted statements plus its terminating flow (used by L1).
pub(crate) struct BlockLift {
    /// Statements without the trailing semicolon or indentation.
    pub statements: Vec<PseudoLine>,
    /// Condition text for a conditional branch terminating the block.
    pub condition: Option<String>,
    pub flow: Flow,
}

/// Lift one basic block (`[start, end)`) into statements + terminating flow.
pub(crate) fn lift_block(session: &mut Session, start: Address, end: Address) -> Result<BlockLift> {
    if end <= start {
        return Ok(BlockLift {
            statements: Vec::new(),
            condition: None,
            flow: Flow::Fallthrough,
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
        });
    }

    let exprs = lifter.into_exprs();
    let mut names: HashMap<u64, String> = HashMap::new();
    for item in &lifted {
        for stmt in &item.stmts {
            if let Stmt::Call { target } = stmt {
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
    let printer = Printer {
        exprs: &exprs,
        names: &names,
        start,
        end,
        labels: &BTreeSet::new(),
    };

    let mut statements = Vec::new();
    for item in &lifted {
        for stmt in &item.stmts {
            if printer.is_noop_write(stmt) {
                continue;
            }
            statements.push(PseudoLine {
                addr: item.addr,
                text: format!("{};", printer.stmt(stmt)),
            });
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
            if let Stmt::Call { target } = stmt {
                calls.insert(target.get());
            }
        }
        match item.flow {
            Flow::Goto(target) | Flow::CondGoto { target, .. } => {
                targets.insert(target.get());
            }
            _ => {}
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
        return Err(Error::internal(format!("nothing to decompile at {start}")));
    }

    let exprs = lifter.into_exprs();
    let mut names: HashMap<u64, String> = HashMap::new();
    for target in targets.iter().chain(calls.iter()) {
        let target = Address::new(*target);
        names.insert(target.get(), resolve_name(session, target)?);
    }

    let printer = Printer {
        exprs: &exprs,
        names: &names,
        start,
        end,
        labels: &targets,
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
}

impl Printer<'_> {
    fn render(self, lifted: &[LiftedInsn]) -> Vec<PseudoLine> {
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
                lines.push(PseudoLine {
                    addr: insn.addr,
                    text: format!("    {};", self.stmt(stmt)),
                });
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

    fn is_noop_write(&self, stmt: &Stmt) -> bool {
        let Stmt::WriteReg { reg, value } = stmt else {
            return false;
        };
        match self.exprs[*value as usize] {
            Expr::Reg(view) => view == *reg,
            _ => false,
        }
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

    fn stmt(&self, stmt: &Stmt) -> String {
        match stmt {
            Stmt::WriteReg { reg, value } => format!("{} = {}", reg.name(), self.expr(*value)),
            Stmt::Store { addr, value, size } => {
                format!(
                    "*(u{}*)({}) = {}",
                    u16::from(*size) * 8,
                    self.expr(*addr),
                    self.expr(*value)
                )
            }
            Stmt::Call { target } => format!("{}()", self.name(*target)),
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
            Expr::Reg(view) => view.name().to_owned(),
            Expr::Temp(id) => format!("t{id}"),
            Expr::Load { addr, size } => {
                format!("*(u{}*)({})", u16::from(*size) * 8, self.expr(*addr))
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
                    // Signed comparisons need a cast; M0 keeps the raw predicate
                    // comment instead of mis-typing the operands.
                    if kind.is_signed() {
                        format!("({lhs} {symbol} {rhs} /* signed */)")
                    } else {
                        format!("({lhs} {symbol} {rhs})")
                    }
                }
                None => format!("/* {} */ 0", cond_name(*kind)),
            },
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
