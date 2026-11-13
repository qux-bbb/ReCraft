// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! AArch64 → IR lifter (W5).
//!
//! The first slice covers the integer core that Go and C startup code are made of:
//! `mov`, `add`/`sub`/`and`/`orr`/`eor`, `ldr`/`str` with a base and a constant offset,
//! `adr`/`adrp`, `movz`/`movk`/`movn`, `cmp`, and the branch forms (`b`, `b.<cond>`,
//! `cbz`/`cbnz`, `bl`, `ret`). Anything else becomes [`Stmt::Intrinsic`] with the original
//! assembly text, and the value environment is cleared because the instruction's effects
//! are unknown.
//!
//! The structurer is still x86-shaped, so this lifter produces IR only; turning it into
//! pseudo-C needs the neutral CFG work that follows.

use std::collections::BTreeMap;

use recraft_common::Address;
use recraft_decode::{DecodedInsn, DecodedOperand};

use crate::expr::{BinOp, CondKind, Expr, ExprId, Flow, Lifted, LiftedInsn, Stmt};
use crate::reg::{Reg, RegView};

/// Lifts AArch64 instructions into the shared IR arena.
#[derive(Debug, Default)]
pub struct A64Lifter {
    exprs: Vec<Expr>,
    /// Latest value written to each register.
    env: BTreeMap<Reg, ExprId>,
    /// Value of the last `cmp`/`cmn`/`tst`, for the next conditional branch.
    cond: Option<ExprId>,
}

