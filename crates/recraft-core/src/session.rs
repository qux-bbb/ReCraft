// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! A `Session` is the in-memory analysis view over one project database.
//!
//! Responsibilities (M0):
//! - own the writer `Project`, the parsed PE metadata and the segment table;
//! - page-cache the original image bytes so decoding does not hit SQLite per instruction;
//! - keep per-byte flags (code/data/head) with a page cache;
//! - apply user [`Command`]s atomically together with their journal record, and undo/redo;
//! - expose read-only views (linear disassembly) to the CLI/UI.
//!
//! Threading: the session is single-threaded (the writer thread). UI/MCP readers use a
//! separate `ReadOnlyProject` connection.

use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use recraft_common::{Address, AddressRange, Error, Result};
use recraft_decode::{DecodeCache, FlowKind, Formatter, Insn, Mode, decode};
use recraft_format::PeImage;
use recraft_storage::Project;
use rusqlite::{Connection, OptionalExtension, params};

use crate::command::{
    self, Command, CommentKind, Effect, JournalRecord, NAME_KIND_AUTO, NAME_KIND_LOADER, StructDef,
    StructField,
};
use crate::flags::{Flags, FlagsStore};
use crate::segments::{self, Segment};
use crate::view::{DisasmLine, FunctionInfo, StringEntry, XrefEntry};

const IMAGE_PAGE_BYTES: u64 = 0x1_0000;
const IMAGE_CACHE_PAGES: usize = 512;
const FLAGS_CACHE_PAGES: usize = 1024;
const DECODE_CACHE_ENTRIES: usize = 65_536;

/// `func.flags` bit marking functions produced by automatic analysis (derived data).
pub const FUNC_FLAG_AUTO: u32 = 1;

/// Page cache over the project's original image bytes.
struct ImageCache {
    pages: HashMap<u64, Vec<u8>>,
    order: VecDeque<u64>,
    capacity: usize,
}

impl ImageCache {
    fn new(capacity: usize) -> Self {
        Self {
            pages: HashMap::new(),
            order: VecDeque::new(),
            capacity: capacity.max(1),
        }
    }

    fn page(&mut self, project: &Project, page_no: u64) -> Result<&[u8]> {
        if self.pages.contains_key(&page_no) {
            touch(&mut self.order, page_no);
        } else {
            let data = project.read_image(page_no * IMAGE_PAGE_BYTES, IMAGE_PAGE_BYTES as usize)?;
            while self.order.len() >= self.capacity {
                if let Some(evicted) = self.order.pop_front() {
                    self.pages.remove(&evicted);
                } else {
                    break;
                }
            }
            self.pages.insert(page_no, data);
            self.order.push_back(page_no);
        }
        self.pages
            .get(&page_no)
            .map(Vec::as_slice)
            .ok_or_else(|| Error::internal("image page vanished from cache"))
    }

    /// Read at a *file* offset (the cache is keyed by file offset, not VA).
    fn read_file_at(&mut self, project: &Project, offset: u64, out: &mut [u8]) -> Result<usize> {
        let mut filled = 0usize;
        while filled < out.len() {
            let cursor = offset.saturating_add(filled as u64);
            let page_no = cursor / IMAGE_PAGE_BYTES;
            let page = self.page(project, page_no)?;
            let inner = (cursor % IMAGE_PAGE_BYTES) as usize;
            if inner >= page.len() {
                break;
            }
            let take = (page.len() - inner).min(out.len() - filled);
            out[filled..filled + take].copy_from_slice(&page[inner..inner + take]);
            filled += take;
        }
        Ok(filled)
    }
}

