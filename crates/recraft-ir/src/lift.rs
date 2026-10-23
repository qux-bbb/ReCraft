// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! x86/x64 → LIR lifting.
//!
//! Supported in M0: mov/movzx/movsx, lea, add/sub/and/or/xor, inc/dec, neg/not,
//! shifts, cmp/test, direct call, jmp/jcc, ret, nop. Everything else becomes an
//! [`Stmt::Intrinsic`] that keeps the original assembly text, so lifting never fails.
//!
//! Flags are *lazy*: only the comparison feeding the next `jcc` is tracked, which
//! covers the overwhelming majority of real code (`cmp/test` immediately before a
//! branch). Other flag consumers fall back to an intrinsic.

use recraft_decode::{Insn, iced_x86};

use crate::expr::{BinOp, CondKind, Expr, ExprId, Flow, LiftedInsn, Stmt, UnOp};
use crate::reg::{RegView, from_iced};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CmpSource {
    /// `cmp` / `sub`: full arithmetic flags are available.
    Arithmetic,
    /// `test` / bitwise / inc / dec: only zero/sign meanings are trustworthy.
    ZeroOnly,
}

#[derive(Debug, Clone, Copy)]
struct CmpState {
    lhs: ExprId,
    rhs: ExprId,
    source: CmpSource,
}

/// Lifts instructions into [`LiftedInsn`]s, owning the expression arena.
pub struct Lifter {
    exprs: Vec<Expr>,
    last_cmp: Option<CmpState>,
}

impl Default for Lifter {
    fn default() -> Self {
        Self::new()
    }
}

impl Lifter {
    pub fn new() -> Self {
        Self {
            exprs: Vec::new(),
            last_cmp: None,
        }
    }

    pub fn exprs(&self) -> &[Expr] {
        &self.exprs
    }

    #[inline]
    pub fn expr(&self, id: ExprId) -> &Expr {
        &self.exprs[id as usize]
    }

    pub fn into_exprs(self) -> Vec<Expr> {
        self.exprs
    }

    fn push(&mut self, expr: Expr) -> ExprId {
        self.exprs.push(expr);
        (self.exprs.len() - 1) as ExprId
    }

    fn constant(&mut self, value: u64) -> ExprId {
        self.push(Expr::Const(value))
    }

    fn reg(&mut self, view: RegView) -> ExprId {
        self.push(Expr::Reg(view))
    }

    /// Lift one instruction.
    pub fn lift(&mut self, insn: &Insn) -> LiftedInsn {
        use iced_x86::Mnemonic as M;
        let mut stmts = Vec::new();
        let addr = insn.addr();
        let flow = match insn.mnemonic() {
            M::Mov => self.lift_mov(insn, &mut stmts),
            M::Movzx => self.lift_movx(insn, &mut stmts, false),
            M::Movsx | M::Movsxd => self.lift_movx(insn, &mut stmts, true),
            M::Lea => self.lift_lea(insn, &mut stmts),
            M::Add | M::Sub | M::And | M::Or | M::Xor => {
                self.lift_binary(insn, &mut stmts);
                Flow::Fallthrough
            }
            M::Inc | M::Dec => {
                self.lift_inc_dec(insn, &mut stmts);
                Flow::Fallthrough
            }
            M::Neg | M::Not => {
                self.lift_unary(insn, &mut stmts);
                Flow::Fallthrough
            }
            M::Shl | M::Shr | M::Sar => {
                self.lift_shift(insn, &mut stmts);
                Flow::Fallthrough
            }
            M::Cmp => {
                self.lift_cmp(insn, false);
                Flow::Fallthrough
            }
            M::Test => {
                self.lift_cmp(insn, true);
                Flow::Fallthrough
            }
            M::Call => match insn.flow() {
                recraft_decode::FlowKind::Call => match insn.branch_target() {
                    Some(target) => {
                        stmts.push(Stmt::Call { target });
                        Flow::Fallthrough
                    }
                    None => self.intrinsic(insn, &mut stmts),
                },
                _ => self.intrinsic(insn, &mut stmts),
            },
            M::Jmp => match insn.flow() {
                recraft_decode::FlowKind::UnconditionalBranch => match insn.branch_target() {
                    Some(target) => Flow::Goto(target),
                    None => self.intrinsic(insn, &mut stmts),
                },
                _ => self.intrinsic(insn, &mut stmts),
            },
            M::Ret | M::Retf | M::Iret => Flow::Return,
            M::Nop => Flow::Fallthrough,
            M::Int3 => self.intrinsic(insn, &mut stmts),
            mnemonic => {
                if let Some(kind) = condition_of(mnemonic) {
                    self.lift_jcc(insn, kind, &mut stmts)
                } else {
                    self.intrinsic(insn, &mut stmts)
                }
            }
        };
        LiftedInsn { addr, stmts, flow }
    }

