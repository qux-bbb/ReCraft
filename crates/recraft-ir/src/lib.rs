// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Semantic IR and x86/x64 lifting.
//!
//! M0: flat LIR (expression arena + straight-line statements) with lazy flags, enough
//! for readable L0 pseudo-C. L1 (SSA, structuring, types) will build on this arena.
//!
//! Part of ReCraft. See `docs/DESIGN.md` §7.

#![forbid(unsafe_code)]

pub mod expr;
pub mod lift;
pub mod reg;

pub use expr::{BinOp, CondKind, Expr, ExprId, Flow, Lifted, LiftedInsn, Stmt, UnOp, cond_name};
pub use lift::{CallAbi, Lifter};
pub use reg::{Reg, RegView, from_iced};