impl A64Lifter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Lift a straight-line region. Returns the lifted region and how many instructions
    /// produced IR (the rest are `Intrinsic`).
    pub fn lift_region(&mut self, insns: &[DecodedInsn]) -> (Lifted, usize) {
        let start = insns.first().map_or(Address::new(0), |insn| insn.addr);
        let mut lifted = Lifted {
            start,
            exprs: Vec::new(),
            insns: Vec::new(),
        };
        let mut ok = 0usize;
        for insn in insns {
            let mut stmts = Vec::new();
            let flow = self.lift_one(insn, &mut stmts);
            if flow.is_some() {
                ok += 1;
            } else {
                // Unknown effects: drop every cached value, keep the assembly text.
                self.env.clear();
                self.cond = None;
                stmts.push(Stmt::Intrinsic {
                    text: format!("{} {}", insn.mnemonic, insn.operand_text())
                        .trim()
                        .to_owned(),
                });
            }
            lifted.insns.push(LiftedInsn {
                addr: insn.addr,
                stmts,
                flow: flow.unwrap_or(Flow::Fallthrough),
            });
        }
        lifted.exprs = std::mem::take(&mut self.exprs);
        (lifted, ok)
    }

    fn lift_one(&mut self, insn: &DecodedInsn, stmts: &mut Vec<Stmt>) -> Option<Flow> {
        let ops = &insn.operands;
        match insn.mnemonic.as_str() {
            "mov" => {
                let dest = Self::view(ops.first()?)?;
                let value = self.read(ops.get(1)?)?;
                self.write(dest, value, stmts);
                Some(Flow::Fallthrough)
            }
            "movz" | "movn" => {
                let dest = Self::view(ops.first()?)?;
                let mut value = self.imm(ops.get(1)?)?;
                if insn.mnemonic == "movn" {
                    let not = self.push(Expr::Un {
                        op: crate::expr::UnOp::Not,
                        arg: value,
                    });
                    value = not;
                }
                self.write(dest, value, stmts);
                Some(Flow::Fallthrough)
            }
            "movk" => {
                // `movk` replaces one 16-bit lane; model it as an or of the shifted halfword.
                let dest = Self::view(ops.first()?)?;
                let imm = self.imm(ops.get(1)?)?;
                let shift = self.constant(16);
                let shifted = self.push(Expr::Bin {
                    op: BinOp::Shl,
                    lhs: imm,
                    rhs: shift,
                });
                let previous = self.reg(dest);
                let value = self.push(Expr::Bin {
                    op: BinOp::Or,
                    lhs: previous,
                    rhs: shifted,
                });
                self.write(dest, value, stmts);
                Some(Flow::Fallthrough)
            }
            "add" | "sub" | "and" | "orr" | "eor" => {
                let dest = Self::view(ops.first()?)?;
                let lhs = self.read(ops.get(1)?)?;
                let rhs = self.read(ops.get(2)?)?;
                let op = match insn.mnemonic.as_str() {
                    "add" => BinOp::Add,
                    "sub" => BinOp::Sub,
                    "and" => BinOp::And,
                    "orr" => BinOp::Or,
                    _ => BinOp::Xor,
                };
                let value = self.push(Expr::Bin { op, lhs, rhs });
                self.write(dest, value, stmts);
                Some(Flow::Fallthrough)
            }
            "ldr" | "ldur" => {
                let dest = Self::view(ops.first()?)?;
                let addr = self.address(ops.get(1)?)?;
                let value = self.push(Expr::Load { addr, size: 8 });
                self.write(dest, value, stmts);
                Some(Flow::Fallthrough)
            }
            "str" | "stur" => {
                let value = self.read(ops.first()?)?;
                let addr = self.address(ops.get(1)?)?;
                stmts.push(Stmt::Store {
                    addr,
                    value,
                    size: 8,
                });
                Some(Flow::Fallthrough)
            }
            "cmp" | "cmn" | "tst" => {
                let lhs = self.read(ops.first()?)?;
                let rhs = self.read(ops.get(1)?)?;
                let op = if insn.mnemonic == "cmn" {
                    BinOp::Add
                } else if insn.mnemonic == "tst" {
                    BinOp::And
                } else {
                    BinOp::Sub
                };
                let value = self.push(Expr::Bin { op, lhs, rhs });
                self.cond = Some(value);
                Some(Flow::Fallthrough)
            }
            "adr" | "adrp" => {
                let dest = Self::view(ops.first()?)?;
                let value = self.imm(ops.get(1)?)?;
                self.write(dest, value, stmts);
                Some(Flow::Fallthrough)
            }
            "b" => Some(Flow::Goto(insn.branch_target?)),
            "ret" => Some(Flow::Return),
            "bl" => {
                let target = insn.branch_target?;
                let args = self.call_args();
                stmts.push(Stmt::Call { target, args });
                Some(Flow::Fallthrough)
            }
            "cbz" | "cbnz" => {
                let value = self.read(ops.first()?)?;
                let zero = self.constant(0);
                let kind = if insn.mnemonic == "cbz" {
                    CondKind::Eq
                } else {
                    CondKind::Ne
                };
                let cond = self.push(Expr::Cmp {
                    kind,
                    lhs: value,
                    rhs: zero,
                });
                Some(Flow::CondGoto {
                    cond,
                    target: insn.branch_target?,
                })
            }
            _ if insn.mnemonic.starts_with("b.") => {
                let cond = self.cond?;
                let zero = self.constant(0);
                let kind = condition_kind(&insn.mnemonic[2..])?;
                let cmp = self.push(Expr::Cmp {
                    kind,
                    lhs: cond,
                    rhs: zero,
                });
                Some(Flow::CondGoto {
                    cond: cmp,
                    target: insn.branch_target?,
                })
            }
            _ => None,
        }
    }

    /// A64 integer-argument registers `x0`–`x7`, as far as they hold known values.
    fn call_args(&mut self) -> Vec<ExprId> {
        (0..8u16)
            .map(|number| {
                let view = Reg::aarch64(number, 8, false);
                self.reg(view)
            })
            .collect()
    }

    // ---- operand plumbing -------------------------------------------------------

    /// Parse a register operand name (`x5`, `w3`, `sp`, `xzr`) into a view.
    fn view(operand: &DecodedOperand) -> Option<RegView> {
        let DecodedOperand::Register { name, size } = operand else {
            return None;
        };
        let width = if *size == 4 { 4 } else { 8 };
        Some(match name.as_str() {
            "sp" | "wsp" | "xsp" => Reg::aarch64(31, width, true),
            "xzr" | "wzr" => Reg::aarch64(31, width, false),
            other => {
                let digits: String = other.chars().filter(|c| c.is_ascii_digit()).collect();
                let number: u16 = digits.parse().ok()?;
                Reg::aarch64(number, width, false)
            }
        })
    }

    fn reg(&mut self, view: RegView) -> ExprId {
        if let Some(value) = self.env.get(&view.base) {
            return *value;
        }
        self.push(Expr::Reg(view))
    }

    /// Read an operand: a register value, an immediate or a memory address.
    fn read(&mut self, operand: &DecodedOperand) -> Option<ExprId> {
        match operand {
            DecodedOperand::Register { .. } => {
                let view = Self::view(operand)?;
                Some(self.reg(view))
            }
            DecodedOperand::Immediate(value) => Some(self.constant(*value as u64)),
            _ => None,
        }
    }

    fn imm(&mut self, operand: &DecodedOperand) -> Option<ExprId> {
        match operand {
            DecodedOperand::Immediate(value) => Some(self.constant(*value as u64)),
            _ => None,
        }
    }

    fn address(&mut self, operand: &DecodedOperand) -> Option<ExprId> {
        let DecodedOperand::Memory { base, disp, .. } = operand else {
            return None;
        };
        let base = base.as_ref()?;
        let view = Self::view(&DecodedOperand::Register {
            name: base.clone(),
            size: 8,
        })?;
        let base = self.reg(view);
        if *disp == 0 {
            return Some(base);
        }
        let offset = self.constant(*disp as u64);
        Some(self.push(Expr::Bin {
            op: BinOp::Add,
            lhs: base,
            rhs: offset,
        }))
    }

    fn write(&mut self, view: RegView, value: ExprId, stmts: &mut Vec<Stmt>) {
        stmts.push(Stmt::WriteReg { reg: view, value });
        if view.base != Reg::A64Zr {
            if view.width == 8 {
                self.env.insert(view.base, value);
            } else {
                self.env.remove(&view.base);
            }
        }
    }

    fn push(&mut self, expr: Expr) -> ExprId {
        self.exprs.push(expr);
        (self.exprs.len() - 1) as ExprId
    }

    fn constant(&mut self, value: u64) -> ExprId {
        self.push(Expr::Const(value))
    }
}