fn touch(order: &mut VecDeque<u64>, key: u64) {
    if let Some(position) = order.iter().position(|&value| value == key)
        && position + 1 != order.len()
    {
        order.remove(position);
        order.push_back(key);
    }
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

/// Open/create one ReCraft project and provide the analysis-facing API.
pub struct Session {
    project: Project,
    image: PeImage,
    mode: Mode,
    segments: Vec<Segment>,
    image_cache: ImageCache,
    flags: FlagsStore,
    decoders: DecodeCache,
}

impl Session {
    /// Import a PE file into a new project at `path`.
    pub fn create(
        path: &Path,
        bytes: &[u8],
        progress: &mut dyn FnMut(u64, u64),
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Self> {
        let image = recraft_format::load(bytes)?;
        let segments = segments::from_pe(&image);
        let format_name = if image.is_64 { "pe32+" } else { "pe32" };
        let original_name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("program")
            .to_owned();
        let project = Project::create(
            path,
            bytes,
            &original_name,
            format_name,
            progress,
            cancelled,
        )?;

        let json = serde_json::to_string(&image)
            .map_err(|error| Error::internal(format!("serialize PE metadata: {error}")))?;
        project.set_meta_string("pe_meta", &json)?;
        write_segments(project.conn(), &segments)?;
        install_initial_names(project.conn(), &image)?;

        Ok(Self {
            mode: Mode::from_is_64(image.is_64),
            image,
            segments,
            project,
            image_cache: ImageCache::new(IMAGE_CACHE_PAGES),
            flags: FlagsStore::new(FLAGS_CACHE_PAGES),
            decoders: DecodeCache::new(DECODE_CACHE_ENTRIES),
        })
    }

    /// Re-open an existing project (no image bytes are re-read up front).
    pub fn open(path: &Path) -> Result<Self> {
        let project = Project::open(path)?;
        let json = project.meta_string("pe_meta")?.ok_or_else(|| {
            Error::storage("project is missing PE metadata (re-import the binary)")
        })?;
        let image: PeImage = serde_json::from_str(&json)
            .map_err(|error| Error::internal(format!("corrupt PE metadata: {error}")))?;
        let segments = segments::from_pe(&image);
        Ok(Self {
            mode: Mode::from_is_64(image.is_64),
            image,
            segments,
            project,
            image_cache: ImageCache::new(IMAGE_CACHE_PAGES),
            flags: FlagsStore::new(FLAGS_CACHE_PAGES),
            decoders: DecodeCache::new(DECODE_CACHE_ENTRIES),
        })
    }

    // ------------------------------------------------------------- accessors --

    pub fn project(&self) -> &Project {
        &self.project
    }

    pub fn project_mut(&mut self) -> &mut Project {
        &mut self.project
    }

    pub fn path(&self) -> &Path {
        self.project.path()
    }

    pub fn image(&self) -> &PeImage {
        &self.image
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    pub fn entry_point(&self) -> Address {
        self.image.entry_point
    }

    pub fn image_len(&self) -> u64 {
        self.image.file_size
    }

    pub fn segment_containing(&self, addr: Address) -> Option<&Segment> {
        self.segments.iter().find(|segment| segment.contains(addr))
    }

    // ------------------------------------------------------------ read paths --

    /// Copy up to `out.len()` image bytes starting at virtual address `addr`;
    /// returns the number of bytes copied. Reads stop at segment boundaries and
    /// never run into file bytes that are not mapped at `addr`.
    pub fn read_at(&mut self, addr: Address, out: &mut [u8]) -> Result<usize> {
        let mut filled = 0usize;
        while filled < out.len() {
            let cursor = addr.get().saturating_add(filled as u64);
            let Some(segment) = self
                .segments
                .iter()
                .find(|segment| segment.contains(Address::new(cursor)))
                .cloned()
            else {
                break;
            };
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
            let file_offset = segment.file_offset + delta;
            let Session {
                project,
                image_cache,
                ..
            } = self;
            let read =
                image_cache.read_file_at(project, file_offset, &mut out[filled..filled + take])?;
            filled += read;
            if read < take {
                break;
            }
        }
        Ok(filled)
    }

    pub fn read_bytes(&mut self, addr: Address, len: usize) -> Result<Vec<u8>> {
        let mut buffer = vec![0u8; len];
        let read = self.read_at(addr, &mut buffer)?;
        buffer.truncate(read);
        Ok(buffer)
    }

    /// Decode the instruction at `addr` (cached).
    pub fn insn_at(&mut self, addr: Address) -> Result<Option<Insn>> {
        if let Some(insn) = self.decoders.get(addr) {
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
        self.decoders.insert(insn);
        Ok(Some(insn))
    }

    pub fn flags_at(&mut self, addr: Address) -> Result<Flags> {
        let Session { project, flags, .. } = self;
        flags.get(project.conn(), addr)
    }

    pub fn snapshot_flags(&mut self, range: AddressRange) -> Result<Vec<Flags>> {
        let Session { project, flags, .. } = self;
        flags.snapshot(project.conn(), range)
    }

    pub fn name_at(&self, addr: Address) -> Result<Option<String>> {
        self.project
            .conn()
            .query_row(
                "SELECT text FROM name WHERE addr = ?1",
                [addr.get() as i64],
                |row| row.get(0),
            )
            .optional()
            .map_err(command::db_err)
    }

    /// All recorded names, ordered by address (IDA "Names" window).
    pub fn names(&self) -> Result<Vec<(Address, String)>> {
        let mut stmt = self
            .project
            .conn()
            .prepare("SELECT addr, text FROM name ORDER BY addr")
            .map_err(command::db_err)?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(command::db_err)?;
        let mut names = Vec::new();
        for row in rows {
            let (addr, text) = row.map_err(command::db_err)?;
            names.push((Address::new(addr as u64), text));
        }
        Ok(names)
    }

    /// User-set prototype (C declaration) at `addr`, if any.
    pub fn prototype_at(&self, addr: Address) -> Result<Option<String>> {
        self.project
            .conn()
            .query_row(
                "SELECT text FROM prototype WHERE addr = ?1",
                [addr.get() as i64],
                |row| row.get(0),
            )
            .optional()
            .map_err(command::db_err)
    }

    /// User-defined struct layout by name.
    pub fn struct_def(&self, name: &str) -> Result<Option<StructDef>> {
        let header: Option<i64> = self
            .project
            .conn()
            .query_row(
                "SELECT size FROM user_struct WHERE name = ?1",
                [name],
                |row| row.get(0),
            )
            .optional()
            .map_err(command::db_err)?;
        let Some(size) = header else {
            return Ok(None);
        };
        let mut statement = self
            .project
            .conn()
            .prepare(
                "SELECT off, name, kind FROM user_struct_field
                 WHERE struct_name = ?1 ORDER BY off",
            )
            .map_err(command::db_err)?;
        let rows = statement
            .query_map([name], |row| {
                Ok(StructField {
                    offset: row.get::<_, i64>(0)? as u64,
                    name: row.get(1)?,
                    kind: row.get(2)?,
                })
            })
            .map_err(command::db_err)?;
        let fields: Vec<StructField> = rows
            .map(|row| row.map_err(command::db_err))
            .collect::<Result<_>>()?;
        Ok(Some(StructDef {
            name: name.to_owned(),
            size: size as u64,
            fields,
        }))
    }

    /// All user-defined struct layouts, ordered by name.
    pub fn structs(&self) -> Result<Vec<StructDef>> {
        let mut statement = self
            .project
            .conn()
            .prepare("SELECT name FROM user_struct ORDER BY name")
            .map_err(command::db_err)?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(command::db_err)?;
        let names: Vec<String> = rows
            .map(|row| row.map_err(command::db_err))
            .collect::<Result<_>>()?;
        let mut defs = Vec::with_capacity(names.len());
        for name in names {
            if let Some(def) = self.struct_def(&name)? {
                defs.push(def);
            }
        }
        Ok(defs)
    }

    /// Struct layout bound to a function argument slot (`0` = first argument).
    pub fn arg_type(&self, func: Address, slot: u8) -> Result<Option<String>> {
        self.project
            .conn()
            .query_row(
                "SELECT struct_name FROM func_arg_type WHERE func_addr = ?1 AND slot = ?2",
                params![func.get() as i64, slot as i64],
                |row| row.get(0),
            )
            .optional()
            .map_err(command::db_err)
    }

    /// Convenience: the struct layout bound to an argument slot, if any.
    pub fn struct_for_arg(&self, func: Address, slot: u8) -> Result<Option<StructDef>> {
        match self.arg_type(func, slot)? {
            Some(name) => self.struct_def(&name),
            None => Ok(None),
        }
    }

    pub fn comment_at(&self, addr: Address, kind: CommentKind) -> Result<Option<String>> {
        self.project
            .conn()
            .query_row(
                "SELECT text FROM comment WHERE addr = ?1 AND kind = ?2",
                params![addr.get() as i64, kind.as_i64()],
                |row| row.get(0),
            )
            .optional()
            .map_err(command::db_err)
    }

    /// End address of the function defined at `start`, if any.
    pub fn func_at(&self, start: Address) -> Result<Option<Address>> {
        let end: Option<i64> = self
            .project
            .conn()
            .query_row(
                "SELECT end FROM func WHERE start = ?1",
                [start.get() as i64],
                |row| row.get(0),
            )
            .optional()
            .map_err(command::db_err)?;
        Ok(end.map(|value| Address::new(value as u64)))
    }

    /// All defined functions, ordered by start address.
    pub fn functions(&self) -> Result<Vec<FunctionInfo>> {
        let mut statement = self
            .project
            .conn()
            .prepare("SELECT start, end FROM func ORDER BY start")
            .map_err(command::db_err)?;
        let rows = statement
            .query_map([], |row| {
                let start: i64 = row.get(0)?;
                let end: i64 = row.get(1)?;
                Ok(FunctionInfo {
                    start: Address::new(start as u64),
                    end: Address::new(end as u64),
                })
            })
            .map_err(command::db_err)?;
        rows.map(|row| row.map_err(command::db_err)).collect()
    }

    /// Resolve a symbol name to its address (first match).
    pub fn addr_of_name(&self, name: &str) -> Result<Option<Address>> {
        let addr: Option<i64> = self
            .project
            .conn()
            .query_row(
                "SELECT addr FROM name WHERE text = ?1 ORDER BY addr LIMIT 1",
                [name],
                |row| row.get(0),
            )
            .optional()
            .map_err(command::db_err)?;
        Ok(addr.map(|value| Address::new(value as u64)))
    }

    /// Replace auto functions intersecting `range` (incremental re-analysis, §6.4).
    pub fn replace_auto_functions_in_range(
        &mut self,
        range: AddressRange,
        functions: &[(Address, Address)],
    ) -> Result<()> {
        let tx = self
            .project
            .conn_mut()
            .transaction()
            .map_err(command::db_err)?;
        tx.execute(
            "DELETE FROM func WHERE (flags & ?1) != 0 AND start < ?3 AND \"end\" > ?2",
            params![
                FUNC_FLAG_AUTO,
                range.start.get() as i64,
                range.end.get() as i64
            ],
        )
        .map_err(command::db_err)?;
        for (start, end) in functions {
            if end <= start {
                continue;
            }
            tx.execute(
                "INSERT INTO func(start, end, flags, cc) VALUES (?1, ?2, ?3, 0)
                 ON CONFLICT(start) DO NOTHING",
                params![start.get() as i64, end.get() as i64, FUNC_FLAG_AUTO],
            )
            .map_err(command::db_err)?;
        }
        tx.commit().map_err(command::db_err)?;
        Ok(())
    }

    /// Replace auto xrefs originating in `range` (incremental re-analysis, §6.4).
    pub fn replace_auto_xrefs_in_range(
        &mut self,
        range: AddressRange,
        xrefs: &[(Address, Address, i64)],
    ) -> Result<()> {
        let tx = self
            .project
            .conn_mut()
            .transaction()
            .map_err(command::db_err)?;
        tx.execute(
            "DELETE FROM xref WHERE from_addr >= ?1 AND from_addr < ?2",
            params![range.start.get() as i64, range.end.get() as i64],
        )
        .map_err(command::db_err)?;
        for (from, to, kind) in xrefs {
            tx.execute(
                "INSERT OR IGNORE INTO xref(from_addr, to_addr, type) VALUES (?1, ?2, ?3)",
                params![from.get() as i64, to.get() as i64, *kind],
            )
            .map_err(command::db_err)?;
        }
        tx.commit().map_err(command::db_err)?;
        Ok(())
    }

    /// Replace auto names in `range` (incremental re-analysis, §6.4).
    pub fn replace_auto_names_in_range(
        &mut self,
        range: AddressRange,
        names: &[(Address, String)],
    ) -> Result<()> {
        let tx = self
            .project
            .conn_mut()
            .transaction()
            .map_err(command::db_err)?;
        tx.execute(
            "DELETE FROM name WHERE kind = ?1 AND addr >= ?2 AND addr < ?3",
            params![
                NAME_KIND_AUTO,
                range.start.get() as i64,
                range.end.get() as i64
            ],
        )
        .map_err(command::db_err)?;
        for (addr, text) in names {
            tx.execute(
                "INSERT OR IGNORE INTO name(addr, text, kind, flags) VALUES (?1, ?2, ?3, 0)",
                params![addr.get() as i64, text, NAME_KIND_AUTO],
            )
            .map_err(command::db_err)?;
        }
        tx.commit().map_err(command::db_err)?;
        Ok(())
    }

    /// Replace API tags in `range` (incremental re-analysis, §6.4).
    pub fn replace_auto_api_tags_in_range(
        &mut self,
        range: AddressRange,
        tags: &[(Address, String, String)],
    ) -> Result<()> {
        let tx = self
            .project
            .conn_mut()
            .transaction()
            .map_err(command::db_err)?;
        tx.execute(
            "DELETE FROM api_tag WHERE addr >= ?1 AND addr < ?2",
            params![range.start.get() as i64, range.end.get() as i64],
        )
        .map_err(command::db_err)?;
        for (addr, rule_id, category) in tags {
            tx.execute(
                "INSERT OR IGNORE INTO api_tag(addr, rule_id, category) VALUES (?1, ?2, ?3)",
                params![addr.get() as i64, rule_id, category],
            )
            .map_err(command::db_err)?;
        }
        tx.commit().map_err(command::db_err)?;
        Ok(())
    }

    /// Replace functions produced by automatic analysis, keeping user-defined ones.
    ///
    /// Analysis results are derived data (see DESIGN §2.3): they are not journaled and
    /// can be recomputed at any time. User `DefineFunc` commands write `flags = 0` and
    /// therefore survive a re-analysis.
    pub fn replace_auto_functions(&mut self, functions: &[(Address, Address)]) -> Result<()> {
        let tx = self
            .project
            .conn_mut()
            .transaction()
            .map_err(command::db_err)?;
        tx.execute("DELETE FROM func WHERE (flags & ?1) != 0", [FUNC_FLAG_AUTO])
            .map_err(command::db_err)?;
        for (start, end) in functions {
            if end <= start {
                continue;
            }
            tx.execute(
                "INSERT INTO func(start, end, flags, cc) VALUES (?1, ?2, ?3, 0)
                 ON CONFLICT(start) DO NOTHING",
                params![start.get() as i64, end.get() as i64, FUNC_FLAG_AUTO as i64],
            )
            .map_err(command::db_err)?;
        }
        tx.commit().map_err(command::db_err)?;
        Ok(())
    }

    /// Replace all derived xrefs (analysis output). User data is untouched.
    pub fn replace_auto_xrefs(&mut self, xrefs: &[(Address, Address, i64)]) -> Result<()> {
        let tx = self
            .project
            .conn_mut()
            .transaction()
            .map_err(command::db_err)?;
        tx.execute("DELETE FROM xref", [])
            .map_err(command::db_err)?;
        for (from, to, kind) in xrefs {
            tx.execute(
                "INSERT OR IGNORE INTO xref(from_addr, to_addr, type) VALUES (?1, ?2, ?3)",
                params![from.get() as i64, to.get() as i64, *kind],
            )
            .map_err(command::db_err)?;
        }
        tx.commit().map_err(command::db_err)?;
        Ok(())
    }

    /// Replace analysis-produced names (`sub_...`). Loader and user names win.
    pub fn replace_auto_names(&mut self, names: &[(Address, String)]) -> Result<()> {
        let tx = self
            .project
            .conn_mut()
            .transaction()
            .map_err(command::db_err)?;
        tx.execute("DELETE FROM name WHERE kind = ?1", [NAME_KIND_AUTO])
            .map_err(command::db_err)?;
        for (addr, text) in names {
            tx.execute(
                "INSERT OR IGNORE INTO name(addr, text, kind, flags) VALUES (?1, ?2, ?3, 0)",
                params![addr.get() as i64, text, NAME_KIND_AUTO],
            )
            .map_err(command::db_err)?;
        }
        tx.commit().map_err(command::db_err)?;
        Ok(())
    }

    /// Replace API tags produced by analysis.
    pub fn replace_auto_api_tags(&mut self, tags: &[(Address, String, String)]) -> Result<()> {
        let tx = self
            .project
            .conn_mut()
            .transaction()
            .map_err(command::db_err)?;
        tx.execute("DELETE FROM api_tag", [])
            .map_err(command::db_err)?;
        for (addr, rule_id, category) in tags {
            tx.execute(
                "INSERT OR IGNORE INTO api_tag(addr, rule_id, category) VALUES (?1, ?2, ?3)",
                params![addr.get() as i64, rule_id, category],
            )
            .map_err(command::db_err)?;
        }
        tx.commit().map_err(command::db_err)?;
        Ok(())
    }

    /// API tags attached to one address (call sites referencing an import).
    pub fn api_tags_at(&self, addr: Address) -> Result<Vec<(String, String)>> {
        let mut stmt = self
            .project
            .conn()
            .prepare("SELECT rule_id, category FROM api_tag WHERE addr = ?1")
            .map_err(command::db_err)?;
        let rows = stmt
            .query_map([addr.get() as i64], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(command::db_err)?;
        rows.map(|row| row.map_err(command::db_err)).collect()
    }

    /// All analysis API tags: `(call site, rule id, category)`.
    pub fn all_api_tags(&self) -> Result<Vec<(Address, String, String)>> {
        let mut stmt = self
            .project
            .conn()
            .prepare("SELECT addr, rule_id, category FROM api_tag ORDER BY addr")
            .map_err(command::db_err)?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    Address::new(row.get::<_, i64>(0)? as u64),
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(command::db_err)?;
        rows.map(|row| row.map_err(command::db_err)).collect()
    }

    /// Search mapped readable segments for a byte pattern (chunked to bound memory).
    pub fn search_bytes(&mut self, needle: &[u8], limit: usize) -> Result<Vec<Address>> {
        if needle.is_empty() {
            return Err(Error::internal("empty search pattern"));
        }
        let limit = limit.max(1);
        let segments = self.segments().to_vec();
        let mut hits = Vec::new();
        'segments: for segment in segments {
            if !segment.perm.is_readable() {
                continue;
            }
            let start = segment.range.start.get();
            let size = segment.range.len();
            let chunk_size = 1usize << 20;
            let overlap = needle.len().saturating_sub(1);
            let mut offset = 0u64;
            while offset < size {
                let len = (size - offset).min(chunk_size as u64) as usize;
                let chunk = self.read_bytes(Address::new(start + offset), len)?;
                for index in 0..=chunk.len().saturating_sub(needle.len()) {
                    if chunk[index..index + needle.len()] == *needle {
                        hits.push(Address::new(start + offset + index as u64));
                        if hits.len() >= limit {
                            break 'segments;
                        }
                    }
                }
                if offset + len as u64 >= size {
                    break;
                }
                offset += (len - overlap) as u64;
            }
        }
        Ok(hits)
    }

    /// Number of analysis-produced names currently stored.
    pub fn auto_name_count(&self) -> Result<u64> {
        let count: i64 = self
            .project
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM name WHERE kind = ?1",
                [NAME_KIND_AUTO],
                |row| row.get(0),
            )
            .map_err(command::db_err)?;
        Ok(count as u64)
    }

    /// Replace all extracted strings (analysis output).
    pub fn replace_auto_strings(&mut self, strings: &[(Address, u32, i64, i64)]) -> Result<()> {
        let tx = self
            .project
            .conn_mut()
            .transaction()
            .map_err(command::db_err)?;
        tx.execute("DELETE FROM string", [])
            .map_err(command::db_err)?;
        for (addr, len, enc, kind) in strings {
            tx.execute(
                "INSERT OR REPLACE INTO string(addr, len, enc, kind) VALUES (?1, ?2, ?3, ?4)",
                params![addr.get() as i64, *len as i64, *enc, *kind],
            )
            .map_err(command::db_err)?;
        }
        tx.commit().map_err(command::db_err)?;
        Ok(())
    }

    /// All stored strings, ordered by address.
    pub fn strings(&self) -> Result<Vec<StringEntry>> {
        let mut stmt = self
            .project
            .conn()
            .prepare("SELECT addr, len, enc, kind FROM string ORDER BY addr")
            .map_err(command::db_err)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(StringEntry {
                    addr: Address::new(row.get::<_, i64>(0)? as u64),
                    len: row.get::<_, i64>(1)? as u32,
                    enc: row.get(2)?,
                    kind: row.get(3)?,
                })
            })
            .map_err(command::db_err)?;
        rows.map(|row| row.map_err(command::db_err)).collect()
    }

    /// Stored string text at `addr`, decoded from its encoding (1 = ASCII, 2 = UTF-16LE).
    ///
    /// Returns `None` when the address is not the start of a stored string.
    pub fn string_at(&mut self, addr: Address) -> Result<Option<String>> {
        let row: Option<(u32, i64)> = self
            .project
            .conn()
            .query_row(
                "SELECT len, enc FROM string WHERE addr = ?1",
                [addr.get() as i64],
                |row| Ok((row.get::<_, i64>(0)? as u32, row.get(1)?)),
            )
            .optional()
            .map_err(command::db_err)?;
        let Some((len, enc)) = row else {
            return Ok(None);
        };
        let mut bytes = vec![0u8; len.min(4096) as usize];
        let read = self.read_at(addr, &mut bytes)?;
        bytes.truncate(read);
        let text = if enc == 2 {
            let units: Vec<u16> = bytes
                .chunks_exact(2)
                .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
                .take_while(|unit| *unit != 0)
                .collect();
            String::from_utf16_lossy(&units)
        } else {
            let end = bytes
                .iter()
                .position(|byte| *byte == 0)
                .unwrap_or(bytes.len());
            String::from_utf8_lossy(&bytes[..end]).into_owned()
        };
        Ok(Some(text))
    }

    /// Cross-references pointing *to* `addr`.
    pub fn xrefs_to(&self, addr: Address) -> Result<Vec<XrefEntry>> {
        let mut stmt = self
            .project
            .conn()
            .prepare(
                "SELECT from_addr, to_addr, type FROM xref WHERE to_addr = ?1 ORDER BY from_addr",
            )
            .map_err(command::db_err)?;
        let rows = stmt
            .query_map([addr.get() as i64], |row| {
                Ok(XrefEntry {
                    from: Address::new(row.get::<_, i64>(0)? as u64),
                    to: Address::new(row.get::<_, i64>(1)? as u64),
                    kind: row.get(2)?,
                })
            })
            .map_err(command::db_err)?;
        rows.map(|row| row.map_err(command::db_err)).collect()
    }

    /// All persisted cross-references, optionally filtered by kind.
    pub fn all_xrefs(&self, kind: Option<i64>) -> Result<Vec<XrefEntry>> {
        let mut stmt = self
            .project
            .conn()
            .prepare(
                "SELECT from_addr, to_addr, type FROM xref
                 WHERE (?1 IS NULL OR type = ?1)
                 ORDER BY from_addr",
            )
            .map_err(command::db_err)?;
        let rows = stmt
            .query_map(params![kind], |row| {
                Ok(XrefEntry {
                    from: Address::new(row.get::<_, i64>(0)? as u64),
                    to: Address::new(row.get::<_, i64>(1)? as u64),
                    kind: row.get(2)?,
                })
            })
            .map_err(command::db_err)?;
        rows.map(|row| row.map_err(command::db_err)).collect()
    }

    /// Cross-references originating *at* `addr`.
    pub fn xrefs_from(&self, addr: Address) -> Result<Vec<XrefEntry>> {
        let mut stmt = self
            .project
            .conn()
            .prepare(
                "SELECT from_addr, to_addr, type FROM xref WHERE from_addr = ?1 ORDER BY to_addr",
            )
            .map_err(command::db_err)?;
        let rows = stmt
            .query_map([addr.get() as i64], |row| {
                Ok(XrefEntry {
                    from: Address::new(row.get::<_, i64>(0)? as u64),
                    to: Address::new(row.get::<_, i64>(1)? as u64),
                    kind: row.get(2)?,
                })
            })
            .map_err(command::db_err)?;
        rows.map(|row| row.map_err(command::db_err)).collect()
    }

    // ----------------------------------------------------------- commands ----

    /// Apply a user command, journal it, and invalidate the redo stack.
    pub fn execute(&mut self, cmd: Command) -> Result<Effect> {
        let inverse = cmd.inverse(self)?;
        let record = JournalRecord {
            command: cmd.clone(),
            inverse,
        };
        let payload = serde_json::to_vec(&record)
            .map_err(|error| Error::internal(format!("serialize journal: {error}")))?;
        let effect = cmd.effect();

        {
            let Session {
                project,
                segments,
                flags,
                ..
            } = self;
            let tx = project.conn_mut().transaction().map_err(command::db_err)?;
            // A new command invalidates everything that could be redone.
            tx.execute("DELETE FROM journal WHERE undone = 1", [])
                .map_err(command::db_err)?;
            command::apply(&tx, segments, flags, &cmd)?;
            tx.execute(
                "INSERT INTO journal(ts, cmd, payload) VALUES (?1, ?2, ?3)",
                params![unix_now(), cmd.name(), payload],
            )
            .map_err(command::db_err)?;
            tx.commit().map_err(command::db_err)?;
        }

        self.decoders.clear();
        Ok(effect)
    }

    /// Undo the newest applied command.
    pub fn undo(&mut self) -> Result<Option<Effect>> {
        let last = {
            let Session { project, .. } = self;
            command::last_active_record(project.conn())?
        };
        let Some((seq, payload)) = last else {
            return Ok(None);
        };
        let record: JournalRecord = serde_json::from_slice(&payload)
            .map_err(|error| Error::internal(format!("corrupt journal: {error}")))?;
        let effect = record.inverse.effect();

        {
            let Session {
                project,
                segments,
                flags,
                ..
            } = self;
            let tx = project.conn_mut().transaction().map_err(command::db_err)?;
            tx.execute("UPDATE journal SET undone = 1 WHERE seq = ?1", [seq])
                .map_err(command::db_err)?;
            command::apply(&tx, segments, flags, &record.inverse)?;
            tx.commit().map_err(command::db_err)?;
        }

        self.decoders.clear();
        Ok(Some(effect))
    }

    /// Redo the oldest undone command.
    pub fn redo(&mut self) -> Result<Option<Effect>> {
        let first = {
            let Session { project, .. } = self;
            command::first_undone_record(project.conn())?
        };
        let Some((seq, payload)) = first else {
            return Ok(None);
        };
        let record: JournalRecord = serde_json::from_slice(&payload)
            .map_err(|error| Error::internal(format!("corrupt journal: {error}")))?;
        let effect = record.command.effect();

        {
            let Session {
                project,
                segments,
                flags,
                ..
            } = self;
            let tx = project.conn_mut().transaction().map_err(command::db_err)?;
            tx.execute("UPDATE journal SET undone = 0 WHERE seq = ?1", [seq])
                .map_err(command::db_err)?;
            command::apply(&tx, segments, flags, &record.command)?;
            tx.commit().map_err(command::db_err)?;
        }

        self.decoders.clear();
        Ok(Some(effect))
    }

    pub fn can_undo(&self) -> Result<bool> {
        let count: i64 = self
            .project
            .conn()
            .query_row("SELECT COUNT(*) FROM journal WHERE undone = 0", [], |row| {
                row.get(0)
            })
            .map_err(command::db_err)?;
        Ok(count > 0)
    }

    pub fn can_redo(&self) -> Result<bool> {
        let count: i64 = self
            .project
            .conn()
            .query_row("SELECT COUNT(*) FROM journal WHERE undone = 1", [], |row| {
                row.get(0)
            })
            .map_err(command::db_err)?;
        Ok(count > 0)
    }

    // --------------------------------------------------------------- views ---

    /// Linear disassembly starting at `start`.
    pub fn disasm(&mut self, start: Address, count: usize) -> Result<Vec<DisasmLine>> {
        let mut lines = Vec::with_capacity(count.min(4096));
        let mut formatter = Formatter::new();
        let mut addr = start;
        for _ in 0..count {
            if self.segment_containing(addr).is_none() {
                break;
            }
            let Some(insn) = self.insn_at(addr)? else {
                break;
            };
            let mut raw = [0u8; 15];
            let read = self.read_at(addr, &mut raw)?;
            if read == 0 {
                break;
            }
            let (mnemonic, mut operands) = {
                let (mnemonic, operands) = formatter.format_parts(&insn);
                (mnemonic.to_owned(), operands.to_owned())
            };
            // Resolve names for direct memory operands (IAT slots, globals) and direct
            // call/jump targets (auto `sub_...`, exports) so disassembly is readable.
            let referenced = if insn.memory_is_direct() {
                Some(Address::new(insn.memory_displacement64()))
            } else if matches!(insn.flow(), FlowKind::Call | FlowKind::UnconditionalBranch) {
                insn.branch_target()
            } else {
                None
            };
            if let Some(target) = referenced
                && let Some(target_name) = self.name_at(target)?
            {
                operands.push_str("  ; ");
                operands.push_str(&target_name);
            }
            let name = self.name_at(addr)?;
            let comment = self.comment_at(addr, CommentKind::Regular)?;
            let flags = self.flags_at(addr)?;
            lines.push(DisasmLine {
                addr,
                len: insn.len(),
                raw,
                mnemonic,
                operands,
                name,
                comment,
                flags,
            });
            addr = insn.next_addr();
        }
        Ok(lines)
    }
}

fn write_segments(conn: &Connection, segments: &[Segment]) -> Result<()> {
    conn.execute("DELETE FROM segment", [])
        .map_err(command::db_err)?;
    for segment in segments {
        conn.execute(
            "INSERT INTO segment(name, start, end, file_off, kind, perm, bitness)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)",
            params![
                segment.name,
                segment.range.start.get() as i64,
                segment.range.end.get() as i64,
                segment.file_offset as i64,
                segment_kind_id(segment.kind),
                perm_id(segment.perm),
            ],
        )
        .map_err(command::db_err)?;
    }
    Ok(())
}

fn segment_kind_id(kind: segments::SegmentKind) -> i64 {
    match kind {
        segments::SegmentKind::Headers => 0,
        segments::SegmentKind::Code => 1,
        segments::SegmentKind::Data => 2,
        segments::SegmentKind::Bss => 3,
    }
}

fn perm_id(perm: segments::Perm) -> i64 {
    match perm {
        segments::Perm::None => 0,
        segments::Perm::Read => 1,
        segments::Perm::ReadWrite => 2,
        segments::Perm::ReadExecute => 3,
        segments::Perm::ReadWriteExecute => 4,
    }
}

fn install_initial_names(conn: &Connection, image: &PeImage) -> Result<()> {
    let mut statement = conn
        .prepare("INSERT OR IGNORE INTO name(addr, text, kind, flags) VALUES (?1, ?2, ?3, 0)")
        .map_err(command::db_err)?;
    statement
        .execute(params![
            image.entry_point.get() as i64,
            "start",
            NAME_KIND_LOADER
        ])
        .map_err(command::db_err)?;
    for export in &image.exports {
        let Some(name) = export.name.as_deref() else {
            continue;
        };
        if name.is_empty() {
            continue;
        }
        let display = recraft_common::demangle(name).unwrap_or_else(|| name.to_owned());
        let va = image.image_base.get().saturating_add(u64::from(export.rva));
        statement
            .execute(params![va as i64, display, NAME_KIND_LOADER])
            .map_err(command::db_err)?;
    }
    // Name IAT slots `MODULE.dll!Api` so disassembly and xrefs resolve imports.
    for module in &image.imports {
        for function in &module.functions {
            if function.iat_va.get() == 0 {
                continue;
            }
            let text = match (&function.name, function.ordinal) {
                (Some(name), _) if !name.is_empty() => {
                    let api = recraft_common::demangle(name).unwrap_or_else(|| name.clone());
                    format!("{}!{api}", module.name)
                }
                (_, Some(ordinal)) => format!("{}!#{ordinal}", module.name),
                _ => continue,
            };
            statement
                .execute(params![
                    function.iat_va.get() as i64,
                    text,
                    NAME_KIND_LOADER
                ])
                .map_err(command::db_err)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flags::CodeData;
    use tempfile::TempDir;

    /// Use a system PE if present (Windows dev/CI); skip the test otherwise.
    fn sample_pe() -> Option<Vec<u8>> {
        for path in [
            "C:/Windows/System32/notepad.exe",
            "C:/Windows/System32/kernel32.dll",
        ] {
            if let Ok(bytes) = std::fs::read(path)
                && !bytes.is_empty()
            {
                return Some(bytes);
            }
        }
        None
    }

    fn fixture(dir: &TempDir) -> Option<Session> {
        let bytes = sample_pe()?;
        let path = dir.path().join("test.rcraft");
        Some(Session::create(&path, &bytes, &mut |_, _| {}, &|| false).expect("create session"))
    }

    #[test]
    fn create_opens_with_entry_name_and_disassembly() {
        let dir = TempDir::new().unwrap();
        let Some(session) = fixture(&dir) else {
            return;
        };
        let entry = session.entry_point();
        assert_ne!(entry.get(), 0);
        assert_eq!(session.mode(), Mode::X64);
        assert!(!session.segments().is_empty());
        assert_eq!(session.name_at(entry).unwrap().as_deref(), Some("start"));

        let path = session.path().to_path_buf();
        drop(session);

        let mut reopened = Session::open(&path).unwrap();
        assert_eq!(reopened.entry_point(), entry);
        let lines = reopened.disasm(entry, 8).unwrap();
        assert!(!lines.is_empty());
        assert_eq!(lines[0].addr, entry);
        assert!(!lines[0].mnemonic.is_empty());
        assert_ne!(lines[0].len, 0);
    }

    #[test]
    fn rename_comment_undo_redo_and_persistence() {
        let dir = TempDir::new().unwrap();
        let Some(mut session) = fixture(&dir) else {
            return;
        };
        let entry = session.entry_point();

        session
            .execute(Command::Rename {
                addr: entry,
                name: Some("my_main".to_owned()),
            })
            .unwrap();
        assert_eq!(session.name_at(entry).unwrap().as_deref(), Some("my_main"));
        assert!(session.can_undo().unwrap());

        session
            .execute(Command::SetComment {
                addr: entry,
                kind: CommentKind::Regular,
                text: Some("网络回调初始化".to_owned()),
            })
            .unwrap();
        assert_eq!(
            session
                .comment_at(entry, CommentKind::Regular)
                .unwrap()
                .as_deref(),
            Some("网络回调初始化")
        );

        session.undo().unwrap();
        assert!(
            session
                .comment_at(entry, CommentKind::Regular)
                .unwrap()
                .is_none()
        );
        session.undo().unwrap();
        assert_eq!(session.name_at(entry).unwrap().as_deref(), Some("start"));
        assert!(session.can_redo().unwrap());

        session.redo().unwrap();
        assert_eq!(session.name_at(entry).unwrap().as_deref(), Some("my_main"));

        let path = session.path().to_path_buf();
        drop(session);

        let mut reopened = Session::open(&path).unwrap();
        assert_eq!(reopened.name_at(entry).unwrap().as_deref(), Some("my_main"));
        // The journal survives reopening: undo still works on the loaded project.
        assert!(reopened.can_undo().unwrap());
        reopened.undo().unwrap();
        assert_eq!(reopened.name_at(entry).unwrap().as_deref(), Some("start"));
        assert!(reopened.can_redo().unwrap());

        // A new command after undo discards the redo stack.
        reopened
            .execute(Command::Rename {
                addr: entry,
                name: Some("final_name".to_owned()),
            })
            .unwrap();
        assert!(!reopened.can_redo().unwrap());
    }

    #[test]
    fn search_bytes_finds_known_patterns() {
        let dir = TempDir::new().unwrap();
        let Some(mut session) = fixture(&dir) else {
            return;
        };
        let entry = session.entry_point();
        let pattern = session.read_bytes(entry, 4).unwrap();
        let hits = session.search_bytes(&pattern, 8).unwrap();
        assert!(hits.contains(&entry), "entry not in {hits:?}");
        assert!(session.search_bytes(&[], 8).is_err());
        // A pattern that cannot occur in the image returns no hits.
        let impossible = vec![0xde, 0xad, 0xbe, 0xef, 0xde, 0xad, 0xbe, 0xef];
        assert!(session.search_bytes(&impossible, 8).unwrap().is_empty());
    }

    #[test]
    fn struct_layouts_and_arg_type_bindings_undo_redo() {
        let dir = TempDir::new().unwrap();
        let Some(mut session) = fixture(&dir) else {
            return;
        };
        let entry = session.entry_point();

        let def = StructDef {
            name: "Probe".to_owned(),
            size: 0x18,
            fields: vec![
                StructField {
                    offset: 0x10,
                    name: "value".to_owned(),
                    kind: "i32".to_owned(),
                },
                StructField {
                    offset: 0,
                    name: "tag".to_owned(),
                    kind: "u32".to_owned(),
                },
            ],
        };
        session.execute(Command::DefineStruct { def }).unwrap();
        let stored = session.struct_def("Probe").unwrap().expect("struct stored");
        // Field order is normalised by offset.
        assert_eq!(stored.fields[0].name, "tag");
        assert_eq!(stored.fields[1].name, "value");

        session
            .execute(Command::ApplyType {
                func: entry,
                slot: 0,
                struct_name: Some("Probe".to_owned()),
            })
            .unwrap();
        assert_eq!(
            session.arg_type(entry, 0).unwrap().as_deref(),
            Some("Probe")
        );
        assert_eq!(
            session
                .struct_for_arg(entry, 0)
                .unwrap()
                .map(|def| def.name),
            Some("Probe".to_owned())
        );

        // Unknown struct is rejected before touching the database.
        assert!(
            session
                .execute(Command::ApplyType {
                    func: entry,
                    slot: 1,
                    struct_name: Some("Missing".to_owned()),
                })
                .is_err()
        );

        // Undo the binding, then the struct; redo restores both.
        session.undo().unwrap();
        assert!(session.arg_type(entry, 0).unwrap().is_none());
        session.undo().unwrap();
        assert!(session.struct_def("Probe").unwrap().is_none());
        session.redo().unwrap();
        assert!(session.struct_def("Probe").unwrap().is_some());
        session.redo().unwrap();
        assert_eq!(
            session.arg_type(entry, 0).unwrap().as_deref(),
            Some("Probe")
        );

        let path = session.path().to_path_buf();
        drop(session);
        let reopened = Session::open(&path).unwrap();
        assert_eq!(
            reopened
                .struct_for_arg(entry, 0)
                .unwrap()
                .expect("binding survives reopen")
                .fields
                .len(),
            2
        );
    }

    #[test]
    fn prototype_undo_redo_and_persistence() {
        let dir = TempDir::new().unwrap();
        let Some(mut session) = fixture(&dir) else {
            return;
        };
        let entry = session.entry_point();
        assert!(session.prototype_at(entry).unwrap().is_none());

        session
            .execute(Command::SetPrototype {
                addr: entry,
                text: Some("int __stdcall my_main(int code)".to_owned()),
            })
            .unwrap();
        assert_eq!(
            session.prototype_at(entry).unwrap().as_deref(),
            Some("int __stdcall my_main(int code)")
        );

        session.undo().unwrap();
        assert!(session.prototype_at(entry).unwrap().is_none());
        session.redo().unwrap();
        assert!(session.prototype_at(entry).unwrap().is_some());

        // Clearing is also a command; undo restores the previous text.
        session
            .execute(Command::SetPrototype {
                addr: entry,
                text: None,
            })
            .unwrap();
        assert!(session.prototype_at(entry).unwrap().is_none());
        session.undo().unwrap();
        assert!(session.prototype_at(entry).unwrap().is_some());

        let path = session.path().to_path_buf();
        drop(session);
        let mut reopened = Session::open(&path).unwrap();
        assert!(reopened.prototype_at(entry).unwrap().is_some());
        reopened.undo().unwrap();
        assert!(reopened.prototype_at(entry).unwrap().is_none());
    }

    #[test]
    fn code_data_flags_roundtrip_with_undo() {
        let dir = TempDir::new().unwrap();
        let Some(mut session) = fixture(&dir) else {
            return;
        };
        let entry = session.entry_point();
        assert_eq!(session.flags_at(entry).unwrap(), Flags::UNKNOWN);

        let range = AddressRange::new(entry, Address::new(entry.get() + 4));
        session
            .execute(Command::SetCodeData {
                range,
                kind: CodeData::Code,
            })
            .unwrap();
        let flags = session.flags_at(entry).unwrap();
        assert!(flags.is_code() && flags.head);
        let tail = session.flags_at(Address::new(entry.get() + 1)).unwrap();
        assert!(tail.is_code() && !tail.head);

        session.undo().unwrap();
        assert_eq!(session.flags_at(entry).unwrap(), Flags::UNKNOWN);
        session.redo().unwrap();
        assert!(session.flags_at(entry).unwrap().is_code());
    }

    #[test]
    fn define_and_undefine_function() {
        let dir = TempDir::new().unwrap();
        let Some(mut session) = fixture(&dir) else {
            return;
        };
        let entry = session.entry_point();
        let end = Address::new(entry.get() + 0x10);
        session
            .execute(Command::DefineFunc { start: entry, end })
            .unwrap();
        assert_eq!(session.func_at(entry).unwrap(), Some(end));
        session.undo().unwrap();
        assert_eq!(session.func_at(entry).unwrap(), None);
        session.redo().unwrap();
        assert_eq!(session.func_at(entry).unwrap(), Some(end));
    }

    #[test]
    fn disasm_reflects_rename_and_comment() {
        let dir = TempDir::new().unwrap();
        let Some(mut session) = fixture(&dir) else {
            return;
        };
        let entry = session.entry_point();
        session
            .execute(Command::Rename {
                addr: entry,
                name: Some("entry_renamed".to_owned()),
            })
            .unwrap();
        session
            .execute(Command::SetComment {
                addr: entry,
                kind: CommentKind::Regular,
                text: Some("hello".to_owned()),
            })
            .unwrap();
        let lines = session.disasm(entry, 1).unwrap();
        assert_eq!(lines[0].name.as_deref(), Some("entry_renamed"));
        assert_eq!(lines[0].comment.as_deref(), Some("hello"));
    }

    #[test]
    fn cpp_export_names_are_demangled() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../corpus/build/cpp/corpus-cpp.dll");
        let Ok(bytes) = std::fs::read(path) else {
            return;
        };
        let dir = TempDir::new().unwrap();
        let project = dir.path().join("cpp.rcraft");
        let session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();

        let mut demangled = Vec::new();
        for export in &session.image().exports {
            let Some(name) = export.name.as_deref() else {
                continue;
            };
            let va = Address::new(session.image().image_base.get() + u64::from(export.rva));
            if let Some(stored) = session.name_at(va).unwrap() {
                assert!(
                    !stored.starts_with('?'),
                    "export {name} was not demangled: {stored}"
                );
                demangled.push(stored);
            }
        }
        assert!(!demangled.is_empty(), "no export names were demangled");
        assert!(
            demangled
                .iter()
                .any(|name| name.contains("DemoTarget::get")),
            "DemoTarget::get missing: {demangled:?}"
        );
        // Overloads include parameter types so they stay distinguishable.
        let overloads: Vec<&String> = demangled
            .iter()
            .filter(|name| name.contains("overloaded"))
            .collect();
        assert!(overloads.len() >= 2, "{demangled:?}");
        assert_ne!(overloads[0], overloads[1]);
    }
}
