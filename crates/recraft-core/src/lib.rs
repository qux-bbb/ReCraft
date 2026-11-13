// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Program database semantics: session, address space, flags, names, functions,
//! the command bus and undo.
//!
//! Part of ReCraft. See `docs/DESIGN.md` §2 (command bus, user data vs derived data).

#![forbid(unsafe_code)]

mod command;
mod flags;
pub mod plt;
mod script;
mod segments;
mod session;
mod snapshot;
mod view;

pub use command::{
    Command, CommentKind, Effect, JournalRecord, NAME_KIND_AUTO, NAME_KIND_LOADER, NAME_KIND_USER,
    StructDef, StructField,
};
pub use flags::{CodeData, Flags, FlagsStore, PAGE_BYTES};
// Loaders live in `recraft-format`; front ends and the decompiler read them through
// the session, so the vocabulary is re-exported here.
pub use plt::PltStub;
pub use recraft_format::{Arch, Format, Image, LoadedImage};
pub use script::{COMMAND_TYPES, parse_command, resolve_address};
pub use segments::{Perm, Segment, SegmentKind};
pub use session::{FUNC_FLAG_AUTO, Session, UndoRecord, UndoReport};
pub use snapshot::SessionSnapshot;
pub use view::{DisasmLine, FunctionInfo, SessionView, StringEntry, StringOverride, XrefEntry};
