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

/// Register view of the instruction's destination, when that destination is a register.
///
/// Flags that describe a write are read at a later branch, when the destination already
/// holds the result; naming that register is the only spelling that still matches.
fn written_register_view(insn: &Insn) -> Option<RegView> {
    if insn.op_kind(0) != iced_x86::OpKind::Register {
        return None;
    }
    from_iced(insn.op_register(0))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CmpSource {
    /// `cmp` / `sub`: full arithmetic flags are available.
    Arithmetic,
    /// `test` / bitwise / inc / dec: only zero/sign meanings are trustworthy.
    ZeroOnly,
    /// `bt`: only CF (= the tested bit) is meaningful.
    BitTest,
    /// `neg`: zero/sign flags describe the result, and CF is "operand was non-zero".
    Negate,
}

#[derive(Debug, Clone, Copy)]
struct CmpState {
    lhs: ExprId,
    rhs: ExprId,
    source: CmpSource,
}

/// Calling convention that decides which registers carry call arguments.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CallAbi {
    /// Microsoft x64: `rcx`, `rdx`, `r8`, `r9`.
    #[default]
    Win64,
    /// System V AMD64 (ELF64): `rdi`, `rsi`, `rdx`, `rcx`, `r8`, `r9`.
    SysV64,
    /// System V i386 (ELF32): every integer argument is pushed on the stack, so there
    /// are no argument registers; the pushed values are the call arguments.
    SysV32,
}

impl CallAbi {
    /// Integer argument registers in ABI order.
    pub fn arg_registers(self) -> &'static [Reg] {
        match self {
            CallAbi::Win64 => &[Reg::Rcx, Reg::Rdx, Reg::R8, Reg::R9],
            CallAbi::SysV64 => &[Reg::Rdi, Reg::Rsi, Reg::Rdx, Reg::Rcx, Reg::R8, Reg::R9],
            CallAbi::SysV32 => &[],
        }
    }

    /// Registers a call may clobber (caller-saved).
    pub fn caller_saved(self) -> &'static [Reg] {
        match self {
            CallAbi::Win64 => &[
                Reg::Rcx,
                Reg::Rdx,
                Reg::R8,
                Reg::R9,
                Reg::Rax,
                Reg::R10,
                Reg::R11,
            ],
            CallAbi::SysV64 => &[
                Reg::Rax,
                Reg::Rcx,
                Reg::Rdx,
                Reg::Rsi,
                Reg::Rdi,
                Reg::R8,
                Reg::R9,
                Reg::R10,
                Reg::R11,
            ],
            CallAbi::SysV32 => &[Reg::Rax, Reg::Rcx, Reg::Rdx],
        }
    }
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
    /// Calling convention used to recover call arguments.
    abi: CallAbi,
}

impl Default for Lifter {
    fn default() -> Self {
        Self::new()
    }
}

impl Lifter {
    pub fn new() -> Self {
        Self::with_abi(CallAbi::Win64)
    }

