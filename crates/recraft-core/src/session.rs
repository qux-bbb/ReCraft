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
use recraft_format::{Arch, Format, Image};
use recraft_storage::{JournalEntry, Project};
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

/// `meta` key holding the serialized [`Image`].
const IMAGE_META_KEY: &str = "image_meta";
/// The key 0.1.0 wrote the same JSON under; read for older projects.
const LEGACY_IMAGE_META_KEY: &str = "pe_meta";
/// Upper bound on the instruction sweep used to find the instruction before an address.
const SWEEP_LIMIT: usize = 50_000;

/// `func.flags` bit marking functions produced by automatic analysis (derived data).
pub const FUNC_FLAG_AUTO: u32 = 1;

/// Salt for the rendered-pseudocode cache. Bump it when the emitter's output changes in a way
/// [`Session::pseudocode_fingerprint`] cannot see (formatting, pass order, ...).
pub const PSEUDOCODE_CACHE_SALT: u32 = 1;

/// One journal record as reported to callers of [`Session::undo_report`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UndoRecord {
    pub seq: i64,
    pub cmd: String,
    pub origin: String,
    pub ts: i64,
    pub batch: Option<i64>,
}

impl UndoRecord {
    fn from_entry(entry: &JournalEntry) -> Self {
        UndoRecord {
            seq: entry.seq,
            cmd: entry.cmd.clone(),
            origin: entry.origin.clone(),
            ts: entry.ts,
            batch: entry.batch,
        }
    }
}

/// What one undo reverted: the records (newest first) and the ranges to redraw.
#[derive(Debug, Clone)]
pub struct UndoReport {
    pub records: Vec<UndoRecord>,
    pub effect: Effect,
}

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
    image: Image,
    format: Format,
    arch: Arch,
    mode: Mode,
    segments: Vec<Segment>,
    image_cache: ImageCache,
    flags: FlagsStore,
    decoders: DecodeCache,
    /// Provenance stamped on every journal record this session writes (`cli`, `mcp`,
    /// `gui`, `ai`, ...): the journal is shared by every front end.
    origin: String,
}

