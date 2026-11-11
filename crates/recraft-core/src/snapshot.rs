// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! A read-only, `Sync` copy of everything rendering reads from a project.
//!
//! Rendering only ever reads bytes, decoded instructions, names, prototypes and string
//! metadata. `Session` owns the writer connection and therefore cannot be shared, so
//! function-level work that wants several cores goes through a [`SessionSnapshot`]
//! instead: it is built once, in one pass over the project, and then answers
//! [`SessionView`] from memory (see `docs/DESIGN.md` §1, analysis pool).

use std::collections::HashMap;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use recraft_common::{Address, Result};
use recraft_decode::{Insn, Mode, decode};
use recraft_format::{Arch, Format, Image};

use crate::command::{StructDef, db_err};
use crate::segments::Segment;
use crate::session::Session;
use crate::view::{FunctionInfo, SessionView};

/// Upper bound on how many decoded instructions a snapshot keeps resident (roughly 50 MB
/// worst case). Reaching it only costs re-decodes, never correctness.
const MAX_DECODED_INSTRUCTIONS: usize = 500_000;

/// Snapshot of one session, safe to share between threads.
pub struct SessionSnapshot {
    image: Image,
    segments: Vec<Segment>,
    /// File bytes backing each segment, parallel to `segments`.
    bytes: Vec<Vec<u8>>,
    mode: Mode,
    arch: Arch,
    format: Format,
    functions: Vec<FunctionInfo>,
    names: HashMap<u64, String>,
    prototypes: HashMap<u64, String>,
    /// `addr -> (len, enc)` for every stored string.
    strings: HashMap<u64, (u32, i64)>,
    structs: HashMap<String, StructDef>,
    /// `(function, argument slot) -> struct name`.
    arg_types: HashMap<(u64, u8), String>,
    /// Decoded instructions shareable by the rendering threads. A plain map is used
    /// instead of `DecodeCache`: its LRU scan is O(cache size) per hit, which would
    /// serialize every instruction lookup of every thread. Entries are never evicted;
    /// past the cap new decodes are simply not cached.
    decoders: RwLock<HashMap<u64, Insn>>,
}

impl SessionSnapshot {
    /// Copy the read paths rendering needs out of `session`.
    ///
    /// The image segments are read once, so later lookups never touch SQLite. That is the
    /// bulk of the snapshot's memory: one copy of every file-backed segment.
    pub(crate) fn capture(session: &Session) -> Result<Self> {
        let conn = session.project().conn();
        let functions = session.functions()?;
        let names = session
            .names()?
            .into_iter()
            .map(|(addr, name)| (addr.get(), name))
            .collect();
        let structs = session
            .structs()?
            .into_iter()
            .map(|def| (def.name.clone(), def))
            .collect();

        let mut prototypes = HashMap::new();
        let mut statement = conn
            .prepare("SELECT addr, text FROM prototype")
            .map_err(db_err)?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, i64>(0)? as u64, row.get::<_, String>(1)?))
            })
            .map_err(db_err)?;
        for row in rows {
            let (addr, text) = row.map_err(db_err)?;
            prototypes.insert(addr, text);
        }

        let mut strings = HashMap::new();
        let mut statement = conn
            .prepare("SELECT addr, len, enc FROM string")
            .map_err(db_err)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)? as u64,
                    (row.get::<_, i64>(1)? as u32, row.get(2)?),
                ))
            })
            .map_err(db_err)?;
        for row in rows {
            let (addr, meta) = row.map_err(db_err)?;
            strings.insert(addr, meta);
        }

        let mut arg_types = HashMap::new();
        let mut statement = conn
            .prepare("SELECT func_addr, slot, struct_name FROM func_arg_type")
            .map_err(db_err)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)? as u64,
                    row.get::<_, i64>(1)? as u8,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(db_err)?;
        for row in rows {
            let (func, slot, name) = row.map_err(db_err)?;
            arg_types.insert((func, slot), name);
        }

        let segments = session.segments().to_vec();
        let mut bytes = Vec::with_capacity(segments.len());
        for segment in &segments {
            let data = if segment.file_size == 0 {
                Vec::new()
            } else {
                session.read_bytes(segment.range.start, segment.file_size as usize)?
            };
            bytes.push(data);
        }

        Ok(Self {
            image: session.image().clone(),
            segments,
            bytes,
            mode: session.mode(),
            arch: session.arch(),
            format: session.format(),
            functions,
            names,
            prototypes,
            strings,
            structs,
            arg_types,
            decoders: RwLock::new(HashMap::new()),
        })
    }

    fn lock_decoders(&self) -> RwLockReadGuard<'_, HashMap<u64, Insn>> {
        // A panic while holding the cache poisons it; the map itself is still consistent.
        self.decoders
            .read()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    fn lock_decoders_mut(&self) -> RwLockWriteGuard<'_, HashMap<u64, Insn>> {
        self.decoders
            .write()
            .unwrap_or_else(|poison| poison.into_inner())
    }
}

