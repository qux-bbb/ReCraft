// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Project container for ReCraft.
//!
//! A project is a single `.rcraft` file backed by SQLite (WAL). Original image bytes
//! are stored in fixed-size chunks, metadata and analysis data live in normal tables,
//! and every user edit is appended to an undo journal.
//!
//! This crate owns the **schema** and the **transaction/journal primitives**. Semantic
//! access (functions, names, xrefs) is implemented in `recraft-core` on top of
//! [`Project::conn`]; `recraft-storage` deliberately stays a thin, mechanical layer.

#![forbid(unsafe_code)]

mod project;
pub mod schema;

pub use project::{IMAGE_CHUNK_SIZE, JournalEntry, Project, ReadOnlyProject};