impl Session {
    /// Import an executable file into a new project at `path`.
    pub fn create(
        path: &Path,
        bytes: &[u8],
        progress: &mut dyn FnMut(u64, u64),
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Self> {
        let loaded = recraft_format::load(bytes)?;
        let image = loaded.image;
        let format = loaded.format;
        let arch = loaded.arch;
        let segments = segments::from_image(&image);
        let format_name = format.meta_name(arch);
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
            .map_err(|error| Error::internal(format!("serialize image metadata: {error}")))?;
        project.set_meta_string(IMAGE_META_KEY, &json)?;
        project.set_meta_string("arch", arch.name())?;
        write_segments(project.conn(), &segments)?;
        install_initial_names(project.conn(), &image)?;

        Ok(Self {
            mode: Mode::from_is_64(image.is_64),
            image,
            format,
            arch,
            segments,
            project,
            image_cache: ImageCache::new(IMAGE_CACHE_PAGES),
            flags: FlagsStore::new(FLAGS_CACHE_PAGES),
            decoders: DecodeCache::new(DECODE_CACHE_ENTRIES),
            origin: "user".to_owned(),
        })
    }

    /// Re-open an existing project (no image bytes are re-read up front).
    pub fn open(path: &Path) -> Result<Self> {
        let project = Project::open(path)?;
        // `image_meta` is the neutral key; 0.1.0 wrote the same JSON under `pe_meta`.
        let json = match project.meta_string(IMAGE_META_KEY)? {
            Some(json) => Some(json),
            None => project.meta_string(LEGACY_IMAGE_META_KEY)?,
        };
        let json = json.ok_or_else(|| {
            Error::storage("project is missing image metadata (re-import the binary)")
        })?;
        let image: Image = serde_json::from_str(&json)
            .map_err(|error| Error::internal(format!("corrupt image metadata: {error}")))?;
        let format = project
            .meta_string("format")?
            .and_then(|value| Format::from_meta(&value))
            .unwrap_or(Format::Pe);
        let arch = project
            .meta_string("arch")?
            .and_then(|value| Arch::from_name(&value))
            .unwrap_or(if image.is_64 { Arch::X64 } else { Arch::X86 });
        let segments = segments::from_image(&image);
        Ok(Self {
            mode: Mode::from_is_64(image.is_64),
            image,
            format,
            arch,
            segments,
            project,
            image_cache: ImageCache::new(IMAGE_CACHE_PAGES),
            flags: FlagsStore::new(FLAGS_CACHE_PAGES),
            decoders: DecodeCache::new(DECODE_CACHE_ENTRIES),
            origin: "user".to_owned(),
        })
    }

    // ------------------------------------------------------------- accessors --

    /// Provenance stamped on journal records (`user`, `cli`, `gui`, `mcp`, `ai`).
    pub fn origin(&self) -> &str {
        &self.origin
    }

    /// Set the provenance used by later commands.
    pub fn set_origin(&mut self, origin: impl Into<String>) {
        self.origin = origin.into();
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    pub fn project_mut(&mut self) -> &mut Project {
        &mut self.project
    }

    pub fn path(&self) -> &Path {
        self.project.path()
    }

    pub fn image(&self) -> &Image {
        &self.image
    }

    /// Container format the project was imported from.
    pub fn format(&self) -> Format {
        self.format
    }

    /// Instruction set of the imported image.
    pub fn arch(&self) -> Arch {
        self.arch
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

    /// All defined functions with their names, in one query.
    ///
    /// Listing functions used to issue one `name_at` query per row (500 rows → 500 round
    /// trips); a LEFT JOIN does the same work once.
    pub fn functions_named(&self) -> Result<Vec<(FunctionInfo, Option<String>)>> {
        let mut statement = self
            .project
            .conn()
            .prepare(
                "SELECT f.start, f.end, n.text FROM func f
                 LEFT JOIN name n ON n.addr = f.start
                 ORDER BY f.start",
            )
            .map_err(command::db_err)?;
        let rows = statement
            .query_map([], |row| {
                let start: i64 = row.get(0)?;
                let end: i64 = row.get(1)?;
                Ok((
                    FunctionInfo {
                        start: Address::new(start as u64),
                        end: Address::new(end as u64),
                    },
                    row.get(2)?,
                ))
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
        // New function boundaries mean the rendered pseudocode no longer matches.
        self.clear_pseudocode_cache()?;
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

    /// Cross-references originating inside `[start, end)`, optionally filtered by kind.
    ///
    /// A function summary used to scan the whole `xref` table twice per call (206 k rows on a
    /// 33 MB DLL); the range predicate keeps it to one indexed scan of the function's body.
    pub fn xrefs_from_range(
        &self,
        start: Address,
        end: Address,
        kind: Option<i64>,
    ) -> Result<Vec<XrefEntry>> {
        let mut stmt = self
            .project
            .conn()
            .prepare(
                "SELECT from_addr, to_addr, type FROM xref
                 WHERE from_addr >= ?1 AND from_addr < ?2
                   AND (?3 IS NULL OR type = ?3)
                 ORDER BY from_addr",
            )
            .map_err(command::db_err)?;
        let rows = stmt
            .query_map(params![start.get() as i64, end.get() as i64, kind], |row| {
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
        let mut effects = self.execute_batch(&[cmd])?;
        Ok(effects.remove(0))
    }

    /// Fingerprint of everything that changes a function's rendered pseudocode.
    ///
    /// The `pseudocode` table is keyed by it, so renaming a function, editing its prototype,
    /// rebinding an argument type, adding a struct layout or re-analysing the function (which
    /// changes its extent) all invalidate the cached text. `engine` is the decompiler version;
    /// bump [`PSEUDOCODE_CACHE_SALT`] when the emitter itself changes shape.
    pub fn pseudocode_fingerprint(
        &self,
        func: Address,
        level: &str,
        engine: &str,
    ) -> Result<String> {
        let end = self.func_at(func)?.map(|end| end.get()).unwrap_or(0);
        let name = self.name_at(func)?.unwrap_or_default();
        let prototype = self.prototype_at(func)?.unwrap_or_default();
        let mut args = String::new();
        for slot in 0..8u8 {
            if let Some(kind) = self.arg_type(func, slot)? {
                args.push_str(&kind);
                args.push('|');
            }
        }
        let (structs, fields): (i64, i64) = self
            .project
            .conn()
            .query_row(
                "SELECT (SELECT COUNT(*) FROM user_struct),
                        (SELECT COUNT(*) FROM user_struct_field)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(command::db_err)?;
        Ok(format!(
            "{engine}|{level}|v{PSEUDOCODE_CACHE_SALT}|{end:x}|{name}|{prototype}|{args}|{structs}|{fields}"
        ))
    }

    /// Cached pseudocode for `func`, when it was rendered from the same fingerprint.
    pub fn cached_pseudocode(&self, func: Address, fingerprint: &str) -> Result<Option<String>> {
        let row = self
            .project
            .conn()
            .query_row(
                "SELECT text, version FROM pseudocode WHERE func_id = ?1",
                [func.get() as i64],
                |row| Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(command::db_err)?;
        Ok(match row {
            Some((text, version)) if version == fingerprint => String::from_utf8(text).ok(),
            _ => None,
        })
    }

    /// Store a freshly rendered pseudocode document under `fingerprint`.
    pub fn store_pseudocode(&self, func: Address, fingerprint: &str, text: &str) -> Result<()> {
        self.project
            .conn()
            .execute(
                "INSERT INTO pseudocode(func_id, text, span_map, version) VALUES (?1, ?2, X'', ?3)
                 ON CONFLICT(func_id) DO UPDATE SET text = excluded.text, version = excluded.version",
                params![func.get() as i64, text.as_bytes(), fingerprint],
            )
            .map_err(command::db_err)?;
        Ok(())
    }

    /// Drop every rendered-pseudocode entry (called whenever the project changes).
    pub fn clear_pseudocode_cache(&self) -> Result<()> {
        self.project
            .conn()
            .execute("DELETE FROM pseudocode", [])
            .map_err(command::db_err)?;
        Ok(())
    }

    /// Apply several commands in one transaction: all of them land or none does.
    ///
    /// The journal records share one batch id, so a single [`Session::undo`] reverts the
    /// whole call — that is what makes an AI "rename these 20 functions" flow one undo.
    pub fn execute_batch(&mut self, commands: &[Command]) -> Result<Vec<Effect>> {
        if commands.is_empty() {
            return Ok(Vec::new());
        }
        // Inverses are computed against the *pre-change* state, so all of them are built
        // before the transaction opens.
        let mut payloads = Vec::with_capacity(commands.len());
        for command in commands {
            let record = JournalRecord {
                command: command.clone(),
                inverse: command.inverse(self)?,
            };
            payloads.push(
                serde_json::to_vec(&record)
                    .map_err(|error| Error::internal(format!("serialize journal: {error}")))?,
            );
        }
        let origin = self.origin.clone();

        {
            let Session {
                project,
                segments,
                flags,
                ..
            } = self;
            let batch = project.next_batch_id()?;
            let tx = project.conn_mut().transaction().map_err(command::db_err)?;
            // A new command invalidates everything that could be redone.
            tx.execute("DELETE FROM journal WHERE undone = 1", [])
                .map_err(command::db_err)?;
            for (command, payload) in commands.iter().zip(&payloads) {
                command::apply(&tx, segments, flags, command)?;
                tx.execute(
                    "INSERT INTO journal(ts, cmd, payload, origin, batch) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![unix_now(), command.name(), payload, origin, Some(batch)],
                )
                .map_err(command::db_err)?;
            }
            tx.commit().map_err(command::db_err)?;
        }

        self.decoders.clear();
        self.clear_pseudocode_cache()?;
        Ok(commands.iter().map(Command::effect).collect())
    }

    /// Undo the newest applied command (or the newest batch, as a unit).
    pub fn undo(&mut self) -> Result<Option<Effect>> {
        Ok(self.undo_report()?.map(|report| report.effect))
    }

    /// Undo the newest applied command (or batch) and report exactly what was reverted.
    ///
    /// Unlike [`Session::undo`], the caller learns the sequence numbers, command names and
    /// origins that were rolled back: an MCP client must be able to tell its own edits from
    /// a GUI user's.
    pub fn undo_report(&mut self) -> Result<Option<UndoReport>> {
        let Some(entry) = self.project.last_active_entry()? else {
            return Ok(None);
        };
        let entries = match entry.batch {
            Some(batch) => self.project.active_batch(batch)?,
            None => vec![entry],
        };
        self.undo_entries(entries).map(Some)
    }

    /// Undo one specific journal record (only the newest applied one, to keep the stack
    /// coherent), together with the rest of its batch, and report what was reverted.
    pub fn undo_seq(&mut self, seq: i64) -> Result<UndoReport> {
        let Some(newest) = self.project.last_active_entry()? else {
            return Err(Error::unsupported("nothing to undo"));
        };
        if newest.seq != seq {
            return Err(Error::unsupported(format!(
                "cannot undo record #{seq}: #{} was applied later — undo the newer changes first",
                newest.seq
            )));
        }
        let entries = match newest.batch {
            Some(batch) => self.project.active_batch(batch)?,
            None => vec![newest],
        };
        self.undo_entries(entries)
    }

    /// Sequence number of the newest journal record (`0` when the journal is empty).
    ///
    /// Front ends echo it back in their write results so a later `undo {seq}` can name the
    /// command it expects to revert.
    pub fn last_journal_seq(&self) -> Result<i64> {
        self.project
            .conn()
            .query_row("SELECT COALESCE(MAX(seq), 0) FROM journal", [], |row| {
                row.get(0)
            })
            .map_err(command::db_err)
    }

    /// What [`Session::undo`] would revert right now, without changing anything.
    pub fn undo_preview(&self) -> Result<Option<Vec<UndoRecord>>> {
        let Some(entry) = self.project.last_active_entry()? else {
            return Ok(None);
        };
        let entries = match entry.batch {
            Some(batch) => self.project.active_batch(batch)?,
            None => vec![entry],
        };
        Ok(Some(entries.iter().map(UndoRecord::from_entry).collect()))
    }

    fn undo_entries(&mut self, entries: Vec<JournalEntry>) -> Result<UndoReport> {
        // `entries` is newest-first; each inverse is applied in that order and flipped in
        // the same transaction, so a crash cannot leave the journal lying about the state.
        let mut records = Vec::with_capacity(entries.len());
        let mut dirty = Vec::new();
        for entry in &entries {
            let record: JournalRecord = serde_json::from_slice(&entry.payload)
                .map_err(|error| Error::internal(format!("corrupt journal: {error}")))?;
            dirty.extend(record.inverse.effect().dirty);
            records.push(UndoRecord::from_entry(entry));
        }

        {
            let Session {
                project,
                segments,
                flags,
                ..
            } = self;
            let tx = project.conn_mut().transaction().map_err(command::db_err)?;
            for entry in &entries {
                let record: JournalRecord = serde_json::from_slice(&entry.payload)
                    .map_err(|error| Error::internal(format!("corrupt journal: {error}")))?;
                tx.execute("UPDATE journal SET undone = 1 WHERE seq = ?1", [entry.seq])
                    .map_err(command::db_err)?;
                command::apply(&tx, segments, flags, &record.inverse)?;
            }
            tx.commit().map_err(command::db_err)?;
        }

        self.decoders.clear();
        self.clear_pseudocode_cache()?;
        Ok(UndoReport {
            records,
            effect: Effect { dirty },
        })
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
        self.clear_pseudocode_cache()?;
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

    /// Address of the instruction that ends exactly at `addr` (its predecessor), if any.
    ///
    /// This is the fallback used when no linear sweep can be anchored before `addr`: it scans
    /// the 15-byte window before `addr` and keeps the latest start whose decode ends on `addr`.
    /// Prefer [`Session::rewind`], which sweeps from a known boundary and cannot pick an
    /// overlapping decode.
    pub fn prev_insn(&mut self, addr: Address) -> Result<Option<Address>> {
        let Some(segment) = self.segment_containing(addr).cloned() else {
            return Ok(None);
        };
        let lowest = addr.get().saturating_sub(15).max(segment.range.start.get());
        let mut candidate = addr.get().saturating_sub(1);
        loop {
            if candidate < lowest {
                return Ok(None);
            }
            let mut raw = [0u8; 15];
            let read = self.read_at(Address::new(candidate), &mut raw)?;
            if read > 0
                && let Some(insn) = decode(&raw[..read], Address::new(candidate), self.mode)
                && insn.next_addr() == addr
            {
                return Ok(Some(Address::new(candidate)));
            }
            candidate = match candidate.checked_sub(1) {
                Some(previous) => previous,
                None => return Ok(None),
            };
        }
    }

    /// Nearest instruction boundary strictly before `addr`.
    ///
    /// Analysed function starts are guaranteed boundaries, so the latest one before `addr` is
    /// the closest place a linear sweep can start. Without analysis data (or before the first
    /// function of a segment) the segment start is used instead.
    fn instruction_anchor(&mut self, addr: Address) -> Result<Address> {
        let segment_start = self
            .segment_containing(addr)
            .map(|segment| segment.range.start)
            .unwrap_or(addr);
        let Ok(functions) = self.functions() else {
            return Ok(segment_start);
        };
        Ok(functions
            .iter()
            .filter(|function| function.start >= segment_start && function.start < addr)
            .map(|function| function.start)
            .max()
            .unwrap_or(segment_start))
    }

    /// Collect the instruction starts in `anchor..addr` (the last `limit` of them).
    ///
    /// Returns the starts plus whether the sweep landed exactly on `addr`; when it does not,
    /// `addr` is not on the instruction stream this project sees and no boundary is known.
    fn sweep_starts(
        &mut self,
        anchor: Address,
        addr: Address,
        limit: usize,
    ) -> Result<(VecDeque<Address>, bool)> {
        let mut starts: VecDeque<Address> = VecDeque::new();
        let mut probe = anchor;
        // Bound the sweep so an address far past the nearest function cannot stall the UI.
        for _ in 0..SWEEP_LIMIT {
            if probe >= addr {
                break;
            }
            starts.push_back(probe);
            if starts.len() > limit {
                starts.pop_front();
            }
            match self.insn_at(probe)? {
                Some(insn) => probe = insn.next_addr(),
                None => break,
            }
        }
        Ok((starts, probe == addr))
    }

    /// Walk `rows` instructions backwards from `addr`, stopping at the segment start.
    ///
    /// Use this instead of subtracting bytes: instructions are variable-length, so a byte
    /// offset lands in the middle of an instruction and the view then decodes nonsense. The
    /// walk sweeps forward from the nearest known boundary before `addr` (a function start, else
    /// the segment start) and takes the last `rows` instruction addresses; overlapping decodes
    /// and `int3` padding are therefore seen the same way the rest of the app sees them.
    pub fn rewind(&mut self, addr: Address, rows: usize) -> Result<Address> {
        if rows == 0 {
            return Ok(addr);
        }
        let mut cursor = addr;
        let mut remaining = rows;
        // Each sweep only covers one anchor..cursor span; crossing a function boundary needs
        // the next anchor, hence the (bounded) loop.
        for _ in 0..64 {
            if remaining == 0 {
                return Ok(cursor);
            }
            let anchor = self.instruction_anchor(cursor)?;
            if anchor >= cursor {
                break;
            }
            let (starts, aligned) = self.sweep_starts(anchor, cursor, remaining)?;
            if !aligned || starts.is_empty() {
                break;
            }
            let taken = starts.len();
            cursor = starts[0];
            remaining -= taken;
        }
        if remaining == 0 {
            return Ok(cursor);
        }
        // No sweep reached `cursor`: fall back to the byte window. Padding (`int3`) and data
        // regions decode unambiguously here; real code is covered by the sweep above.
        for _ in 0..remaining {
            match self.prev_insn(cursor)? {
                Some(previous) => cursor = previous,
                None => break,
            }
        }
        Ok(cursor)
    }

    /// Walk `rows` instructions forwards from `addr`, stopping at the segment end.
    pub fn advance(&mut self, addr: Address, rows: usize) -> Result<Address> {
        let mut cursor = addr;
        for _ in 0..rows {
            match self.insn_at(cursor)? {
                Some(insn) if self.segment_containing(insn.next_addr()).is_some() => {
                    cursor = insn.next_addr();
                }
                _ => break,
            }
        }
        Ok(cursor)
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

fn install_initial_names(conn: &Connection, image: &Image) -> Result<()> {
    let mut statement = conn
        .prepare("INSERT OR IGNORE INTO name(addr, text, kind, flags) VALUES (?1, ?2, ?3, 0)")
        .map_err(command::db_err)?;
    // Names the container carries itself (ELF `.symtab`/`.dynsym`). They go in first so
    // a real symbol at the entry point wins over the synthetic `start` label; PE leaves
    // `symbols` empty, so its naming order is unchanged.
    for symbol in &image.symbols {
        if symbol.name.is_empty() || symbol.address.get() == 0 {
            continue;
        }
        let display = recraft_common::demangle(&symbol.name).unwrap_or_else(|| symbol.name.clone());
        statement
            .execute(params![
                symbol.address.get() as i64,
                display,
                NAME_KIND_LOADER
            ])
            .map_err(command::db_err)?;
    }
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
    fn rewind_and_advance_stay_on_instruction_boundaries() {
        let dir = TempDir::new().unwrap();
        let Some(mut session) = fixture(&dir) else {
            return;
        };
        let entry = session.entry_point();
        let rows = session.disasm(entry, 40).unwrap();
        assert!(rows.len() >= 20, "fixture needs a few instructions");
        let addresses: Vec<Address> = rows.iter().map(|row| row.addr).collect();

        // Every row is `rows` instructions after the entry, even though instruction lengths
        // differ (subtracting a byte count used to land inside an instruction).
        for (index, expected) in addresses.iter().enumerate().take(17).skip(1) {
            assert_eq!(
                session.advance(entry, index).unwrap(),
                *expected,
                "advance({index}) is not instruction aligned"
            );
            assert_eq!(
                session.rewind(*expected, index).unwrap(),
                entry,
                "rewind({index}) is not instruction aligned"
            );
        }
        // Walking further back leaves the function and crosses the `int3` padding; the walk
        // must stay byte-exact instead of stalling on a mid-instruction decode.
        let padding = session.rewind(entry, 1).unwrap();
        assert_eq!(padding.get(), entry.get() - 1);
        assert_ne!(session.rewind(entry, 40).unwrap(), entry);
    }

    #[test]
    fn rewind_stops_at_the_segment_start() {
        let dir = TempDir::new().unwrap();
        let Some(mut session) = fixture(&dir) else {
            return;
        };
        let first_code = session
            .segments()
            .iter()
            .filter(|segment| segment.perm.is_executable())
            .map(|segment| segment.range.start)
            .min()
            .unwrap();
        // No image bytes exist before the segment: rewinding saturates instead of walking out
        // of the image (which used to blank the disassembly view).
        assert_eq!(session.rewind(first_code, 50).unwrap(), first_code);
        assert!(session.prev_insn(first_code).unwrap().is_none());
    }

    /// Projects with analysis data have function starts, and the entry itself is usually one of
    /// them: rewinding from a function start must still find earlier instructions instead of
    /// falling back to a byte scan.
    #[test]
    fn rewind_works_from_a_function_start() {
        let dir = TempDir::new().unwrap();
        let Some(mut session) = fixture(&dir) else {
            return;
        };
        let entry = session.entry_point();
        let rows = session.disasm(entry, 8).unwrap();
        // The entry function, plus a function that ends right before the entry's padding.
        session
            .execute(Command::DefineFunc {
                start: entry,
                end: rows.last().unwrap().addr,
            })
            .unwrap();

        // 20 instructions before the entry are inside the padding that precedes it, and the
        // rows after them must still decode into the entry when swept forward.
        let back = session.rewind(entry, 1).unwrap();
        assert_eq!(
            back.get(),
            entry.get() - 1,
            "should step into the int3 padding"
        );
        let anchor = session.rewind(entry, 16).unwrap();
        let resumed = session.advance(anchor, 16).unwrap();
        assert_eq!(
            resumed, entry,
            "rewind/advance disagree across the function start"
        );
    }

    #[test]
    fn batch_is_one_journal_group_and_one_undo() {
        let dir = TempDir::new().unwrap();
        let Some(mut session) = fixture(&dir) else {
            return;
        };
        let entry = session.entry_point();
        let rows = session.disasm(entry, 4).unwrap();
        assert!(rows.len() >= 2, "fixture needs a few instructions");
        let (first, second) = (entry, rows[1].addr);
        let before: Vec<_> = [first, second]
            .iter()
            .map(|addr| session.name_at(*addr).unwrap())
            .collect();

        session.set_origin("mcp");
        let commands = [
            Command::Rename {
                addr: first,
                name: Some("batch_one".to_owned()),
            },
            Command::Rename {
                addr: second,
                name: Some("batch_two".to_owned()),
            },
        ];
        let effects = session.execute_batch(&commands).unwrap();
        assert_eq!(effects.len(), 2);
        assert_eq!(
            session.name_at(first).unwrap().as_deref(),
            Some("batch_one")
        );
        assert_eq!(
            session.name_at(second).unwrap().as_deref(),
            Some("batch_two")
        );

        // Both records carry the batch id and the caller's provenance.
        let preview = session.undo_preview().unwrap().unwrap();
        assert_eq!(preview.len(), 2, "a batch is undone as a unit");
        assert!(preview.iter().all(|record| record.origin == "mcp"));
        assert!(preview.iter().all(|record| record.batch.is_some()));
        let batch = preview[0].batch;
        assert!(preview.iter().all(|record| record.batch == batch));

        // One undo reverts the whole call.
        let report = session.undo_report().unwrap().unwrap();
        assert_eq!(report.records.len(), 2);
        assert_eq!(report.records[0].cmd, "rename");
        assert_eq!(session.name_at(first).unwrap(), before[0]);
        assert_eq!(session.name_at(second).unwrap(), before[1]);

        // A stale record cannot be undone out of order.
        session.set_origin("gui");
        session
            .execute(Command::Rename {
                addr: first,
                name: Some("later".to_owned()),
            })
            .unwrap();
        let older = preview[0].seq;
        let error = session.undo_seq(older).unwrap_err();
        assert!(
            error.to_string().contains("applied later"),
            "unexpected error: {error}"
        );
        let newest = session.last_journal_seq().unwrap();
        let report = session.undo_seq(newest).unwrap();
        assert_eq!(report.records.len(), 1);
        assert_eq!(report.records[0].origin, "gui");
    }

    #[test]
    fn functions_named_matches_functions_and_carries_names() {
        let dir = TempDir::new().unwrap();
        let Some(session) = fixture(&dir) else {
            return;
        };
        let plain = session.functions().unwrap();
        let named = session.functions_named().unwrap();
        assert_eq!(plain.len(), named.len());
        for (index, (function, name)) in named.iter().enumerate() {
            assert_eq!(function.start, plain[index].start);
            assert_eq!(function.end, plain[index].end);
            assert_eq!(*name, session.name_at(function.start).unwrap());
        }
    }

    #[test]
    fn pseudocode_cache_hits_and_invalidates_on_write() {
        let dir = TempDir::new().unwrap();
        let Some(mut session) = fixture(&dir) else {
            return;
        };
        let entry = session.entry_point();
        let fingerprint = session.pseudocode_fingerprint(entry, "l2", "test").unwrap();
        assert!(
            session
                .cached_pseudocode(entry, &fingerprint)
                .unwrap()
                .is_none()
        );
        session
            .store_pseudocode(entry, &fingerprint, "int f(void) {}\n")
            .unwrap();
        assert_eq!(
            session.cached_pseudocode(entry, &fingerprint).unwrap(),
            Some("int f(void) {}\n".to_owned())
        );

        // Renaming changes the fingerprint, and any command drops the cached text.
        session
            .execute(Command::Rename {
                addr: entry,
                name: Some("renamed_entry".to_owned()),
            })
            .unwrap();
        let after = session.pseudocode_fingerprint(entry, "l2", "test").unwrap();
        assert_ne!(fingerprint, after, "the name is part of the fingerprint");
        assert!(session.cached_pseudocode(entry, &after).unwrap().is_none());

        // A different level or engine also misses.
        session
            .store_pseudocode(entry, &after, "int f(void) {}\n")
            .unwrap();
        assert!(
            session
                .cached_pseudocode(entry, "recraft-decompile 999|")
                .unwrap()
                .is_none()
        );
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

    /// A rename must be emittable C and must not make two addresses share one symbol:
    /// AI proposals and `rename`/`apply` take free text, so this is the last line of
    /// defence before the listing becomes ambiguous.
    #[test]
    fn rename_rejects_invalid_reserved_and_duplicate_names() {
        let dir = TempDir::new().unwrap();
        let Some(mut session) = fixture(&dir) else {
            return;
        };
        let entry = session.entry_point();
        // Any other address can own a name; no second function record is needed.
        let other = Address::new(entry.get() + 0x100);

        for (name, expected) in [
            ("", "must not be empty"),
            ("1st_name", "not a C identifier"),
            ("has space", "not a C identifier"),
            ("dash-name", "not a C identifier"),
            ("while", "reserved word"),
            ("struct", "reserved word"),
        ] {
            let error = session
                .execute(Command::Rename {
                    addr: entry,
                    name: Some(name.to_owned()),
                })
                .expect_err(name);
            assert!(
                error.to_string().contains(expected),
                "{name:?}: {error} (want {expected:?})"
            );
        }

        // A name another address already owns is refused; its own name is not a conflict.
        session
            .execute(Command::Rename {
                addr: entry,
                name: Some("owned_name".to_owned()),
            })
            .unwrap();
        let error = session
            .execute(Command::Rename {
                addr: other,
                name: Some("owned_name".to_owned()),
            })
            .expect_err("duplicate must be refused");
        assert!(error.to_string().contains("already used"), "{error}");
        session
            .execute(Command::Rename {
                addr: entry,
                name: Some("owned_name".to_owned()),
            })
            .expect("re-applying a function's own name is a no-op");
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