impl SessionView for SessionSnapshot {
    fn image(&self) -> &Image {
        &self.image
    }

    fn segments(&self) -> &[Segment] {
        &self.segments
    }

    fn mode(&self) -> Mode {
        self.mode
    }

    fn arch(&self) -> Arch {
        self.arch
    }

    fn format(&self) -> Format {
        self.format
    }

    fn entry_point(&self) -> Address {
        self.image.entry_point
    }

    fn functions(&self) -> Result<Vec<FunctionInfo>> {
        Ok(self.functions.clone())
    }

    fn name_at(&self, addr: Address) -> Result<Option<String>> {
        Ok(self.names.get(&addr.get()).cloned())
    }

    fn prototype_at(&self, addr: Address) -> Result<Option<String>> {
        Ok(self.prototypes.get(&addr.get()).cloned())
    }

    fn struct_for_arg(&self, func: Address, slot: u8) -> Result<Option<StructDef>> {
        let Some(name) = self.arg_types.get(&(func.get(), slot)) else {
            return Ok(None);
        };
        Ok(self.structs.get(name).cloned())
    }

    fn insn_at(&self, addr: Address) -> Result<Option<Insn>> {
        let cached = self.lock_decoders().get(&addr.get()).copied();
        if let Some(insn) = cached {
            return Ok(Some(insn));
        }
        let mut raw = [0u8; 15];
        let read = self.read_at(addr, &mut raw)?;
        if read == 0 {
            return Ok(None);
        }
        let Some(insn) = decode(&raw[..read], addr, self.mode) else {
            return Ok(None);
        };
        {
            let mut decoded = self.lock_decoders_mut();
            if decoded.len() < MAX_DECODED_INSTRUCTIONS {
                decoded.insert(addr.get(), insn);
            }
        }
        Ok(Some(insn))
    }

    fn read_at(&self, addr: Address, out: &mut [u8]) -> Result<usize> {
        let mut filled = 0usize;
        while filled < out.len() {
            let cursor = addr.get().saturating_add(filled as u64);
            let Some(index) = self
                .segments
                .iter()
                .position(|segment| segment.contains(Address::new(cursor)))
            else {
                break;
            };
            let segment = &self.segments[index];
            let data = &self.bytes[index];
            let delta = cursor - segment.range.start.get();
            if delta >= segment.file_size {
                break;
            }
            let remaining_in_file = segment.file_size - delta;
            let remaining_in_segment = segment.range.end.get() - cursor;
            let want = (out.len() - filled) as u64;
            let take = want.min(remaining_in_file).min(remaining_in_segment) as usize;
            if take == 0 {
                break;
            }
            let available = data.len().saturating_sub(delta as usize);
            if available == 0 {
                break;
            }
            let copied = take.min(available);
            out[filled..filled + copied]
                .copy_from_slice(&data[delta as usize..delta as usize + copied]);
            filled += copied;
            // A truncated project file ends the read exactly like `Session::read_at` does.
            if copied < take {
                break;
            }
        }
        Ok(filled)
    }

    fn string_meta_at(&self, addr: Address) -> Result<Option<(u32, i64)>> {
        Ok(self.strings.get(&addr.get()).copied())
    }
}
