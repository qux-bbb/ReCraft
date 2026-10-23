// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! User commands: the only way to mutate the program database.
//!
//! Every command is applied in one SQL transaction together with a journal record
//! that stores both the command and its inverse. Undo/redo replay those records, so
//! the journal is also the crash-recovery log for user edits.

use recraft_common::{Address, AddressRange, Error, Result};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::flags::{CodeData, Flags, FlagsStore};
use crate::segments::Segment;
use crate::session::Session;

/// Name records created by the loader (entry point, exports, imports, ...) — not user edits.
pub const NAME_KIND_LOADER: i64 = 0;
/// Name records created by the user through [`Command::Rename`].
pub const NAME_KIND_USER: i64 = 1;
/// Name records produced by automatic analysis (e.g. `sub_140001000`); derived data.
pub const NAME_KIND_AUTO: i64 = 2;

/// Which comment slot to address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommentKind {
    /// Shown on the instruction line.
    Regular = 0,
    /// Shown on every reference.
    Repeatable = 1,
    /// Shown above the instruction.
    Anterior = 2,
}

impl CommentKind {
    #[inline]
    pub const fn as_i64(self) -> i64 {
        self as i64
    }

    #[inline]
    pub const fn from_i64(value: i64) -> Self {
        match value {
            1 => CommentKind::Repeatable,
            2 => CommentKind::Anterior,
            _ => CommentKind::Regular,
        }
    }
}

/// A mutation of the program database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    /// Set (`Some`) or delete (`None`) the name at `addr`.
    Rename { addr: Address, name: Option<String> },
    /// Set (`Some`) or delete (`None`) a comment.
    SetComment {
        addr: Address,
        kind: CommentKind,
        text: Option<String>,
    },
    /// Mark a range as code or data (also used to clear back to unknown).
    SetCodeData { range: AddressRange, kind: CodeData },
    /// Internal: restore a previously captured set of per-byte flags (undo support).
    RestoreFlags { start: Address, flags: Vec<Flags> },
    /// Define a function at `start`.
    DefineFunc { start: Address, end: Address },
    /// Remove the function starting at `start`.
    UndefineFunc { start: Address },
}

impl Command {
    pub const fn name(&self) -> &'static str {
        match self {
            Command::Rename { .. } => "rename",
            Command::SetComment { .. } => "comment",
            Command::SetCodeData { .. } => "code_data",
            Command::RestoreFlags { .. } => "restore_flags",
            Command::DefineFunc { .. } => "define_func",
            Command::UndefineFunc { .. } => "undefine_func",
        }
    }

    /// Ranges the UI should refresh after this command (or its inverse) is applied.
    pub fn effect(&self) -> Effect {
        let dirty = match self {
            Command::Rename { addr, .. } | Command::UndefineFunc { start: addr } => {
                vec![AddressRange::new(*addr, Address::new(addr.get() + 1))]
            }
            Command::SetComment { addr, .. } => {
                vec![AddressRange::new(*addr, Address::new(addr.get() + 1))]
            }
            Command::SetCodeData { range, .. } => vec![*range],
            Command::RestoreFlags { start, flags } => vec![AddressRange::new(
                *start,
                Address::new(start.get() + flags.len() as u64),
            )],
            Command::DefineFunc { start, end } => vec![AddressRange::new(*start, *end)],
        };
        Effect { dirty }
    }

    /// Build the command that undoes `self`, based on the current database state.
    pub fn inverse(&self, session: &mut Session) -> Result<Command> {
        match self {
            Command::Rename { addr, .. } => Ok(Command::Rename {
                addr: *addr,
                name: session.name_at(*addr)?,
            }),
            Command::SetComment { addr, kind, .. } => Ok(Command::SetComment {
                addr: *addr,
                kind: *kind,
                text: session.comment_at(*addr, *kind)?,
            }),
            Command::SetCodeData { range, .. } => {
                // One page max keeps the inverse small; UI commands are per-selection.
                if range.len() > crate::flags::PAGE_BYTES {
                    return Err(Error::internal(format!(
                        "code/data command range too large: {:#x} bytes",
                        range.len()
                    )));
                }
                Ok(Command::RestoreFlags {
                    start: range.start,
                    flags: session.snapshot_flags(*range)?,
                })
            }
            Command::RestoreFlags { start, flags } => Ok(Command::RestoreFlags {
                start: *start,
                flags: session.snapshot_flags(AddressRange::new(
                    *start,
                    Address::new(start.get() + flags.len() as u64),
                ))?,
            }),
            Command::DefineFunc { start, .. } => Ok(Command::UndefineFunc { start: *start }),
            Command::UndefineFunc { start } => match session.func_at(*start)? {
                Some(end) => Ok(Command::DefineFunc { start: *start, end }),
                None => Ok(Command::UndefineFunc { start: *start }),
            },
        }
    }
}

