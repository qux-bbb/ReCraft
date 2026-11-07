// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Program database semantics: session, address space, flags, names, functions,
//! the command bus and undo.
//!
//! Part of ReCraft. See `docs/DESIGN.md` §2 (command bus, user data vs derived data).

#![forbid(unsafe_code)]

mod command;
mod flags;
mod segments;
mod session;
mod view;

pub use command::{
    Command, CommentKind, Effect, JournalRecord, NAME_KIND_AUTO, NAME_KIND_LOADER, NAME_KIND_USER,
    StructDef, StructField,
};
pub use flags::{CodeData, Flags, FlagsStore, PAGE_BYTES};
pub use segments::{Perm, Segment, SegmentKind};
pub use session::{FUNC_FLAG_AUTO, Session, UndoRecord, UndoReport};
pub use view::{DisasmLine, FunctionInfo, StringEntry, XrefEntry};
