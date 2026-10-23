// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! L2 variable recovery (first cut).
//!
//! Recovers which incoming argument registers are actually used before being written
//! (Windows x64 fastcall: rcx, rdx, r8, r9) and exposes them as `arg1..arg4`. Stack
//! slots (`[rsp + const]`) are named `var_XX` by the printer. Types, SSA and struct
//! recovery come later.

use std::collections::{BTreeMap, BTreeSet};

use recraft_common::{Address, Result};
use recraft_core::Session;
use recraft_ir::{Expr, ExprId, Lifter, Reg, RegView, Stmt};

/// Windows x64 integer argument registers, in declaration order.
pub const X64_ARG_REGS: [Reg; 4] = [Reg::Rcx, Reg::Rdx, Reg::R8, Reg::R9];

const MAX_ARG_SCAN_INSNS: usize = 4096;

/// Recovered argument names for one function.
#[derive(Debug, Clone, Default)]
pub struct ArgNames {
    names: BTreeMap<Reg, String>,
}

impl ArgNames {
    pub fn name_for(&self, reg: Reg) -> Option<&str> {
        self.names.get(&reg).map(String::as_str)
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// `arg1, arg2, ...` in calling-convention order.
    pub fn ordered(&self) -> Vec<(&'static str, &str)> {
        X64_ARG_REGS
            .iter()
            .filter_map(|reg| {
                self.names
                    .get(reg)
                    .map(|name| (reg.name64(), name.as_str()))
            })
            .collect()
    }

    /// C parameter list (`int64 arg1, int64 arg2`) or `void`.
    pub fn signature(&self) -> String {
        let params: Vec<String> = self
            .ordered()
            .iter()
            .map(|(_, name)| format!("int64 {name}"))
            .collect();
        if params.is_empty() {
            "void".to_owned()
        } else {
            params.join(", ")
        }
    }

    /// `arg1=rcx, arg2=rdx` for a leading comment.
    pub fn comment(&self) -> String {
        self.ordered()
            .iter()
            .map(|(reg, name)| format!("{name}={reg}"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// Find argument registers that are read before their first write.
pub fn recover_args(session: &mut Session, start: Address, end: Address) -> Result<ArgNames> {
    let mut lifter = Lifter::new();
    let mut lifted = Vec::new();
    let mut addr = start;
    while addr < end && lifted.len() < MAX_ARG_SCAN_INSNS {
        let Some(insn) = session.insn_at(addr)? else {
            break;
        };
        if insn.is_invalid() {
            break;
        }
        let item = lifter.lift(&insn);
        let next = insn.next_addr();
        lifted.push(item);
        if next <= addr || next >= end {
            break;
        }
        addr = next;
    }
    let exprs = lifter.into_exprs();
    let arg_registers: BTreeSet<Reg> = X64_ARG_REGS.into_iter().collect();
    let mut written: BTreeSet<Reg> = BTreeSet::new();
    let mut incoming: BTreeSet<Reg> = BTreeSet::new();

    for item in &lifted {
        for stmt in &item.stmts {
            walk_stmt_reads(stmt, &exprs, &mut |view: &RegView| {
                if arg_registers.contains(&view.base) && !written.contains(&view.base) {
                    incoming.insert(view.base);
                }
            });
            if let Stmt::WriteReg { reg, .. } = stmt {
                written.insert(reg.base);
            }
        }
    }

    let mut names = BTreeMap::new();
    for (index, reg) in X64_ARG_REGS.iter().enumerate() {
        if incoming.contains(reg) {
            names.insert(*reg, format!("arg{}", index + 1));
        }
    }
    Ok(ArgNames { names })
}

fn walk_stmt_reads(stmt: &Stmt, exprs: &[Expr], visit: &mut impl FnMut(&RegView)) {
    match stmt {
        Stmt::WriteReg { value, .. } => walk_expr(exprs, *value, visit),
        Stmt::Store { addr, value, .. } => {
            walk_expr(exprs, *addr, visit);
            walk_expr(exprs, *value, visit);
        }
        Stmt::Call { .. } | Stmt::Intrinsic { .. } => {}
    }
}

fn walk_expr(exprs: &[Expr], id: ExprId, visit: &mut impl FnMut(&RegView)) {
    match &exprs[id as usize] {
        Expr::Reg(view) => visit(view),
        Expr::Load { addr, .. } => walk_expr(exprs, *addr, visit),
        Expr::Bin { lhs, rhs, .. } | Expr::Cmp { lhs, rhs, .. } => {
            walk_expr(exprs, *lhs, visit);
            walk_expr(exprs, *rhs, visit);
        }
        Expr::Un { arg, .. } | Expr::Cast { arg, .. } => walk_expr(exprs, *arg, visit),
        Expr::Const(_) | Expr::Temp(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn corpus_session() -> Option<(Session, tempfile::TempDir)> {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../corpus/build/x64/corpus.exe");
        let bytes = std::fs::read(path).ok()?;
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("l2.rcraft");
        let mut session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let result = recraft_analysis::analyze(&mut session).unwrap();
        recraft_analysis::persist(&mut session, &result).unwrap();
        Some((session, dir))
    }

    fn bounds(session: &Session, name: &str) -> (Address, Address) {
        let start = session.addr_of_name(name).unwrap().unwrap();
        let function = session
            .functions()
            .unwrap()
            .into_iter()
            .find(|function| function.start == start)
            .unwrap_or_else(|| panic!("{name} function missing"));
        (function.start, function.end)
    }

    #[test]
    fn add_leaf_uses_two_argument_registers() {
        let Some((mut session, _dir)) = corpus_session() else {
            return;
        };
        let (start, end) = bounds(&session, "add_leaf");
        let args = recover_args(&mut session, start, end).unwrap();
        let ordered = args.ordered();
        assert_eq!(ordered.len(), 2, "{ordered:?}");
        assert_eq!(ordered[0], ("rcx", "arg1"));
        assert_eq!(ordered[1], ("rdx", "arg2"));
    }

    #[test]
    fn written_registers_are_not_arguments() {
        let Some((mut session, _dir)) = corpus_session() else {
            return;
        };
        // branchy(x) reads rcx only.
        let (start, end) = bounds(&session, "branchy");
        let args = recover_args(&mut session, start, end).unwrap();
        let ordered = args.ordered();
        assert_eq!(ordered.len(), 1, "{ordered:?}");
        assert_eq!(ordered[0].1, "arg1");
    }
}
