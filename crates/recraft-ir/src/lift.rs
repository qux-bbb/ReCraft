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

use std::collections::BTreeMap;

use recraft_decode::{Insn, iced_x86};

use crate::expr::{BinOp, CondKind, Expr, ExprId, Flow, LiftedInsn, Stmt, UnOp};
use crate::reg::{Reg, RegView, from_iced};

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
    /// Latest value written to each register (used to recover call arguments).
    reg_env: BTreeMap<Reg, ExprId>,
    /// Recent `push` values (oldest first); the newest pushes are the call arguments
    /// on x86 (and arguments 5+ on x64).
    stack_args: Vec<ExprId>,
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
            reg_env: BTreeMap::new(),
            stack_args: Vec::new(),
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
            M::Mov | M::Movd | M::Movq | M::Movdqa | M::Movdqu | M::Movaps | M::Movups => {
                self.lift_mov(insn, &mut stmts)
            }
            M::Movss => self.lift_mov_float(insn, 4, &mut stmts),
            M::Movsd => self.lift_mov_float(insn, 8, &mut stmts),
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
            M::Imul => self.lift_imul(insn, &mut stmts),
            M::Push => self.lift_push(insn, &mut stmts),
            M::Pop => self.lift_pop(insn, &mut stmts),
            // Packed integer SIMD: lane-wise arithmetic prints as scalar operators.
            M::Paddb | M::Paddw | M::Paddd | M::Paddq => {
                self.lift_binary_plain(insn, BinOp::Add, &mut stmts);
                Flow::Fallthrough
            }
            M::Psubb | M::Psubw | M::Psubd | M::Psubq => {
                self.lift_binary_plain(insn, BinOp::Sub, &mut stmts);
                Flow::Fallthrough
            }
            M::Pmullw | M::Pmulld | M::Pmuludq => {
                self.lift_binary_plain(insn, BinOp::Mul, &mut stmts);
                Flow::Fallthrough
            }
            M::Pand => {
                self.lift_binary_plain(insn, BinOp::And, &mut stmts);
                Flow::Fallthrough
            }
            M::Por => {
                self.lift_binary_plain(insn, BinOp::Or, &mut stmts);
                Flow::Fallthrough
            }
            M::Pxor | M::Xorps | M::Xorpd => {
                self.lift_binary_plain(insn, BinOp::Xor, &mut stmts);
                Flow::Fallthrough
            }
            M::Psrldq => {
                self.lift_binary_plain(insn, BinOp::Shr, &mut stmts);
                Flow::Fallthrough
            }
            M::Pslldq => {
                self.lift_binary_plain(insn, BinOp::Shl, &mut stmts);
                Flow::Fallthrough
            }
            // Scalar float arithmetic (single/double precision).
            M::Addss | M::Addsd => {
                self.lift_binary_float(insn, BinOp::FAdd, float_operand_size(insn), &mut stmts);
                Flow::Fallthrough
            }
            M::Subss | M::Subsd => {
                self.lift_binary_float(insn, BinOp::FSub, float_operand_size(insn), &mut stmts);
                Flow::Fallthrough
            }
            M::Mulss | M::Mulsd => {
                self.lift_binary_float(insn, BinOp::FMul, float_operand_size(insn), &mut stmts);
                Flow::Fallthrough
            }
            M::Divss | M::Divsd => {
                self.lift_binary_float(insn, BinOp::FDiv, float_operand_size(insn), &mut stmts);
                Flow::Fallthrough
            }
            M::Cvttss2si | M::Cvttsd2si | M::Cvtsi2ss | M::Cvtsi2sd => {
                self.lift_convert(insn, &mut stmts)
            }
            M::Call => match insn.flow() {
                recraft_decode::FlowKind::Call => match insn.branch_target() {
                    Some(target) => {
                        let mut args: Vec<ExprId> = if insn.mode().is_64() {
                            const ARG_REGS: [Reg; 4] = [Reg::Rcx, Reg::Rdx, Reg::R8, Reg::R9];
                            ARG_REGS
                                .iter()
                                .map(|reg| match self.reg_env.get(reg).copied() {
                                    // A self-referential value (`dec ecx` -> `ecx - 1`) was
                                    // computed from the pre-write value; by the call the
                                    // register already holds the result, so print the
                                    // register itself to stay semantically correct.
                                    Some(value) if expr_reads_reg(&self.exprs, value, *reg) => {
                                        self.reg(RegView::full(*reg))
                                    }
                                    Some(value) => value,
                                    None => self.reg(RegView::full(*reg)),
                                })
                                .collect()
                        } else {
                            Vec::new()
                        };
                        // Arguments pushed on the stack: newest push is the first
                        // stack argument (x86: all arguments; x64: arguments 5+).
                        let stack_count = self.stack_args.len().min(8);
                        for value in self.stack_args.iter().rev().take(stack_count) {
                            args.push(*value);
                        }
                        stmts.push(Stmt::Call { target, args });
                        // Caller-saved registers are clobbered by the call.
                        for reg in [
                            Reg::Rcx,
                            Reg::Rdx,
                            Reg::R8,
                            Reg::R9,
                            Reg::Rax,
                            Reg::R10,
                            Reg::R11,
                        ] {
                            self.reg_env.remove(&reg);
                        }
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
                } else if let Some(kind) = cmov_condition_of(mnemonic) {
                    self.lift_cmov(insn, kind, &mut stmts)
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
                if view.offset == 0 {
                    self.reg_env.insert(view.base, value);
                } else {
                    self.reg_env.remove(&view.base);
                }
                if view.base == Reg::Rsp {
                    // `sub esp`/`add esp` move the frame: stale push values are void.
                    self.stack_args.clear();
                }
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

    fn lift_binary_plain(&mut self, insn: &Insn, op: BinOp, stmts: &mut Vec<Stmt>) {
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

    fn read_operand_float(&mut self, insn: &Insn, index: u32, float_size: u8) -> Option<ExprId> {
        use iced_x86::OpKind;
        match insn.op_kind(index) {
            OpKind::Register => {
                let view = from_iced(insn.op_register(index))?;
                Some(self.reg(view))
            }
            OpKind::Memory => {
                let addr = self.memory_address(insn)?;
                Some(self.push(Expr::LoadFloat {
                    addr,
                    size: float_size,
                }))
            }
            _ => None,
        }
    }

    fn write_operand_float(
        &mut self,
        insn: &Insn,
        index: u32,
        value: ExprId,
        float_size: u8,
        stmts: &mut Vec<Stmt>,
    ) -> bool {
        use iced_x86::OpKind;
        match insn.op_kind(index) {
            OpKind::Register => {
                let Some(view) = from_iced(insn.op_register(index)) else {
                    return false;
                };
                stmts.push(Stmt::WriteReg { reg: view, value });
                if view.offset == 0 {
                    self.reg_env.insert(view.base, value);
                } else {
                    self.reg_env.remove(&view.base);
                }
                if view.base == Reg::Rsp {
                    // `sub esp`/`add esp` move the frame: stale push values are void.
                    self.stack_args.clear();
                }
                true
            }
            OpKind::Memory => {
                let Some(addr) = self.memory_address(insn) else {
                    return false;
                };
                stmts.push(Stmt::StoreFloat {
                    addr,
                    value,
                    size: float_size,
                });
                true
            }
            _ => false,
        }
    }

    fn lift_mov_float(&mut self, insn: &Insn, float_size: u8, stmts: &mut Vec<Stmt>) -> Flow {
        let Some(value) = self.read_operand_float(insn, 1, float_size) else {
            return self.intrinsic(insn, stmts);
        };
        if self.write_operand_float(insn, 0, value, float_size, stmts) {
            Flow::Fallthrough
        } else {
            self.intrinsic(insn, stmts)
        }
    }

    fn lift_binary_float(&mut self, insn: &Insn, op: BinOp, float_size: u8, stmts: &mut Vec<Stmt>) {
        let (Some(lhs), Some(rhs)) = (
            self.read_operand_float(insn, 0, float_size),
            self.read_operand_float(insn, 1, float_size),
        ) else {
            stmts.push(Stmt::Intrinsic { text: insn.asm() });
            return;
        };
        let result = self.push(Expr::Bin { op, lhs, rhs });
        if !self.write_operand_float(insn, 0, result, float_size, stmts) {
            stmts.push(Stmt::Intrinsic { text: insn.asm() });
        }
    }

    /// Float <-> integer conversions: modelled as a cast; flag state is dropped because
    /// these instructions do not update EFLAGS.
    fn lift_convert(&mut self, insn: &Insn, stmts: &mut Vec<Stmt>) -> Flow {
        use iced_x86::Mnemonic as M;
        let float_size = float_operand_size(insn);
        let source = match insn.mnemonic() {
            // Float -> integer: source is a float operand.
            M::Cvttss2si | M::Cvttsd2si => self.read_operand_float(insn, 1, float_size),
            // Integer -> float: source is an integer operand, destination is float.
            _ => self.read_operand(insn, 1),
        };
        let Some(source) = source else {
            return self.intrinsic(insn, stmts);
        };
        let width = match insn.mnemonic() {
            M::Cvttss2si | M::Cvttsd2si => destination_width(insn).unwrap_or(4),
            _ => 4,
        };
        let cast = self.push(Expr::Cast {
            arg: source,
            width,
            signed: true,
        });
        let written = match insn.mnemonic() {
            M::Cvtsi2ss | M::Cvtsi2sd => self.write_operand_float(insn, 0, cast, float_size, stmts),
            _ => self.write_operand(insn, 0, cast, stmts),
        };
        if written {
            self.last_cmp = None;
            Flow::Fallthrough
        } else {
            self.intrinsic(insn, stmts)
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
        let Some(condition) = self.condition_expr(kind, true) else {
            // Flags not modelled: keep the branch, but say so.
            stmts.push(Stmt::Intrinsic {
                text: format!("if ({} /* unmodelled flags */) goto {target}", insn.asm()),
            });
            return Flow::Fallthrough;
        };
        Flow::CondGoto {
            cond: condition,
            target,
        }
    }

    /// Build `Expr::Cmp` from the lazily tracked comparison when its flags are usable for
    /// `kind`. `consume` clears the tracked comparison (branches do; `cmov` does not).
    fn condition_expr(&mut self, kind: CondKind, consume: bool) -> Option<ExprId> {
        let state = self.last_cmp?;
        if state.source != CmpSource::Arithmetic && !uses_zero_only(kind) {
            return None;
        }
        let condition = self.push(Expr::Cmp {
            kind,
            lhs: state.lhs,
            rhs: state.rhs,
        });
        if consume {
            self.last_cmp = None;
        }
        Some(condition)
    }

    fn lift_imul(&mut self, insn: &Insn, stmts: &mut Vec<Stmt>) -> Flow {
        // 2-operand: dst = dst * src; 3-operand: dst = src * imm. The one-operand form
        // writes rdx:rax and stays an intrinsic.
        let (lhs, rhs) = match insn.op_count() {
            2 => (self.read_operand(insn, 0), self.read_operand(insn, 1)),
            3 => (self.read_operand(insn, 1), self.read_operand(insn, 2)),
            _ => return self.intrinsic(insn, stmts),
        };
        let (Some(lhs), Some(rhs)) = (lhs, rhs) else {
            return self.intrinsic(insn, stmts);
        };
        let product = self.push(Expr::Bin {
            op: BinOp::Mul,
            lhs,
            rhs,
        });
        if self.write_operand(insn, 0, product, stmts) {
            // imul clobbers flags; zero-only consumers must not reuse an old compare.
            self.last_cmp = None;
            Flow::Fallthrough
        } else {
            self.intrinsic(insn, stmts)
        }
    }

    fn lift_cmov(&mut self, insn: &Insn, kind: CondKind, stmts: &mut Vec<Stmt>) -> Flow {
        let Some(condition) = self.condition_expr(kind, false) else {
            return self.intrinsic(insn, stmts);
        };
        let (Some(source), Some(current)) =
            (self.read_operand(insn, 1), self.read_operand(insn, 0))
        else {
            return self.intrinsic(insn, stmts);
        };
        let selected = self.push(Expr::Select {
            cond: condition,
            then: source,
            otherwise: current,
        });
        if self.write_operand(insn, 0, selected, stmts) {
            // cmov preserves flags: the tracked comparison stays valid for a later jcc.
            Flow::Fallthrough
        } else {
            self.intrinsic(insn, stmts)
        }
    }

    fn pointer_size(insn: &Insn) -> usize {
        if insn.mode().is_64() { 8 } else { 4 }
    }

    fn lift_push(&mut self, insn: &Insn, stmts: &mut Vec<Stmt>) -> Flow {
        let size = Self::pointer_size(insn);
        let Some(value) = self.read_operand(insn, 0) else {
            return self.intrinsic(insn, stmts);
        };
        let rsp_old = self.reg(RegView::full(crate::reg::Reg::Rsp));
        let delta = self.constant(size as u64);
        let rsp_new = self.push(Expr::Bin {
            op: BinOp::Sub,
            lhs: rsp_old,
            rhs: delta,
        });
        // Store before updating rsp: the emitted expression is evaluated at its print
        // position, so `*(u64*)((rsp - 8))` must still see the old stack pointer.
        stmts.push(Stmt::Store {
            addr: rsp_new,
            value,
            size: size as u8,
        });
        stmts.push(Stmt::WriteReg {
            reg: RegView::full(crate::reg::Reg::Rsp),
            value: rsp_new,
        });
        self.stack_args.push(value);
        if self.stack_args.len() > 8 {
            self.stack_args.remove(0);
        }
        Flow::Fallthrough
    }

    fn lift_pop(&mut self, insn: &Insn, stmts: &mut Vec<Stmt>) -> Flow {
        use iced_x86::OpKind;
        let size = Self::pointer_size(insn);
        // `pop rsp` has order-dependent semantics; keep it as assembly.
        if insn.op_kind(0) == OpKind::Register
            && crate::reg::from_iced(insn.op_register(0))
                .is_some_and(|view| view.base == crate::reg::Reg::Rsp)
        {
            return self.intrinsic(insn, stmts);
        }
        let rsp = self.reg(RegView::full(crate::reg::Reg::Rsp));
        let value = self.push(Expr::Load {
            addr: rsp,
            size: size as u8,
        });
        if !self.write_operand(insn, 0, value, stmts) {
            return self.intrinsic(insn, stmts);
        }
        let rsp_old = self.reg(RegView::full(crate::reg::Reg::Rsp));
        let delta = self.constant(size as u64);
        let rsp_new = self.push(Expr::Bin {
            op: BinOp::Add,
            lhs: rsp_old,
            rhs: delta,
        });
        stmts.push(Stmt::WriteReg {
            reg: RegView::full(crate::reg::Reg::Rsp),
            value: rsp_new,
        });
        self.stack_args.clear();
        Flow::Fallthrough
    }
}

/// Only equality/zero-ish predicates survive non-arithmetic flag producers.
fn uses_zero_only(kind: CondKind) -> bool {
    // `test a, b` sets SF/ZF from `a & b`, clears OF and CF. Equality is exact, and
    // signed comparisons are valid too because OF=0: jle means `(a&b) <= 0`, etc.
    matches!(
        kind,
        CondKind::Eq
            | CondKind::Ne
            | CondKind::SignedGt
            | CondKind::SignedGe
            | CondKind::SignedLt
            | CondKind::SignedLe
    )
}

fn expr_reads_reg(exprs: &[Expr], expr: ExprId, reg: Reg) -> bool {
    match &exprs[expr as usize] {
        Expr::Reg(view) => view.base == reg,
        Expr::Bin { lhs, rhs, .. } => {
            expr_reads_reg(exprs, *lhs, reg) || expr_reads_reg(exprs, *rhs, reg)
        }
        Expr::Un { arg, .. }
        | Expr::Cast { arg, .. }
        | Expr::Load { addr: arg, .. }
        | Expr::LoadFloat { addr: arg, .. } => expr_reads_reg(exprs, *arg, reg),
        Expr::Cmp { lhs, rhs, .. } => {
            expr_reads_reg(exprs, *lhs, reg) || expr_reads_reg(exprs, *rhs, reg)
        }
        Expr::Select {
            cond,
            then,
            otherwise,
        } => {
            expr_reads_reg(exprs, *cond, reg)
                || expr_reads_reg(exprs, *then, reg)
                || expr_reads_reg(exprs, *otherwise, reg)
        }
        Expr::Const(_) | Expr::Temp(_) => false,
    }
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

fn cmov_condition_of(mnemonic: iced_x86::Mnemonic) -> Option<CondKind> {
    use iced_x86::Mnemonic as M;
    Some(match mnemonic {
        M::Cmove => CondKind::Eq,
        M::Cmovne => CondKind::Ne,
        M::Cmova => CondKind::UnsignedGt,
        M::Cmovae => CondKind::UnsignedGe,
        M::Cmovb => CondKind::UnsignedLt,
        M::Cmovbe => CondKind::UnsignedLe,
        M::Cmovg => CondKind::SignedGt,
        M::Cmovge => CondKind::SignedGe,
        M::Cmovl => CondKind::SignedLt,
        M::Cmovle => CondKind::SignedLe,
        M::Cmovs => CondKind::Sign,
        M::Cmovns => CondKind::NotSign,
        M::Cmovo => CondKind::Overflow,
        M::Cmovno => CondKind::NotOverflow,
        M::Cmovp => CondKind::Parity,
        M::Cmovnp => CondKind::NotParity,
        _ => return None,
    })
}

fn destination_width(insn: &Insn) -> Option<u8> {
    if insn.op_kind(0) != iced_x86::OpKind::Register {
        return None;
    }
    from_iced(insn.op_register(0)).map(|view| view.width)
}

/// Scalar float operand width for an SSE instruction (4 = f32, 8 = f64).
fn float_operand_size(insn: &Insn) -> u8 {
    use iced_x86::Mnemonic as M;
    match insn.mnemonic() {
        M::Movsd | M::Addsd | M::Subsd | M::Mulsd | M::Divsd | M::Cvttsd2si | M::Cvtsi2sd => 8,
        _ => 4,
    }
}

fn memory_size_bytes(insn: &Insn) -> Option<u8> {
    use iced_x86::MemorySize as S;
    Some(match insn.memory_size() {
        S::UInt8 | S::Int8 => 1,
        S::UInt16 | S::Int16 => 2,
        S::UInt32 | S::Int32 | S::Float32 => 4,
        S::UInt64 | S::Int64 | S::Float64 => 8,
        S::UInt128 | S::Int128 => 16,
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
    fn test_flags_support_signed_conditional_jumps() {
        // test ecx, ecx ; jle rel8
        let test = recraft_decode::decode(
            &[0x85, 0xc9],
            Address::new(0x1000),
            recraft_decode::Mode::X64,
        )
        .expect("test decoded");
        let jle = recraft_decode::decode(
            &[0x7e, 0x05],
            Address::new(0x1002),
            recraft_decode::Mode::X64,
        )
        .expect("jle decoded");
        let mut lifter = Lifter::new();
        lifter.lift(&test);
        let item = lifter.lift(&jle);
        match item.flow {
            Flow::CondGoto { cond, .. } => match lifter.expr(cond) {
                Expr::Cmp { kind, .. } => assert_eq!(*kind, CondKind::SignedLe),
                other => panic!("expected signed comparison, got {other:?}"),
            },
            other => panic!("expected a conditional branch, got {other:?}"),
        }
    }

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
        match &call.stmts[0] {
            Stmt::Call { target, args } => {
                assert_eq!(*target, Address::new(0x1015));
                // No arguments were set up: the incoming argument registers are used.
                assert_eq!(args.len(), 4);
            }
            other => panic!("unexpected stmt {other:?}"),
        }
        let (_, ret) = lift_one(&[0xc3], 0x1000);
        assert_eq!(ret.flow, Flow::Return);
    }

    #[test]
    fn unsupported_becomes_intrinsic() {
        let (_, cpuid) = lift_one(&[0x0f, 0xa2], 0x1000); // cpuid
        match &cpuid.stmts[0] {
            Stmt::Intrinsic { text } => assert!(text.contains("cpuid"), "text: {text}"),
            other => panic!("unexpected stmt {other:?}"),
        }
    }

    #[test]
    fn push_and_pop_model_the_stack_pointer() {
        let (_, push) = lift_one(&[0x50], 0x1000); // push rax
        assert!(
            push.stmts
                .iter()
                .all(|stmt| !matches!(stmt, Stmt::Intrinsic { .. })),
            "push fell back to intrinsic: {:?}",
            push.stmts
        );
        assert!(matches!(push.stmts[0], Stmt::Store { size: 8, .. }));
        assert!(matches!(
            push.stmts[1],
            Stmt::WriteReg { reg, .. } if reg.base == crate::reg::Reg::Rsp
        ));

        let (_, pop) = lift_one(&[0x58], 0x1000); // pop rax
        assert!(matches!(
            pop.stmts[0],
            Stmt::WriteReg { ref reg, .. } if reg.name() == "rax"
        ));
        assert!(matches!(
            pop.stmts[1],
            Stmt::WriteReg { reg, .. } if reg.base == crate::reg::Reg::Rsp
        ));
    }

    #[test]
    fn imul_lifts_to_multiplication() {
        let (lifter, item) = lift_one(&[0x0f, 0xaf, 0xc3], 0x1000); // imul eax, ebx
        match &item.stmts[0] {
            Stmt::WriteReg { value, .. } => assert!(matches!(
                lifter.expr(*value),
                Expr::Bin { op: BinOp::Mul, .. }
            )),
            other => panic!("unexpected stmt {other:?}"),
        }
    }

    #[test]
    fn cmov_uses_the_lazy_comparison_and_keeps_flags() {
        let cmp = recraft_decode::decode(
            &[0x3b, 0xc3],
            Address::new(0x1000),
            recraft_decode::Mode::X64,
        )
        .expect("cmp");
        let cmov = recraft_decode::decode(
            &[0x0f, 0x45, 0xc3],
            Address::new(0x1002),
            recraft_decode::Mode::X64,
        )
        .expect("cmovne");
        let mut lifter = Lifter::new();
        lifter.lift(&cmp);
        let item = lifter.lift(&cmov);
        assert!(
            item.stmts
                .iter()
                .all(|stmt| !matches!(stmt, Stmt::Intrinsic { .. })),
            "cmov fell back to intrinsic"
        );
        match &item.stmts[0] {
            Stmt::WriteReg { value, .. } => {
                assert!(matches!(lifter.expr(*value), Expr::Select { .. }))
            }
            other => panic!("unexpected stmt {other:?}"),
        }
    }

    #[test]
    fn simd_and_float_ops_are_lifted() {
        // movdqa xmm0, xmm1
        let (_, movdqa) = lift_one(&[0x66, 0x0f, 0x6f, 0xc1], 0x1000);
        assert!(
            matches!(movdqa.stmts[0], Stmt::WriteReg { ref reg, .. } if reg.name() == "xmm0"),
            "unexpected: {:?}",
            movdqa.stmts
        );
        // paddd xmm0, xmm1
        let (lifter, paddd) = lift_one(&[0x66, 0x0f, 0xfe, 0xc1], 0x1000);
        assert!(
            paddd
                .stmts
                .iter()
                .all(|stmt| !matches!(stmt, Stmt::Intrinsic { .. })),
            "paddd fell back: {:?}",
            paddd.stmts
        );
        match &paddd.stmts[0] {
            Stmt::WriteReg { value, .. } => {
                assert!(matches!(
                    lifter.expr(*value),
                    Expr::Bin { op: BinOp::Add, .. }
                ))
            }
            other => panic!("unexpected stmt {other:?}"),
        }
        // addss xmm0, xmm1
        let (lifter, addss) = lift_one(&[0xf3, 0x0f, 0x58, 0xc1], 0x1000);
        assert!(
            addss
                .stmts
                .iter()
                .all(|stmt| !matches!(stmt, Stmt::Intrinsic { .. })),
            "addss fell back: {:?}",
            addss.stmts
        );
        match &addss.stmts[0] {
            Stmt::WriteReg { value, .. } => {
                assert!(matches!(
                    lifter.expr(*value),
                    Expr::Bin {
                        op: BinOp::FAdd,
                        ..
                    }
                ))
            }
            other => panic!("unexpected stmt {other:?}"),
        }
    }

    #[test]
    fn float_memory_operands_are_typed() {
        // movss xmm0, dword ptr [rax]
        let (lifter, load) = lift_one(&[0xf3, 0x0f, 0x10, 0x00], 0x1000);
        match &load.stmts[0] {
            Stmt::WriteReg { value, .. } => assert!(matches!(
                lifter.expr(*value),
                Expr::LoadFloat { size: 4, .. }
            )),
            other => panic!("unexpected stmt {other:?}"),
        }
        // addss xmm0, dword ptr [rax]
        let (lifter, add) = lift_one(&[0xf3, 0x0f, 0x58, 0x00], 0x1000);
        match &add.stmts[0] {
            Stmt::WriteReg { value, .. } => {
                let Expr::Bin { op, rhs, .. } = lifter.expr(*value) else {
                    panic!("expected Bin");
                };
                assert_eq!(*op, BinOp::FAdd);
                assert!(matches!(lifter.expr(*rhs), Expr::LoadFloat { size: 4, .. }));
            }
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