/// Ranges invalidated by a command.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Effect {
    pub dirty: Vec<AddressRange>,
}

/// What is stored in the journal for one command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalRecord {
    pub command: Command,
    pub inverse: Command,
}

/// Apply a command to the database. Must run inside the command's transaction.
pub(crate) fn apply(
    conn: &Connection,
    _segments: &[Segment],
    flags: &mut FlagsStore,
    command: &Command,
) -> Result<()> {
    match command {
        Command::Rename { addr, name } => match name {
            Some(name) => {
                conn.execute(
                    "INSERT INTO name(addr, text, kind, flags) VALUES (?1, ?2, ?3, 0)
                     ON CONFLICT(addr) DO UPDATE SET text = excluded.text,
                                                    kind = excluded.kind",
                    params![addr.get() as i64, name, NAME_KIND_USER],
                )
                .map_err(db_err)?;
            }
            None => {
                conn.execute("DELETE FROM name WHERE addr = ?1", [addr.get() as i64])
                    .map_err(db_err)?;
            }
        },
        Command::SetComment { addr, kind, text } => match text {
            Some(text) => {
                conn.execute(
                    "INSERT INTO comment(addr, kind, text) VALUES (?1, ?2, ?3)
                     ON CONFLICT(addr, kind) DO UPDATE SET text = excluded.text",
                    params![addr.get() as i64, kind.as_i64(), text],
                )
                .map_err(db_err)?;
            }
            None => {
                conn.execute(
                    "DELETE FROM comment WHERE addr = ?1 AND kind = ?2",
                    params![addr.get() as i64, kind.as_i64()],
                )
                .map_err(db_err)?;
            }
        },
        Command::SetCodeData { range, kind } => {
            flags.fill(conn, range.start, range.len(), *kind)?;
        }
        Command::RestoreFlags {
            start,
            flags: values,
        } => {
            flags.write(conn, *start, values)?;
        }
        Command::DefineFunc { start, end } => {
            if end <= start {
                return Err(Error::internal("function end must be after start"));
            }
            conn.execute(
                "INSERT INTO func(start, end, flags, cc) VALUES (?1, ?2, 0, 0)
                 ON CONFLICT(start) DO UPDATE SET end = excluded.end",
                params![start.get() as i64, end.get() as i64],
            )
            .map_err(db_err)?;
        }
        Command::UndefineFunc { start } => {
            conn.execute("DELETE FROM func WHERE start = ?1", [start.get() as i64])
                .map_err(db_err)?;
        }
    }
    Ok(())
}

/// Read the newest applied journal record for undo.
pub(crate) fn last_active_record(conn: &Connection) -> Result<Option<(i64, Vec<u8>)>> {
    conn.query_row(
        "SELECT seq, payload FROM journal WHERE undone = 0 ORDER BY seq DESC LIMIT 1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .optional()
    .map_err(db_err)
}

/// Read the oldest undone journal record for redo.
pub(crate) fn first_undone_record(conn: &Connection) -> Result<Option<(i64, Vec<u8>)>> {
    conn.query_row(
        "SELECT seq, payload FROM journal WHERE undone = 1 ORDER BY seq ASC LIMIT 1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .optional()
    .map_err(db_err)
}

pub(crate) fn db_err(error: rusqlite::Error) -> Error {
    Error::storage(error.to_string())
}
