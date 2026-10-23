// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Low-level IR (LIR): expression trees in an arena, plus straight-line statements.
//!
//! M0 scope: enough to print readable pseudo-C for straight-line code and simple
//! branches. The IR is intentionally flat: control flow is carried by [`Flow`] next to
//! each lifted instruction, not by nested statements (structuring is L1 work).

use recraft_common::Address;
use serde::{Deserialize, Serialize};

use crate::reg::RegView;

/// Index into [`Lifted::exprs`].
pub type ExprId = u32;

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    /// Scalar float arithmetic (same C operators, distinct for type-aware printing).
    FAdd,
    FSub,
    FMul,
    FDiv,
    And,
    Or,
    Xor,
    Shl,
    Shr,
    Sar,
}

impl BinOp {
    pub const fn symbol(self) -> &'static str {
        match self {
            BinOp::Add | BinOp::FAdd => "+",
            BinOp::Sub | BinOp::FSub => "-",
            BinOp::Mul | BinOp::FMul => "*",
            BinOp::Div | BinOp::FDiv => "/",
            BinOp::And => "&",
            BinOp::Or => "|",
            BinOp::Xor => "^",
            BinOp::Shl => "<<",
            BinOp::Shr => ">>",
            BinOp::Sar => ">>",
        }
    }

    pub const fn is_float(self) -> bool {
        matches!(self, BinOp::FAdd | BinOp::FSub | BinOp::FMul | BinOp::FDiv)
    }
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnOp {
    Not,
    Neg,
}

/// Comparison predicates (also used for `jcc`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CondKind {
    Eq,
    Ne,
    UnsignedGt,
    UnsignedGe,
    UnsignedLt,
    UnsignedLe,
    SignedGt,
    SignedGe,
    SignedLt,
    SignedLe,
    Sign,
    NotSign,
    Overflow,
    NotOverflow,
    Parity,
    NotParity,
}

impl CondKind {
    /// C operator for the kinds that can be expressed as a direct comparison.
    pub const fn comparison_symbol(self) -> Option<&'static str> {
        Some(match self {
            CondKind::Eq => "==",
            CondKind::Ne => "!=",
            CondKind::UnsignedGt | CondKind::SignedGt => ">",
            CondKind::UnsignedGe | CondKind::SignedGe => ">=",
            CondKind::UnsignedLt | CondKind::SignedLt => "<",
            CondKind::UnsignedLe | CondKind::SignedLe => "<=",
            _ => return None,
        })
    }

    pub const fn is_signed(self) -> bool {
        matches!(
            self,
            CondKind::SignedGt | CondKind::SignedGe | CondKind::SignedLt | CondKind::SignedLe
        )
    }
}

/// One expression node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Expr {
    Const(u64),
    Reg(RegView),
    Temp(u32),
    Load {
        addr: ExprId,
        size: u8,
    },
    /// Scalar float load (`movss`/`movsd`/float arithmetic with a memory operand).
    LoadFloat {
        addr: ExprId,
        /// 4 = f32, 8 = f64.
        size: u8,
    },
    Bin {
        op: BinOp,
        lhs: ExprId,
        rhs: ExprId,
    },
    Un {
        op: UnOp,
        arg: ExprId,
    },
    Cast {
        arg: ExprId,
        width: u8,
        signed: bool,
    },
    Cmp {
        kind: CondKind,
        lhs: ExprId,
        rhs: ExprId,
    },
    /// `cmovcc`: `cond ? then : otherwise`. The condition comes from the lazily tracked
    /// comparison; the instruction preserves flags.
    Select {
        cond: ExprId,
        then: ExprId,
        otherwise: ExprId,
    },
}

/// One straight-line statement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stmt {
    /// Write a (possibly partial) register view.
    WriteReg { reg: RegView, value: ExprId },
    /// Store `value` at `addr`.
    Store {
        addr: ExprId,
        value: ExprId,
        size: u8,
    },
    /// Scalar float store (`movss`/`movsd`).
    StoreFloat {
        addr: ExprId,
        value: ExprId,
        /// 4 = f32, 8 = f64.
        size: u8,
    },
    /// Direct call to a known address, with the argument expressions captured at the
    /// call site (x64 register arguments; empty on x86 where arguments live on the stack).
    Call { target: Address, args: Vec<ExprId> },
    /// Instruction we do not model yet; the original assembly text is kept.
    Intrinsic { text: String },
}

/// Control transfer of one lifted instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Flow {
    Fallthrough,
    Goto(Address),
    CondGoto { cond: ExprId, target: Address },
    Return,
}

/// Result of lifting one instruction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiftedInsn {
    pub addr: Address,
    pub stmts: Vec<Stmt>,
    pub flow: Flow,
}

/// A lifted region: expression arena + instructions in address order.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Lifted {
    pub start: Address,
    pub exprs: Vec<Expr>,
    pub insns: Vec<LiftedInsn>,
}

impl Lifted {
    #[inline]
    pub fn expr(&self, id: ExprId) -> &Expr {
        &self.exprs[id as usize]
    }
}

/// Short human-readable name for a comparison (used by the printer/tests).
pub fn cond_name(kind: CondKind) -> &'static str {
    match kind {
        CondKind::Eq => "eq",
        CondKind::Ne => "ne",
        CondKind::UnsignedGt => "ugt",
        CondKind::UnsignedGe => "uge",
        CondKind::UnsignedLt => "ult",
        CondKind::UnsignedLe => "ule",
        CondKind::SignedGt => "sgt",
        CondKind::SignedGe => "sge",
        CondKind::SignedLt => "slt",
        CondKind::SignedLe => "sle",
        CondKind::Sign => "sign",
        CondKind::NotSign => "not_sign",
        CondKind::Overflow => "overflow",
        CondKind::NotOverflow => "not_overflow",
        CondKind::Parity => "parity",
        CondKind::NotParity => "not_parity",
    }
}