    fn intrinsic(&mut self, insn: &Insn, stmts: &mut Vec<Stmt>) -> Flow {
        stmts.push(Stmt::Intrinsic { text: insn.asm() });
        Flow::Fallthrough
    }

    fn read_operand(&mut self, insn: &Insn, index: u32) -> Option<ExprId> {
        use iced_x86::OpKind;
        match insn.op_kind(index) {
            OpKind::Register => {
                let view = from_iced(insn.op_register(index))?;
                Some(self.reg(view))
            }
            OpKind::Memory => {
                let size = memory_size_bytes(insn)?;
                let addr = self.memory_address(insn)?;
                Some(self.push(Expr::Load { addr, size }))
            }
            OpKind::Immediate8
            | OpKind::Immediate8to16
            | OpKind::Immediate8to32
            | OpKind::Immediate8to64
            | OpKind::Immediate16
            | OpKind::Immediate32
            | OpKind::Immediate32to64
            | OpKind::Immediate64 => {
                let value = immediate_value(insn, index)?;
                Some(self.constant(value))
            }
            _ => None,
        }
    }

    fn write_operand(
        &mut self,
        insn: &Insn,
        index: u32,
        value: ExprId,
        stmts: &mut Vec<Stmt>,
    ) -> bool {
        use iced_x86::OpKind;
        match insn.op_kind(index) {
            OpKind::Register => {
                let Some(view) = from_iced(insn.op_register(index)) else {
                    return false;
                };
                stmts.push(Stmt::WriteReg { reg: view, value });
                true
            }
            OpKind::Memory => {
                let Some(size) = memory_size_bytes(insn) else {
                    return false;
                };
                let Some(addr) = self.memory_address(insn) else {
                    return false;
                };
                stmts.push(Stmt::Store { addr, value, size });
                true
            }
            _ => false,
        }
    }

    fn memory_address(&mut self, insn: &Insn) -> Option<ExprId> {
        use iced_x86::Register as R;
        let segment = insn.memory_segment();
        if !matches!(segment, R::None | R::DS | R::SS) {
            // fs/gs-relative (TEB/PEB, ...) is not modelled in M0.
            return None;
        }
        let displacement_raw = insn.memory_displacement64();
        if insn.is_ip_rel_memory_operand() {
            return Some(self.constant(displacement_raw));
        }
        // Base/index are unsigned, but the displacement is signed: `[rbp-0x10]`
        // must print as a subtraction, not as `rbp + 0xfffffffffffffff0`.
        let displacement = displacement_raw as i64;
        let base = insn.memory_base();
        let index = insn.memory_index();
        let scale = insn.memory_index_scale();
        let base_expr = if base == R::None {
            None
        } else {
            Some(self.reg(from_iced(base)?))
        };
        let index_expr = if index == R::None {
            None
        } else {
            let view = from_iced(index)?;
            let index_expr = self.reg(view);
            let scale_expr = self.constant(u64::from(scale));
            Some(self.push(Expr::Bin {
                op: BinOp::Mul,
                lhs: index_expr,
                rhs: scale_expr,
            }))
        };
        let sum = match (base_expr, index_expr) {
            (Some(base_expr), Some(index_expr)) => Some(self.push(Expr::Bin {
                op: BinOp::Add,
                lhs: base_expr,
                rhs: index_expr,
            })),
            (Some(base_expr), None) => Some(base_expr),
            (None, Some(index_expr)) => Some(index_expr),
            (None, None) => None,
        };
        let value = match (sum, displacement) {
            (Some(sum), 0) => sum,
            (Some(sum), delta) if delta > 0 => {
                let delta_expr = self.constant(delta as u64);
                self.push(Expr::Bin {
                    op: BinOp::Add,
                    lhs: sum,
                    rhs: delta_expr,
                })
            }
            (Some(sum), delta) => {
                let delta_expr = self.constant(delta.unsigned_abs());
                self.push(Expr::Bin {
                    op: BinOp::Sub,
                    lhs: sum,
                    rhs: delta_expr,
                })
            }
            (None, _) => self.constant(displacement_raw),
        };
        Some(value)
    }