    /// A lifter for an image whose x64 calls use `abi` (ELF binaries use SysV).
    pub fn with_abi(abi: CallAbi) -> Self {
        Self {
            exprs: Vec::new(),
            last_cmp: None,
            reg_env: BTreeMap::new(),
            stack_args: Vec::new(),
            abi,
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
            M::Cdqe => self.lift_cdqe(&mut stmts),
            M::Cdq | M::Cqo | M::Cwd => self.lift_cdq(insn, &mut stmts),
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
            M::Shl | M::Shr | M::Sar | M::Ror | M::Rol => {
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
            M::Bt => {
                self.lift_bt(insn);
                Flow::Fallthrough
            }
            M::Bts | M::Btr | M::Btc => self.lift_bit_modify(insn, &mut stmts),
            M::Sbb => self.lift_sbb(insn, &mut stmts),
            // `lock cmpxchg` stays assembly (it updates memory conditionally), but the
            // comparison flags it leaves behind are exact.
            M::Cmpxchg => {
                self.lift_cmpxchg_flags(insn);
                self.intrinsic(insn, &mut stmts)
            }
            M::Xadd => self.lift_xadd(insn, &mut stmts),
            M::Imul => self.lift_imul(insn, &mut stmts),
            // Bit scans read one operand and write another; rendering them as builtin calls keeps
            // the result usable instead of leaving `__asm("bsf ...")` plus a move in the output.
            M::Bsf | M::Bsr | M::Popcnt => {
                self.lift_bit_scan(insn, &mut stmts);
                Flow::Fallthrough
            }
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
                            self.abi
                                .arg_registers()
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
                        for reg in self.abi.caller_saved() {
                            self.reg_env.remove(reg);
                        }
                        Flow::Fallthrough
                    }
                    None => self.intrinsic(insn, &mut stmts),
                },
                recraft_decode::FlowKind::IndirectCall => {
                    if let Some(target) = self.read_operand(insn, 0) {
                        stmts.push(Stmt::CallIndirect { target });
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
                        self.stack_args.clear();
                        Flow::Fallthrough
                    } else {
                        self.intrinsic(insn, &mut stmts)
                    }
                }
                _ => self.intrinsic(insn, &mut stmts),
            },
            M::Jmp => match insn.flow() {
                recraft_decode::FlowKind::UnconditionalBranch => match insn.branch_target() {
                    Some(target) => Flow::Goto(target),
                    None => self.intrinsic(insn, &mut stmts),
                },
                // `jmp [rip+disp]` is a tail call through an IAT/function-pointer slot.
                // Register or indexed forms are usually jump-table dispatch, which the
                // structurer turns into `switch`; keep those untouched.
                recraft_decode::FlowKind::IndirectBranch if insn.is_ip_rel_memory_operand() => {
                    if let Some(target) = self.read_operand(insn, 0) {
                        stmts.push(Stmt::CallIndirect { target });
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
                        self.stack_args.clear();
                        Flow::Return
                    } else {
                        self.intrinsic(insn, &mut stmts)
                    }
                }
                _ => self.intrinsic(insn, &mut stmts),
            },
            M::Ret | M::Retf | M::Iret => Flow::Return,
            // CET landing pads: Ubuntu's gcc enables -fcf-protection by default, which puts
            // endbr64 at every function entry. It has no source-level effect.
            M::Nop | M::Endbr64 | M::Endbr32 => Flow::Fallthrough,
            M::Int3 => self.intrinsic(insn, &mut stmts),
            mnemonic => {
                if let Some(kind) = condition_of(mnemonic) {
                    self.lift_jcc(insn, kind, &mut stmts)
                } else if let Some(kind) = cmov_condition_of(mnemonic) {
                    self.lift_cmov(insn, kind, &mut stmts)
                } else if let Some(kind) = set_condition_of(mnemonic) {
                    self.lift_setcc(insn, kind, &mut stmts)
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
        // Every two-operand arithmetic instruction writes its destination, so a later
        // branch sees the *new* value: comparing `lhs` again (`sub ecx, 1; jne` →
        // `ecx != 1`) would subtract twice. The zero/sign flags describe the result, and
        // the destination register holds it, so name that register instead. Only equality
        // and sign predicates are trustworthy after arithmetic (OF is not modelled).
        let zero = self.constant(0);
        let lhs = match written_register_view(insn) {
            Some(view) => self.reg(view),
            // A memory destination cannot be re-read cheaply; the result expression is the
            // best available approximation.
            None => result,
        };
        self.last_cmp = Some(CmpState {
            lhs,
            rhs: zero,
            source: CmpSource::ZeroOnly,
        });
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
        let lhs = match written_register_view(insn) {
            Some(view) => self.reg(view),
            None => result,
        };
        self.last_cmp = Some(CmpState {
            lhs,
            rhs: zero,
            source: CmpSource::ZeroOnly,
        });
        if !self.write_operand(insn, 0, result, stmts) {
            stmts.push(Stmt::Intrinsic { text: insn.asm() });
        }
    }

    /// `bsf`/`bsr`/`popcnt`: one operand in, one operand out, no flags we model.
    fn lift_bit_scan(&mut self, insn: &Insn, stmts: &mut Vec<Stmt>) {
        use iced_x86::Mnemonic as M;
        let op = match insn.mnemonic() {
            M::Bsf => UnOp::Ctz,
            M::Bsr => UnOp::Bsr,
            _ => UnOp::Popcount,
        };
        // The source is operand 1; operand 0 is the destination.
        let Some(arg) = self.read_operand(insn, 1) else {
            stmts.push(Stmt::Intrinsic { text: insn.asm() });
            return;
        };
        let result = self.push(Expr::Un { op, arg });
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
        if insn.mnemonic() == M::Neg {
            // `neg` sets its flags from the result; `not` leaves them untouched.
            let zero = self.constant(0);
            let lhs = match written_register_view(insn) {
                Some(view) => self.reg(view),
                None => result,
            };
            self.last_cmp = Some(CmpState {
                lhs,
                rhs: zero,
                source: CmpSource::Negate,
            });
        }
        if !self.write_operand(insn, 0, result, stmts) {
            stmts.push(Stmt::Intrinsic { text: insn.asm() });
        }
    }

    fn lift_shift(&mut self, insn: &Insn, stmts: &mut Vec<Stmt>) {
        use iced_x86::Mnemonic as M;
        let op = match insn.mnemonic() {
            M::Shl => BinOp::Shl,
            M::Shr => BinOp::Shr,
            M::Ror => BinOp::Ror,
            M::Rol => BinOp::Rol,
            _ => BinOp::Sar,
        };
        let (Some(lhs), Some(rhs)) = (self.read_operand(insn, 0), self.read_operand(insn, 1))
        else {
            stmts.push(Stmt::Intrinsic { text: insn.asm() });
            return;
        };
        let result = self.push(Expr::Bin { op, lhs, rhs });
        if matches!(op, BinOp::Shl | BinOp::Shr | BinOp::Sar) {
            // A shift sets SF and ZF from its own result. Keeping the flags of an older
            // instruction would print a condition that does not hold (`shr ecx, 1; jne`
            // would test the previous `add`). Rotates leave SF/ZF untouched.
            let zero = self.constant(0);
            let lhs = match written_register_view(insn) {
                Some(view) => self.reg(view),
                None => result,
            };
            self.last_cmp = Some(CmpState {
                lhs,
                rhs: zero,
                source: CmpSource::ZeroOnly,
            });
        }
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
        // SF/!SF are exact signed comparisons with zero for every tracked flag source
        // (`cmp`/`sub` set SF from the result, `test`/bitwise/inc/dec set it from theirs).
        // Overflow/parity are not modelled yet: keep them as assembly rather than print
        // a wrong `/* overflow */ 0` placeholder.
        let kind = match kind {
            CondKind::Sign => CondKind::SignedLt,
            CondKind::NotSign => CondKind::SignedGe,
            CondKind::Overflow | CondKind::NotOverflow | CondKind::Parity | CondKind::NotParity => {
                return None;
            }
            other => other,
        };
        let condition = match state.source {
            CmpSource::Arithmetic => self.push(Expr::Cmp {
                kind,
                lhs: state.lhs,
                rhs: state.rhs,
            }),
            CmpSource::ZeroOnly => {
                if !uses_zero_only(kind) {
                    return None;
                }
                self.push(Expr::Cmp {
                    kind,
                    lhs: state.lhs,
                    rhs: state.rhs,
                })
            }
            CmpSource::BitTest => {
                // `bt` defines only CF: CF = (value >> bit) & 1. Only CF-based
                // conditions (jb/jae) are meaningful afterwards.
                if !matches!(kind, CondKind::UnsignedLt | CondKind::UnsignedGe) {
                    return None;
                }
                let shifted = self.push(Expr::Bin {
                    op: BinOp::Shr,
                    lhs: state.lhs,
                    rhs: state.rhs,
                });
                let one = self.constant(1);
                let masked = self.push(Expr::Bin {
                    op: BinOp::And,
                    lhs: shifted,
                    rhs: one,
                });
                let zero = self.constant(0);
                let cmp_kind = if matches!(kind, CondKind::UnsignedLt) {
                    CondKind::Ne
                } else {
                    CondKind::Eq
                };
                self.push(Expr::Cmp {
                    kind: cmp_kind,
                    lhs: masked,
                    rhs: zero,
                })
            }
            CmpSource::Negate => {
                // `neg` sets CF from the operand being non-zero, while ZF/SF describe the
                // result: `jb`/`jae` mean "operand was non-zero", not "result < 0".
                let kind = match kind {
                    CondKind::UnsignedLt => CondKind::Ne,
                    CondKind::UnsignedGe => CondKind::Eq,
                    other => other,
                };
                self.push(Expr::Cmp {
                    kind,
                    lhs: state.lhs,
                    rhs: state.rhs,
                })
            }
        };
        if consume {
            self.last_cmp = None;
        }
        Some(condition)
    }

    /// `bt` only updates flags (CF = tested bit); the consuming branch rebuilds the bit
    /// test as an expression, so no statement is emitted here.
    fn lift_bt(&mut self, insn: &Insn) {
        match (self.read_operand(insn, 0), self.read_operand(insn, 1)) {
            (Some(lhs), Some(rhs)) => {
                self.last_cmp = Some(CmpState {
                    lhs,
                    rhs,
                    source: CmpSource::BitTest,
                });
            }
            _ => self.last_cmp = None,
        }
    }

    /// `bts`/`btr`/`btc`: set, reset or complement one bit of the destination. The flag
    /// definition (CF = the old bit) is kept so a later `jb`/`jae` can still read it.
    /// `cdqe`: sign-extend `eax` into `rax` (the 64-bit form of `movsxd rax, eax`).
    fn lift_cdqe(&mut self, stmts: &mut Vec<Stmt>) -> Flow {
        let source = self.reg(RegView::new(Reg::Rax, 4, 0));
        let value = self.push(Expr::Cast {
            arg: source,
            width: 8,
            signed: true,
        });
        let reg = RegView::full(Reg::Rax);
        stmts.push(Stmt::WriteReg { reg, value });
        self.reg_env.insert(Reg::Rax, value);
        Flow::Fallthrough
    }

    fn lift_cdq(&mut self, insn: &Insn, stmts: &mut Vec<Stmt>) -> Flow {
        use iced_x86::Mnemonic as M;
        let width = match insn.mnemonic() {
            M::Cqo => 8u8,
            M::Cwd => 2,
            _ => 4,
        };
        let source = self.reg(RegView::new(Reg::Rax, width, 0));
        let zero = self.constant(0);
        let negative = self.push(Expr::Cmp {
            kind: CondKind::SignedLt,
            lhs: source,
            rhs: zero,
        });
        let minus_one = self.constant(u64::MAX);
        let zero = self.constant(0);
        let value = self.push(Expr::Select {
            cond: negative,
            then: minus_one,
            otherwise: zero,
        });
        stmts.push(Stmt::WriteReg {
            reg: RegView::new(Reg::Rdx, width, 0),
            value,
        });
        self.reg_env.insert(Reg::Rdx, value);
        Flow::Fallthrough
    }

    /// `cmpxchg dst, src` compares `dst` with the accumulator (al/ax/eax/rax) and leaves a
    /// full comparison in the flags: `je` right after it means "the exchange happened".
    fn lift_cmpxchg_flags(&mut self, insn: &Insn) {
        let Some(destination) = self.read_operand(insn, 0) else {
            self.last_cmp = None;
            return;
        };
        let size = match insn.op_kind(0) {
            iced_x86::OpKind::Register => insn.op_register(0).size(),
            _ => memory_size_bytes(insn).map_or(8, usize::from),
        };
        let accumulator = self.reg(RegView::new(Reg::Rax, size as u8, 0));
        self.last_cmp = Some(CmpState {
            lhs: destination,
            rhs: accumulator,
            source: CmpSource::Arithmetic,
        });
    }

    /// `sbb x, x` materialises the carry flag as `0`/`-1`: `x = CF ? -1 : 0`. The two-operand
    /// form with different operands subtracts with borrow and stays assembly.
    fn lift_sbb(&mut self, insn: &Insn, stmts: &mut Vec<Stmt>) -> Flow {
        let (Some(destination), Some(source)) =
            (self.read_operand(insn, 0), self.read_operand(insn, 1))
        else {
            return self.intrinsic(insn, stmts);
        };
        if !matches!(
            (&self.exprs[destination as usize], &self.exprs[source as usize]),
            (Expr::Reg(lhs), Expr::Reg(rhs)) if lhs == rhs
        ) {
            return self.intrinsic(insn, stmts);
        }
        let Some(condition) = self.condition_expr(CondKind::UnsignedLt, false) else {
            return self.intrinsic(insn, stmts);
        };
        let minus_one = self.constant(u64::MAX);
        let zero = self.constant(0);
        let value = self.push(Expr::Select {
            cond: condition,
            then: minus_one,
            otherwise: zero,
        });
        if !self.write_operand(insn, 0, value, stmts) {
            return self.intrinsic(insn, stmts);
        }
        // The carry flag has been consumed into the value.
        self.last_cmp = None;
        Flow::Fallthrough
    }

    fn lift_bit_modify(&mut self, insn: &Insn, stmts: &mut Vec<Stmt>) -> Flow {
        use iced_x86::Mnemonic as M;
        let (Some(target), Some(bit)) = (self.read_operand(insn, 0), self.read_operand(insn, 1))
        else {
            return self.intrinsic(insn, stmts);
        };
        let width_bits = match insn.op_kind(0) {
            iced_x86::OpKind::Register => (insn.op_register(0).size() * 8) as u64,
            _ => memory_size_bytes(insn).map_or(64, |bytes| u64::from(bytes) * 8),
        };
        // The CPU masks the bit index to the operand width; a constant index is folded at
        // lift time so the mask prints as one plain constant.
        let constant_bit = match &self.exprs[bit as usize] {
            Expr::Const(value) => Some(*value & (width_bits - 1)),
            _ => None,
        };
        let bit = match constant_bit {
            Some(value) => self.constant(value),
            None => {
                let mask = self.constant(width_bits - 1);
                self.push(Expr::Bin {
                    op: BinOp::And,
                    lhs: bit,
                    rhs: mask,
                })
            }
        };
        let shifted = match constant_bit {
            Some(value) => self.constant(1u64 << value),
            None => {
                let one = self.constant(1);
                self.push(Expr::Bin {
                    op: BinOp::Shl,
                    lhs: one,
                    rhs: bit,
                })
            }
        };
        let updated = match insn.mnemonic() {
            M::Bts => self.push(Expr::Bin {
                op: BinOp::Or,
                lhs: target,
                rhs: shifted,
            }),
            M::Btr => {
                let inverted = match constant_bit {
                    Some(value) => {
                        let ones = if width_bits == 64 {
                            u64::MAX
                        } else {
                            (1u64 << width_bits) - 1
                        };
                        self.constant(ones ^ (1u64 << value))
                    }
                    None => self.push(Expr::Un {
                        op: UnOp::Not,
                        arg: shifted,
                    }),
                };
                self.push(Expr::Bin {
                    op: BinOp::And,
                    lhs: target,
                    rhs: inverted,
                })
            }
            _ => self.push(Expr::Bin {
                op: BinOp::Xor,
                lhs: target,
                rhs: shifted,
            }),
        };
        self.last_cmp = Some(CmpState {
            lhs: target,
            rhs: bit,
            source: CmpSource::BitTest,
        });
        if self.write_operand(insn, 0, updated, stmts) {
            Flow::Fallthrough
        } else {
            self.intrinsic(insn, stmts)
        }
    }

    fn lift_imul(&mut self, insn: &Insn, stmts: &mut Vec<Stmt>) -> Flow {
        // 2-operand: dst = dst * src; 3-operand: dst = src * imm. The one-operand form
        // writes rdx:rax; the 32-bit form fits a 64-bit intermediate and is lifted below,
        // larger widths stay an intrinsic (no portable 128-bit product).
        let (lhs, rhs) = match insn.op_count() {
            1 => return self.lift_imul_one(insn, stmts),
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

    /// `imul r/m32`: signed `edx:eax = eax * src`. The full product fits in `i64`, so the
    /// high half is a shift instead of an intrinsic.
    fn lift_imul_one(&mut self, insn: &Insn, stmts: &mut Vec<Stmt>) -> Flow {
        use iced_x86::OpKind;
        let size = match insn.op_kind(0) {
            OpKind::Register => from_iced(insn.op_register(0)).map(|view| view.width),
            OpKind::Memory => memory_size_bytes(insn),
            _ => None,
        };
        let Some(size) = size else {
            return self.intrinsic(insn, stmts);
        };
        if size != 4 {
            return self.intrinsic(insn, stmts);
        }
        let Some(source) = self.read_operand(insn, 0) else {
            return self.intrinsic(insn, stmts);
        };
        let eax = self.reg(RegView::new(Reg::Rax, 4, 0));
        // Sign-extend through int32 explicitly: the register's C type is unsigned, so a
        // direct (int64) cast would zero-extend a negative operand.
        let eax32 = self.push(Expr::Cast {
            arg: eax,
            width: 4,
            signed: true,
        });
        let eax64 = self.push(Expr::Cast {
            arg: eax32,
            width: 8,
            signed: true,
        });
        let source32 = self.push(Expr::Cast {
            arg: source,
            width: 4,
            signed: true,
        });
        let source64 = self.push(Expr::Cast {
            arg: source32,
            width: 8,
            signed: true,
        });
        let product = self.push(Expr::Bin {
            op: BinOp::Mul,
            lhs: eax64,
            rhs: source64,
        });
        let shift = self.constant(32);
        let high_bits = self.push(Expr::Bin {
            op: BinOp::Shr,
            lhs: product,
            rhs: shift,
        });
        let low = self.push(Expr::Cast {
            arg: product,
            width: 4,
            signed: false,
        });
        let high = self.push(Expr::Cast {
            arg: high_bits,
            width: 4,
            signed: false,
        });
        // The high half must be computed before eax is overwritten.
        stmts.push(Stmt::WriteReg {
            reg: RegView::new(Reg::Rdx, 4, 0),
            value: high,
        });
        stmts.push(Stmt::WriteReg {
            reg: RegView::new(Reg::Rax, 4, 0),
            value: low,
        });
        self.reg_env.insert(Reg::Rax, low);
        self.reg_env.insert(Reg::Rdx, high);
        self.last_cmp = None;
        Flow::Fallthrough
    }

    /// `xadd dst, src`: `src = dst; dst = dst + src`, with the add's flags. For a register
    /// destination the old value is recovered as `dst - src` after the write; for memory the
    /// same expression re-reads the updated slot. No temporary is needed.
    fn lift_xadd(&mut self, insn: &Insn, stmts: &mut Vec<Stmt>) -> Flow {
        use iced_x86::OpKind;
        let (Some(destination), Some(source)) =
            (self.read_operand(insn, 0), self.read_operand(insn, 1))
        else {
            return self.intrinsic(insn, stmts);
        };
        let sum = self.push(Expr::Bin {
            op: BinOp::Add,
            lhs: destination,
            rhs: source,
        });
        if !self.write_operand(insn, 0, sum, stmts) {
            return self.intrinsic(insn, stmts);
        }
        // Read the destination as it is *after* the store: a register holds the sum, a
        // memory slot must be reloaded (the old value is the new value minus the source).
        let after = match insn.op_kind(0) {
            OpKind::Register => match from_iced(insn.op_register(0)) {
                Some(view) => self.reg(view),
                None => return self.intrinsic(insn, stmts),
            },
            OpKind::Memory => {
                let (Some(addr), Some(size)) = (self.memory_address(insn), memory_size_bytes(insn))
                else {
                    return self.intrinsic(insn, stmts);
                };
                self.push(Expr::Load { addr, size })
            }
            _ => return self.intrinsic(insn, stmts),
        };
        let old = self.push(Expr::Bin {
            op: BinOp::Sub,
            lhs: after,
            rhs: source,
        });
        if !self.write_operand(insn, 1, old, stmts) {
            return self.intrinsic(insn, stmts);
        }
        // The add's zero/sign flags describe the new destination, exactly like `add`.
        let zero = self.constant(0);
        self.last_cmp = Some(CmpState {
            lhs: after,
            rhs: zero,
            source: CmpSource::ZeroOnly,
        });
        Flow::Fallthrough
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

    /// `setcc r/m8`: write 0/1 from the tracked flag comparison (does not consume flags).
    fn lift_setcc(&mut self, insn: &Insn, kind: CondKind, stmts: &mut Vec<Stmt>) -> Flow {
        let Some(condition) = self.condition_expr(kind, false) else {
            return self.intrinsic(insn, stmts);
        };
        let byte = self.push(Expr::Cast {
            arg: condition,
            width: 1,
            signed: false,
        });
        if self.write_operand(insn, 0, byte, stmts) {
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

fn set_condition_of(mnemonic: iced_x86::Mnemonic) -> Option<CondKind> {
    use iced_x86::Mnemonic as M;
    Some(match mnemonic {
        M::Sete => CondKind::Eq,
        M::Setne => CondKind::Ne,
        M::Seta => CondKind::UnsignedGt,
        M::Setae => CondKind::UnsignedGe,
        M::Setb => CondKind::UnsignedLt,
        M::Setbe => CondKind::UnsignedLe,
        M::Setg => CondKind::SignedGt,
        M::Setge => CondKind::SignedGe,
        M::Setl => CondKind::SignedLt,
        M::Setle => CondKind::SignedLe,
        M::Sets => CondKind::Sign,
        M::Setns => CondKind::NotSign,
        M::Seto => CondKind::Overflow,
        M::Setno => CondKind::NotOverflow,
        M::Setp => CondKind::Parity,
        M::Setnp => CondKind::NotParity,
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
    let size = insn.memory_size().size();
    match size {
        1 | 2 | 4 | 8 | 16 => Some(size as u8),
        _ => None,
    }
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
    fn endbr64_lifts_to_nothing() {
        // f3 0f 1e fa: the CET landing pad Ubuntu's gcc puts at every entry.
        let insn = recraft_decode::decode(
            &[0xf3, 0x0f, 0x1e, 0xfa],
            Address::new(0x1000),
            recraft_decode::Mode::X64,
        )
        .expect("endbr64 decoded");
        let mut lifter = Lifter::new();
        let item = lifter.lift(&insn);
        assert!(matches!(item.flow, Flow::Fallthrough));
        assert!(item.stmts.is_empty(), "endbr64 must not produce statements");
    }

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
    fn sbb_with_itself_materialises_the_carry_flag() {
        // cmp eax, 5; sbb ecx, ecx — lift both, the state carries the comparison.
        let cmp = recraft_decode::decode(
            &[0x83, 0xf8, 0x05],
            Address::new(0x1000),
            recraft_decode::Mode::X64,
        )
        .expect("decoded");
        let sbb = recraft_decode::decode(
            &[0x19, 0xc9],
            Address::new(0x1003),
            recraft_decode::Mode::X64,
        )
        .expect("decoded");
        let mut lifter = Lifter::new();
        lifter.lift(&cmp);
        let item = lifter.lift(&sbb);
        let Some(Stmt::WriteReg { reg, value }) = item.stmts.last() else {
            panic!("unexpected stmts {:?}", item.stmts);
        };
        assert_eq!(reg.name(), "ecx");
        assert!(matches!(lifter.expr(*value), Expr::Select { .. }));

        // sbb edx, eax (different operands) stays assembly.
        let (_, item) = lift_one(&[0x19, 0xc2], 0x1000);
        assert!(matches!(item.stmts.first(), Some(Stmt::Intrinsic { .. })));
    }

    #[test]
    fn bit_scan_instructions_become_unary_expressions() {
        // bsf eax, ecx -> eax = ctz(ecx)
        let (lifter, item) = lift_one(&[0x0f, 0xbc, 0xc1], 0x1000);
        match &item.stmts[0] {
            Stmt::WriteReg { reg, value } => {
                assert_eq!(reg.name(), "eax");
                assert!(matches!(
                    lifter.expr(*value),
                    Expr::Un { op: UnOp::Ctz, .. }
                ));
            }
            other => panic!("unexpected {other:?}"),
        }
        // popcnt eax, ecx -> eax = popcnt(ecx)
        let (lifter, item) = lift_one(&[0xf3, 0x0f, 0xb8, 0xc1], 0x1000);
        match &item.stmts[0] {
            Stmt::WriteReg { value, .. } => {
                assert!(matches!(
                    lifter.expr(*value),
                    Expr::Un {
                        op: UnOp::Popcount,
                        ..
                    }
                ));
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn cdq_writes_the_sign_of_eax_to_edx() {
        // cdq
        let (lifter, item) = lift_one(&[0x99], 0x1000);
        match &item.stmts[0] {
            Stmt::WriteReg { reg, value } => {
                assert_eq!(reg.name(), "edx");
                assert!(matches!(
                    lifter.expr(*value),
                    Expr::Select {
                        cond,
                        ..
                    } if matches!(lifter.expr(*cond), Expr::Cmp { kind: CondKind::SignedLt, .. })
                ));
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn cdqe_sign_extends_eax() {
        // cdqe
        let (lifter, item) = lift_one(&[0x48, 0x98], 0x1000);
        match &item.stmts[0] {
            Stmt::WriteReg { reg, value } => {
                assert_eq!(reg.name(), "rax");
                assert!(matches!(
                    lifter.expr(*value),
                    Expr::Cast {
                        width: 8,
                        signed: true,
                        ..
                    }
                ));
            }
            other => panic!("unexpected stmt {other:?}"),
        }
    }

    #[test]
    fn bit_set_and_reset_lift_to_masks() {
        // bts eax, 0x1c
        let (lifter, item) = lift_one(&[0x0f, 0xba, 0xe8, 0x1c], 0x1000);
        match &item.stmts[0] {
            Stmt::WriteReg { value, .. } => {
                assert!(matches!(
                    lifter.expr(*value),
                    Expr::Bin { op: BinOp::Or, .. }
                ));
            }
            other => panic!("unexpected stmt {other:?}"),
        }
        // btr edx, 0x1c
        let (lifter, item) = lift_one(&[0x0f, 0xba, 0xf2, 0x1c], 0x1000);
        match &item.stmts[0] {
            Stmt::WriteReg { value, .. } => {
                assert!(matches!(
                    lifter.expr(*value),
                    Expr::Bin { op: BinOp::And, .. }
                ));
            }
            other => panic!("unexpected stmt {other:?}"),
        }
        // btc ecx, 3
        let (lifter, item) = lift_one(&[0x0f, 0xba, 0xf9, 0x03], 0x1000);
        match &item.stmts[0] {
            Stmt::WriteReg { value, .. } => {
                assert!(matches!(
                    lifter.expr(*value),
                    Expr::Bin { op: BinOp::Xor, .. }
                ));
            }
            other => panic!("unexpected stmt {other:?}"),
        }
    }

    #[test]
    fn one_operand_imul_writes_both_halves() {
        // imul ecx: edx:eax = eax * ecx
        let (lifter, item) = lift_one(&[0xf7, 0xe9], 0x1000);
        assert_eq!(item.stmts.len(), 2);
        match &item.stmts[0] {
            Stmt::WriteReg { reg, value } => {
                assert_eq!(reg.base, Reg::Rdx);
                assert_eq!(reg.width, 4);
                assert!(matches!(lifter.expr(*value), Expr::Cast { .. }));
            }
            other => panic!("unexpected stmt {other:?}"),
        }
        match &item.stmts[1] {
            Stmt::WriteReg { reg, .. } => {
                assert_eq!(reg.base, Reg::Rax);
                assert_eq!(reg.width, 4);
            }
            other => panic!("unexpected stmt {other:?}"),
        }
    }

    #[test]
    fn xadd_lifts_to_add_and_old_value() {
        // xadd edx, ecx (ModRM CA: reg=ecx, rm=edx)
        let (lifter, item) = lift_one(&[0x0f, 0xc1, 0xca], 0x1000);
        assert_eq!(item.stmts.len(), 2);
        match &item.stmts[0] {
            Stmt::WriteReg { reg, value } => {
                assert_eq!(reg.base, Reg::Rdx);
                assert!(matches!(
                    lifter.expr(*value),
                    Expr::Bin { op: BinOp::Add, .. }
                ));
            }
            other => panic!("unexpected stmt {other:?}"),
        }
        match &item.stmts[1] {
            Stmt::WriteReg { reg, value } => {
                assert_eq!(reg.base, Reg::Rcx);
                match lifter.expr(*value) {
                    Expr::Bin {
                        op: BinOp::Sub,
                        lhs,
                        ..
                    } => assert!(
                        matches!(lifter.expr(*lhs), Expr::Reg(view) if view.base == Reg::Rdx)
                    ),
                    other => panic!("unexpected old value {other:?}"),
                }
            }
            other => panic!("unexpected stmt {other:?}"),
        }
    }

    #[test]
    fn locked_xadd_memory_reloads_the_slot() {
        // lock xadd [0x1000], ecx
        let (lifter, item) = lift_one(&[0xf0, 0x0f, 0xc1, 0x0d, 0x00, 0x10, 0x00, 0x00], 0x1000);
        assert_eq!(item.stmts.len(), 2);
        match &item.stmts[1] {
            Stmt::WriteReg { reg, value } => {
                assert_eq!(reg.base, Reg::Rcx);
                match lifter.expr(*value) {
                    Expr::Bin {
                        op: BinOp::Sub,
                        lhs,
                        ..
                    } => assert!(matches!(lifter.expr(*lhs), Expr::Load { .. })),
                    other => panic!("unexpected old value {other:?}"),
                }
            }
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
    fn setcc_uses_tracked_flags() {
        // cmp eax, ecx ; sete al
        let cmp = recraft_decode::decode(
            &[0x39, 0xc8],
            recraft_common::Address::new(0x1000),
            recraft_decode::Mode::X64,
        )
        .expect("cmp decodes");
        let setcc = recraft_decode::decode(
            &[0x0f, 0x94, 0xc0],
            recraft_common::Address::new(0x1002),
            recraft_decode::Mode::X64,
        )
        .expect("sete decodes");
        let mut lifter = Lifter::new();
        lifter.lift(&cmp);
        let item = lifter.lift(&setcc);
        assert!(
            matches!(item.stmts[0], Stmt::WriteReg { .. }),
            "sete should write a register: {:?}",
            item.stmts
        );
    }

    #[test]
    fn sign_flags_feed_branches() {
        // add eax, ecx ; js rel8
        let add = recraft_decode::decode(
            &[0x01, 0xc8],
            recraft_common::Address::new(0x1000),
            recraft_decode::Mode::X64,
        )
        .expect("add decodes");
        let js = recraft_decode::decode(
            &[0x78, 0x10],
            recraft_common::Address::new(0x1002),
            recraft_decode::Mode::X64,
        )
        .expect("js decodes");
        let mut lifter = Lifter::new();
        lifter.lift(&add);
        let branch = lifter.lift(&js);
        assert!(
            matches!(branch.flow, Flow::CondGoto { .. }),
            "add/js should produce a real condition: {:?}",
            branch.flow
        );
        assert!(
            branch.stmts.is_empty(),
            "unexpected intrinsic: {:?}",
            branch.stmts
        );
    }

    #[test]
    fn bit_test_flags_feed_branches() {
        // bt rcx, rax ; jae rel8
        let bt = recraft_decode::decode(
            &[0x48, 0x0f, 0xa3, 0xc1],
            recraft_common::Address::new(0x1000),
            recraft_decode::Mode::X64,
        )
        .expect("bt decodes");
        let jae = recraft_decode::decode(
            &[0x73, 0x14],
            recraft_common::Address::new(0x1004),
            recraft_decode::Mode::X64,
        )
        .expect("jae decodes");
        let mut lifter = Lifter::new();
        assert!(lifter.lift(&bt).stmts.is_empty());
        let branch = lifter.lift(&jae);
        assert!(
            matches!(branch.flow, Flow::CondGoto { .. }),
            "bt/jae should produce a real condition: {:?}",
            branch.flow
        );
        assert!(
            branch.stmts.is_empty(),
            "unexpected intrinsic: {:?}",
            branch.stmts
        );
    }

    #[test]
    fn indirect_calls_are_lifted() {
        // call qword ptr [rip+0]  (IAT-style indirect call)
        let (_, call) = lift_one(&[0xff, 0x15, 0x00, 0x00, 0x00, 0x00], 0x1000);
        assert!(
            matches!(call.stmts[0], Stmt::CallIndirect { .. }),
            "unexpected: {:?}",
            call.stmts
        );
        assert_eq!(call.flow, Flow::Fallthrough);
        // jmp qword ptr [rip+0]  (tail call through an IAT slot)
        let (_, tail) = lift_one(&[0xff, 0x25, 0x00, 0x00, 0x00, 0x00], 0x1000);
        assert!(
            matches!(tail.stmts[0], Stmt::CallIndirect { .. }),
            "unexpected: {:?}",
            tail.stmts
        );
        assert_eq!(tail.flow, Flow::Return);
    }

    #[test]
    fn unaligned_simd_moves_are_lifted() {
        let insn = recraft_decode::decode(
            &[0x0f, 0x11, 0x02],
            recraft_common::Address::new(0x1000),
            recraft_decode::Mode::X64,
        )
        .unwrap();
        eprintln!(
            "debug: mnemonic={:?} size={:?} seg={:?} base={:?} mem_size_bytes={:?}",
            insn.mnemonic(),
            insn.memory_size(),
            insn.memory_segment(),
            insn.memory_base(),
            memory_size_bytes(&insn)
        );
        // movups [rdx], xmm0
        let (_, store) = lift_one(&[0x0f, 0x11, 0x02], 0x1000);
        assert!(
            matches!(store.stmts[0], Stmt::Store { size: 16, .. }),
            "unexpected: {:?}",
            store.stmts
        );
        // movups xmm0, [rdx]
        let (_, load) = lift_one(&[0x0f, 0x10, 0x02], 0x1000);
        assert!(
            matches!(load.stmts[0], Stmt::WriteReg { ref reg, .. } if reg.name() == "xmm0"),
            "unexpected: {:?}",
            load.stmts
        );
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
