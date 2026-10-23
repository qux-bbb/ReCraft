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

/// One field of a user-defined struct layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructField {
    /// Byte offset from the start of the struct.
    pub offset: u64,
    pub name: String,
    /// C-ish field type: `u8`/`i8`/`u16`/`i16`/`u32`/`i32`/`u64`/`i64`/`f32`/`f64`/
    /// `ptr`/`char`/`wchar`.
    pub kind: String,
}

/// A user-defined struct layout (`Command::DefineStruct`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructDef {
    pub name: String,
    pub size: u64,
    /// Sorted by offset, unique.
    pub fields: Vec<StructField>,
}

impl StructDef {
    /// Validate identifier, size and field list; normalises field order.
    pub fn validate(mut self) -> Result<Self> {
        if self.name.is_empty()
            || !self.name.chars().enumerate().all(|(index, ch)| {
                ch == '_' || ch.is_ascii_alphanumeric() && (index > 0 || !ch.is_ascii_digit())
            })
        {
            return Err(Error::internal(format!(
                "invalid struct name: {:?}",
                self.name
            )));
        }
        if self.size == 0 {
            return Err(Error::internal("struct size must be positive"));
        }
        self.fields.sort_by_key(|field| field.offset);
        for pair in self.fields.windows(2) {
            if pair[0].offset == pair[1].offset {
                return Err(Error::internal(format!(
                    "duplicate field offset {:#x}",
                    pair[0].offset
                )));
            }
        }
        for field in &self.fields {
            if field.offset >= self.size {
                return Err(Error::internal(format!(
                    "field {:?} offset {:#x} outside struct size {:#x}",
                    field.name, field.offset, self.size
                )));
            }
            if field.name.is_empty() {
                return Err(Error::internal("empty struct field name"));
            }
            if !matches!(
                field.kind.as_str(),
                "u8" | "i8"
                    | "u16"
                    | "i16"
                    | "u32"
                    | "i32"
                    | "u64"
                    | "i64"
                    | "f32"
                    | "f64"
                    | "ptr"
                    | "char"
                    | "wchar"
            ) {
                return Err(Error::internal(format!(
                    "unsupported field type {:?} (use u8/i8/u16/i16/u32/i32/u64/i64/f32/f64/ptr/char/wchar)",
                    field.kind
                )));
            }
        }
        Ok(self)
    }

    pub fn field_at(&self, offset: u64) -> Option<&StructField> {
        self.fields.iter().find(|field| field.offset == offset)
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
    /// Set (`Some`) or delete (`None`) a user prototype (C declaration) for a function.
    SetPrototype { addr: Address, text: Option<String> },
    /// Define or replace a user struct layout.
    DefineStruct { def: StructDef },
    /// Internal: delete a user struct layout (undo support).
    DeleteStruct { name: String },
    /// Bind a struct layout to a function argument slot (`0` = first argument).
    ApplyType {
        func: Address,
        slot: u8,
        struct_name: Option<String>,
    },
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
            Command::SetPrototype { .. } => "prototype",
            Command::DefineStruct { .. } => "define_struct",
            Command::DeleteStruct { .. } => "delete_struct",
            Command::ApplyType { .. } => "apply_type",
        }
    }

    /// Ranges the UI should refresh after this command (or its inverse) is applied.
    pub fn effect(&self) -> Effect {
        let dirty = match self {
            Command::Rename { addr, .. }
            | Command::UndefineFunc { start: addr }
            | Command::SetPrototype { addr, .. }
            | Command::ApplyType { func: addr, .. } => {
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
            Command::DefineStruct { .. } | Command::DeleteStruct { .. } => Vec::new(),
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
            Command::SetPrototype { addr, .. } => Ok(Command::SetPrototype {
                addr: *addr,
                text: session.prototype_at(*addr)?,
            }),
            Command::DefineStruct { def } => match session.struct_def(&def.name)? {
                Some(previous) => Ok(Command::DefineStruct { def: previous }),
                None => Ok(Command::DeleteStruct {
                    name: def.name.clone(),
                }),
            },
            Command::DeleteStruct { name } => match session.struct_def(name)? {
                Some(previous) => Ok(Command::DefineStruct { def: previous }),
                None => Ok(Command::DeleteStruct { name: name.clone() }),
            },
            Command::ApplyType { func, slot, .. } => Ok(Command::ApplyType {
                func: *func,
                slot: *slot,
                struct_name: session.arg_type(*func, *slot)?,
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
        Command::SetPrototype { addr, text } => match text {
            Some(text) => {
                conn.execute(
                    "INSERT INTO prototype(addr, text) VALUES (?1, ?2)
                     ON CONFLICT(addr) DO UPDATE SET text = excluded.text",
                    params![addr.get() as i64, text],
                )
                .map_err(db_err)?;
            }
            None => {
                conn.execute("DELETE FROM prototype WHERE addr = ?1", [addr.get() as i64])
                    .map_err(db_err)?;
            }
        },
        Command::DefineStruct { def } => {
            let def = def.clone().validate()?;
            conn.execute(
                "INSERT INTO user_struct(name, size) VALUES (?1, ?2)
                 ON CONFLICT(name) DO UPDATE SET size = excluded.size",
                params![def.name, def.size as i64],
            )
            .map_err(db_err)?;
            conn.execute(
                "DELETE FROM user_struct_field WHERE struct_name = ?1",
                [def.name.as_str()],
            )
            .map_err(db_err)?;
            for field in &def.fields {
                conn.execute(
                    "INSERT INTO user_struct_field(struct_name, off, name, kind)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![def.name, field.offset as i64, field.name, field.kind],
                )
                .map_err(db_err)?;
            }
        }
        Command::DeleteStruct { name } => {
            conn.execute("DELETE FROM user_struct WHERE name = ?1", [name.as_str()])
                .map_err(db_err)?;
            conn.execute(
                "DELETE FROM user_struct_field WHERE struct_name = ?1",
                [name.as_str()],
            )
            .map_err(db_err)?;
            conn.execute(
                "DELETE FROM func_arg_type WHERE struct_name = ?1",
                [name.as_str()],
            )
            .map_err(db_err)?;
        }
        Command::ApplyType {
            func,
            slot,
            struct_name,
        } => match struct_name {
            Some(struct_name) => {
                let exists: bool = conn
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM user_struct WHERE name = ?1)",
                        [struct_name.as_str()],
                        |row| row.get(0),
                    )
                    .map_err(db_err)?;
                if !exists {
                    return Err(Error::internal(format!(
                        "unknown struct type {struct_name:?}; define it first"
                    )));
                }
                conn.execute(
                    "INSERT INTO func_arg_type(func_addr, slot, struct_name)
                     VALUES (?1, ?2, ?3)
                     ON CONFLICT(func_addr, slot) DO UPDATE SET struct_name = excluded.struct_name",
                    params![func.get() as i64, *slot as i64, struct_name],
                )
                .map_err(db_err)?;
            }
            None => {
                conn.execute(
                    "DELETE FROM func_arg_type WHERE func_addr = ?1 AND slot = ?2",
                    params![func.get() as i64, *slot as i64],
                )
                .map_err(db_err)?;
            }
        },
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