    fn lift_mov(&mut self, insn: &Insn, stmts: &mut Vec<Stmt>) -> Flow {
        let Some(value) = self.read_operand(insn, 1) else {
            return self.intrinsic(insn, stmts);
        };
        if self.write_operand(insn, 0, value, stmts) {
            Flow::Fallthrough
        } else {
            self.intrinsic(insn, stmts)
        }
    }

    fn lift_movx(&mut self, insn: &Insn, stmts: &mut Vec<Stmt>, signed: bool) -> Flow {
        let Some(value) = self.read_operand(insn, 1) else {
            return self.intrinsic(insn, stmts);
        };
        let width = destination_width(insn).unwrap_or(4);
        let cast = self.push(Expr::Cast {
            arg: value,
            width,
            signed,
        });
        if self.write_operand(insn, 0, cast, stmts) {
            Flow::Fallthrough
        } else {
            self.intrinsic(insn, stmts)
        }
    }

    fn lift_lea(&mut self, insn: &Insn, stmts: &mut Vec<Stmt>) -> Flow {
        let Some(address) = self.memory_address(insn) else {
            return self.intrinsic(insn, stmts);
        };
        if self.write_operand(insn, 0, address, stmts) {
            Flow::Fallthrough
        } else {
            self.intrinsic(insn, stmts)
        }
    }

    fn lift_binary(&mut self, insn: &Insn, stmts: &mut Vec<Stmt>) {
        use iced_x86::Mnemonic as M;
        let op = match insn.mnemonic() {
            M::Add => BinOp::Add,
            M::Sub => BinOp::Sub,
            M::And => BinOp::And,
            M::Or => BinOp::Or,
            _ => BinOp::Xor,
        };
        let (Some(lhs), Some(rhs)) = (self.read_operand(insn, 0), self.read_operand(insn, 1))
        else {
            stmts.push(Stmt::Intrinsic { text: insn.asm() });
            return;
        };
        let result = self.push(Expr::Bin { op, lhs, rhs });
        if insn.mnemonic() == M::Sub {
            // `sub` produces full arithmetic flags relative to its operands.
            self.last_cmp = Some(CmpState {
                lhs,
                rhs,
                source: CmpSource::Arithmetic,
            });
        } else {
            // For add/and/or/xor only the zero flag meaning is trustworthy.
            let zero = self.constant(0);
            self.last_cmp = Some(CmpState {
                lhs: result,
                rhs: zero,
                source: CmpSource::ZeroOnly,
            });
        }
        if !self.write_operand(insn, 0, result, stmts) {
            stmts.push(Stmt::Intrinsic { text: insn.asm() });
        }
    }