/// Map an A64 condition suffix to the comparison kind it tests.
fn condition_kind(suffix: &str) -> Option<CondKind> {
    Some(match suffix {
        "eq" => CondKind::Eq,
        "ne" => CondKind::Ne,
        "hi" => CondKind::UnsignedGt,
        "hs" => CondKind::UnsignedGe,
        "lo" => CondKind::UnsignedLt,
        "ls" => CondKind::UnsignedLe,
        "gt" => CondKind::SignedGt,
        "ge" => CondKind::SignedGe,
        "lt" => CondKind::SignedLt,
        "le" => CondKind::SignedLe,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use recraft_decode::decode_aarch64;

    fn decode(words: &[u32]) -> Vec<DecodedInsn> {
        words
            .iter()
            .enumerate()
            .filter_map(|(index, word)| {
                let addr = Address::new(0x1000 + (index as u64) * 4);
                decode_aarch64(&word.to_le_bytes(), addr)
            })
            .collect()
    }

    fn lift(words: &[u32]) -> (Lifted, usize) {
        A64Lifter::new().lift_region(&decode(words))
    }

    #[test]
    fn probe_encodings() {
        for word in [
            0x9100_0402u32,
            0xf940_0b90,
            0xf814_0ffe,
            0xf100_805f,
            0x5400_00c8,
            0x9400_0008,
            0x1400_0000,
        ] {
            let insn =
                recraft_decode::decode_aarch64(&word.to_le_bytes(), Address::new(0x1000)).unwrap();
            println!(
                "{word:08x} -> {} | {:?} | {:#x?}",
                insn.mnemonic, insn.operands, insn.branch_target
            );
        }
    }
    #[test]
    fn add_and_ldr_write_a_register() {
        // add x2, x0, #1 ; ldr x16, [x28, #0x10]
        let (lifted, ok) = lift(&[0x9100_0402, 0xf940_0b90]);
        assert_eq!(ok, 2);
        assert!(matches!(
            &lifted.insns[0].stmts[0],
            Stmt::WriteReg { reg, .. } if reg.name() == "x2"
        ));
        let Stmt::WriteReg { value, .. } = &lifted.insns[1].stmts[0] else {
            panic!("expected a register write");
        };
        assert!(matches!(lifted.expr(*value), Expr::Load { size: 8, .. }));
    }

    #[test]
    fn a_store_keeps_an_address_and_a_size() {
        // str x30, [sp, #-192]
        let (lifted, ok) = lift(&[0xf814_0ffe]);
        assert_eq!(ok, 1);
        assert!(matches!(
            &lifted.insns[0].stmts[0],
            Stmt::Store { size: 8, .. }
        ));
    }

    #[test]
    fn a_compare_feeds_the_next_branch() {
        // cmp x2, #0x20 ; b.hi +0x18
        let (lifted, ok) = lift(&[0xf100_805f, 0x5400_00c8]);
        assert_eq!(ok, 2);
        assert!(lifted.insns[0].stmts.is_empty(), "cmp emits no statement");
        match lifted.insns[1].flow {
            Flow::CondGoto { target, .. } => assert_eq!(target, Address::new(0x101c)),
            other => panic!("expected a conditional branch, got {other:?}"),
        }
    }

    #[test]
    fn a_bl_is_a_call_and_a_b_is_a_goto() {
        // bl 0x1020 ; b 0x1000
        let (lifted, ok) = lift(&[0x9400_0008, 0x1400_0000]);
        assert_eq!(ok, 2);
        assert!(matches!(
            &lifted.insns[0].stmts[0],
            Stmt::Call { target, .. } if *target == Address::new(0x1020)
        ));
        assert_eq!(lifted.insns[1].flow, Flow::Goto(Address::new(0x1004)));
    }

    #[test]
    fn a_return_ends_the_region() {
        let (lifted, ok) = lift(&[0xd65f_03c0]);
        assert_eq!(ok, 1);
        assert_eq!(lifted.insns[0].flow, Flow::Return);
    }

    #[test]
    fn an_unmodelled_instruction_becomes_an_intrinsic() {
        // udf #0
        let (lifted, ok) = lift(&[0x0000_0000]);
        assert_eq!(ok, 0);
        assert!(matches!(&lifted.insns[0].stmts[0], Stmt::Intrinsic { .. }));
    }
}