    fn lift_inc_dec(&mut self, insn: &Insn, stmts: &mut Vec<Stmt>) {
        use iced_x86::Mnemonic as M;
        let op = if insn.mnemonic() == M::Inc {
            BinOp::Add
        } else {
            BinOp::Sub
        };
        let Some(lhs) = self.read_operand(insn, 0) else {
            stmts.push(Stmt::Intrinsic { text: insn.asm() });
            return;
        };
        let one = self.constant(1);
        let result = self.push(Expr::Bin { op, lhs, rhs: one });
        let zero = self.constant(0);
        self.last_cmp = Some(CmpState {
            lhs: result,
            rhs: zero,
            source: CmpSource::ZeroOnly,
        });
        if !self.write_operand(insn, 0, result, stmts) {
            stmts.push(Stmt::Intrinsic { text: insn.asm() });
        }
    }

    fn lift_unary(&mut self, insn: &Insn, stmts: &mut Vec<Stmt>) {
        use iced_x86::Mnemonic as M;
        let op = if insn.mnemonic() == M::Neg {
            UnOp::Neg
        } else {
            UnOp::Not
        };
        let Some(arg) = self.read_operand(insn, 0) else {
            stmts.push(Stmt::Intrinsic { text: insn.asm() });
            return;
        };
        let result = self.push(Expr::Un { op, arg });
        if !self.write_operand(insn, 0, result, stmts) {
            stmts.push(Stmt::Intrinsic { text: insn.asm() });
        }
    }

    fn lift_shift(&mut self, insn: &Insn, stmts: &mut Vec<Stmt>) {
        use iced_x86::Mnemonic as M;
        let op = match insn.mnemonic() {
            M::Shl => BinOp::Shl,
            M::Shr => BinOp::Shr,
            _ => BinOp::Sar,
        };
        let (Some(lhs), Some(rhs)) = (self.read_operand(insn, 0), self.read_operand(insn, 1))
        else {
            stmts.push(Stmt::Intrinsic { text: insn.asm() });
            return;
        };
        let result = self.push(Expr::Bin { op, lhs, rhs });
        if !self.write_operand(insn, 0, result, stmts) {
            stmts.push(Stmt::Intrinsic { text: insn.asm() });
        }
    }

    fn lift_cmp(&mut self, insn: &Insn, test: bool) {
        let (Some(lhs), Some(rhs)) = (self.read_operand(insn, 0), self.read_operand(insn, 1))
        else {
            self.last_cmp = None;
            return;
        };
        if test {
            let and = self.push(Expr::Bin {
                op: BinOp::And,
                lhs,
                rhs,
            });
            let zero = self.constant(0);
            self.last_cmp = Some(CmpState {
                lhs: and,
                rhs: zero,
                source: CmpSource::ZeroOnly,
            });
        } else {
            self.last_cmp = Some(CmpState {
                lhs,
                rhs,
                source: CmpSource::Arithmetic,
            });
        }
    }

    fn lift_jcc(&mut self, insn: &Insn, kind: CondKind, stmts: &mut Vec<Stmt>) -> Flow {
        let Some(target) = insn.branch_target() else {
            return self.intrinsic(insn, stmts);
        };
        let cond = match (self.last_cmp, kind.comparison_symbol()) {
            (Some(state), Some(_))
                if state.source == CmpSource::Arithmetic || uses_zero_only(kind) =>
            {
                self.push(Expr::Cmp {
                    kind,
                    lhs: state.lhs,
                    rhs: state.rhs,
                })
            }
            _ => {
                // Flags not modelled: keep the branch, but say so.
                stmts.push(Stmt::Intrinsic {
                    text: format!("if ({} /* unmodelled flags */) goto {target}", insn.asm()),
                });
                return Flow::Fallthrough;
            }
        };
        self.last_cmp = None;
        Flow::CondGoto { cond, target }
    }
}

/// Only equality/zero-ish predicates survive non-arithmetic flag producers.
fn uses_zero_only(kind: CondKind) -> bool {
    matches!(kind, CondKind::Eq | CondKind::Ne)
}

fn condition_of(mnemonic: iced_x86::Mnemonic) -> Option<CondKind> {
    use iced_x86::Mnemonic as M;
    Some(match mnemonic {
        // iced canonicalizes aliases (jz -> je), so only the primary names exist.
        M::Je => CondKind::Eq,
        M::Jne => CondKind::Ne,
        M::Ja => CondKind::UnsignedGt,
        M::Jae => CondKind::UnsignedGe,
        M::Jb => CondKind::UnsignedLt,
        M::Jbe => CondKind::UnsignedLe,
        M::Jg => CondKind::SignedGt,
        M::Jge => CondKind::SignedGe,
        M::Jl => CondKind::SignedLt,
        M::Jle => CondKind::SignedLe,
        M::Js => CondKind::Sign,
        M::Jns => CondKind::NotSign,
        M::Jo => CondKind::Overflow,
        M::Jno => CondKind::NotOverflow,
        M::Jp => CondKind::Parity,
        M::Jnp => CondKind::NotParity,
        _ => return None,
    })
}

fn destination_width(insn: &Insn) -> Option<u8> {
    if insn.op_kind(0) != iced_x86::OpKind::Register {
        return None;
    }
    from_iced(insn.op_register(0)).map(|view| view.width)
}

fn memory_size_bytes(insn: &Insn) -> Option<u8> {
    use iced_x86::MemorySize as S;
    Some(match insn.memory_size() {
        S::UInt8 | S::Int8 => 1,
        S::UInt16 | S::Int16 => 2,
        S::UInt32 | S::Int32 | S::Float32 => 4,
        S::UInt64 | S::Int64 | S::Float64 => 8,
        _ => return None,
    })
}

fn immediate_value(insn: &Insn, index: u32) -> Option<u64> {
    use iced_x86::OpKind;
    Some(match insn.op_kind(index) {
        OpKind::Immediate8 => u64::from(insn.immediate8()),
        OpKind::Immediate8to16 => insn.immediate8to16() as i64 as u64,
        OpKind::Immediate8to32 => insn.immediate8to32() as i64 as u64,
        OpKind::Immediate8to64 => insn.immediate8to64() as u64,
        OpKind::Immediate16 => u64::from(insn.immediate16()),
        OpKind::Immediate32 => u64::from(insn.immediate32()),
        OpKind::Immediate32to64 => insn.immediate32to64() as u64,
        OpKind::Immediate64 => insn.immediate64(),
        _ => return None,
    })
}

/// Convenience: lift one instruction decoded from `bytes` at `ip` (x64).
#[cfg(test)]
fn lift_one(bytes: &[u8], ip: u64) -> (Lifter, LiftedInsn) {
    let insn = recraft_decode::decode(
        bytes,
        recraft_common::Address::new(ip),
        recraft_decode::Mode::X64,
    )
    .expect("decoded");
    let mut lifter = Lifter::new();
    let item = lifter.lift(&insn);
    (lifter, item)
}

#[cfg(test)]
mod tests {
    use super::*;
    use recraft_common::Address;

    #[test]
    fn mov_reg_reg() {
        let (lifter, item) = lift_one(&[0x48, 0x89, 0xd8], 0x1000); // mov rax, rbx
        assert_eq!(item.stmts.len(), 1);
        match &item.stmts[0] {
            Stmt::WriteReg { reg, value } => {
                assert_eq!(reg.name(), "rax");
                assert_eq!(
                    *lifter.expr(*value),
                    Expr::Reg(RegView::full(crate::reg::Reg::Rbx))
                );
            }
            other => panic!("unexpected stmt {other:?}"),
        }
        assert_eq!(item.flow, Flow::Fallthrough);
    }

    #[test]
    fn add_register_immediate() {
        let (lifter, item) = lift_one(&[0x48, 0x83, 0xc4, 0x28], 0x1000); // add rsp, 0x28
        match &item.stmts[0] {
            Stmt::WriteReg { reg, value } => {
                assert_eq!(reg.name(), "rsp");
                match lifter.expr(*value) {
                    Expr::Bin {
                        op: BinOp::Add,
                        lhs,
                        rhs,
                    } => {
                        assert!(matches!(lifter.expr(*lhs), Expr::Reg(_)));
                        assert!(matches!(lifter.expr(*rhs), Expr::Const(0x28)));
                    }
                    other => panic!("unexpected expr {other:?}"),
                }
            }
            other => panic!("unexpected stmt {other:?}"),
        }
    }

    #[test]
    fn mov_with_memory_operand() {
        let (lifter, item) = lift_one(&[0x48, 0x8b, 0x43, 0x08], 0x1000); // mov rax, [rbx+8]
        match &item.stmts[0] {
            Stmt::WriteReg { value, .. } => match lifter.expr(*value) {
                Expr::Load { addr, size } => {
                    assert_eq!(*size, 8);
                    assert!(matches!(
                        lifter.expr(*addr),
                        Expr::Bin { op: BinOp::Add, .. }
                    ));
                }
                other => panic!("unexpected expr {other:?}"),
            },
            other => panic!("unexpected stmt {other:?}"),
        }
    }

    #[test]
    fn lea_does_not_load() {
        let (lifter, item) = lift_one(&[0x48, 0x8d, 0x44, 0x8b, 0x10], 0x1000); // lea rax, [rbx+rcx*4+0x10]
        match &item.stmts[0] {
            Stmt::WriteReg { value, .. } => {
                assert!(!matches!(lifter.expr(*value), Expr::Load { .. }));
            }
            other => panic!("unexpected stmt {other:?}"),
        }
    }

    #[test]
    fn cmp_and_jcc_build_condition() {
        let (mut lifter, item) = lift_one(&[0x48, 0x83, 0xfc, 0x10], 0x1000); // cmp rsp, 0x10
        assert!(item.stmts.is_empty());
        assert_eq!(item.flow, Flow::Fallthrough);

        let insn = recraft_decode::decode(
            &[0x74, 0x05],
            Address::new(0x1004),
            recraft_decode::Mode::X64,
        )
        .unwrap();
        let jcc = lifter.lift(&insn);
        match jcc.flow {
            Flow::CondGoto { cond, target } => {
                assert_eq!(target, Address::new(0x100b));
                match lifter.expr(cond) {
                    Expr::Cmp { kind, lhs, rhs } => {
                        assert_eq!(*kind, CondKind::Eq);
                        assert!(matches!(lifter.expr(*lhs), Expr::Reg(_)));
                        assert!(matches!(lifter.expr(*rhs), Expr::Const(0x10)));
                    }
                    other => panic!("unexpected cond {other:?}"),
                }
            }
            other => panic!("unexpected flow {other:?}"),
        }
    }

    #[test]
    fn call_and_ret() {
        let (_, call) = lift_one(&[0xe8, 0x10, 0x00, 0x00, 0x00], 0x1000);
        assert_eq!(
            call.stmts[0],
            Stmt::Call {
                target: Address::new(0x1015)
            }
        );
        let (_, ret) = lift_one(&[0xc3], 0x1000);
        assert_eq!(ret.flow, Flow::Return);
    }

    #[test]
    fn unsupported_becomes_intrinsic() {
        let (_, push) = lift_one(&[0x50], 0x1000); // push rax
        match &push.stmts[0] {
            Stmt::Intrinsic { text } => assert!(text.contains("push"), "text: {text}"),
            other => panic!("unexpected stmt {other:?}"),
        }
    }

    #[test]
    fn movzx_casts() {
        let (lifter, item) = lift_one(&[0x0f, 0xb6, 0xc1], 0x1000); // movzx eax, cl
        match &item.stmts[0] {
            Stmt::WriteReg { reg, value } => {
                assert_eq!(reg.name(), "eax");
                assert!(matches!(
                    lifter.expr(*value),
                    Expr::Cast {
                        signed: false,
                        width: 4,
                        ..
                    }
                ));
            }
            other => panic!("unexpected stmt {other:?}"),
        }
    }
}
