// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Single-file project container: creation, image import, journal, read helpers.
//!
//! Concurrency model (see `docs/DESIGN.md` §2): exactly one writer connection
//! (owned by the write thread) and any number of reader connections opened with
//! `PRAGMA query_only=ON`. SQLite WAL lets readers run while the writer commits.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use recraft_common::{Error, Result};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use sha2::{Digest, Sha256};

use crate::schema;

/// Size of one `image_chunk` row.
pub const IMAGE_CHUNK_SIZE: u64 = 256 * 1024;

const BUSY_TIMEOUT: Duration = Duration::from_secs(5);
/// SQLite page cache, in KiB (negative means KiB in `PRAGMA cache_size`).
const CACHE_SIZE_KIB: i64 = 256 * 1024;
const MMAP_SIZE: i64 = 256 * 1024 * 1024;
const SQLITE_PAGE_SIZE: i64 = 65536;

/// One undo-journal record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalEntry {
    pub seq: i64,
    pub ts: i64,
    pub cmd: String,
    pub payload: Vec<u8>,
    /// Who issued the command (`gui`, `cli`, `mcp`, `ai`, `user`).
    pub origin: String,
    /// Commands applied by one bulk call share a batch id.
    pub batch: Option<i64>,
}

fn db_err(e: rusqlite::Error) -> Error {
    Error::storage(e.to_string())
}

/// Map one journal row in [`Project::JOURNAL_COLUMNS`] order.
fn journal_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<JournalEntry> {
    Ok(JournalEntry {
        seq: row.get(0)?,
        ts: row.get(1)?,
        cmd: row.get(2)?,
        payload: row.get(3)?,
        origin: row.get(4)?,
        batch: row.get(5)?,
    })
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// Execute a PRAGMA whether or not it returns rows.
fn pragma(conn: &Connection, sql: &str) -> Result<()> {
    let mut stmt = conn.prepare(sql).map_err(db_err)?;
    let mut rows = stmt.query([]).map_err(db_err)?;
    while rows.next().map_err(db_err)?.is_some() {}
    Ok(())
}

fn configure_writer(conn: &Connection, is_new: bool) -> Result<()> {
    conn.busy_timeout(BUSY_TIMEOUT).map_err(db_err)?;
    // `page_size` must be set before the first table is created, and before WAL
    // touches the database header, otherwise SQLite silently keeps the default.
    if is_new {
        pragma(conn, &format!("PRAGMA page_size={SQLITE_PAGE_SIZE}"))?;
    }
    pragma(conn, "PRAGMA journal_mode=WAL")?;
    pragma(conn, "PRAGMA synchronous=NORMAL")?;
    pragma(conn, "PRAGMA foreign_keys=ON")?;
    pragma(conn, "PRAGMA temp_store=MEMORY")?;
    pragma(conn, &format!("PRAGMA cache_size=-{CACHE_SIZE_KIB}"))?;
    pragma(conn, &format!("PRAGMA mmap_size={MMAP_SIZE}"))?;
    Ok(())
}

fn remove_project_files(path: &Path) {
    let _ = fs::remove_file(path);
    let _ = fs::remove_file(path.with_extension("rcraft-wal"));
    let _ = fs::remove_file(path.with_extension("rcraft-shm"));
    // SQLite names sidecars by appending to the full path, e.g. `x.rcraft-wal`.
    for suffix in ["-wal", "-shm"] {
        let mut sidecar = path.as_os_str().to_os_string();
        sidecar.push(suffix);
        let _ = fs::remove_file(PathBuf::from(sidecar));
    }
}

/// The project database (writer side).
pub struct Project {
    conn: Connection,
    path: PathBuf,
}

impl Project {
    /// Create a new project file and copy `image` into it.
    ///
    /// `progress(done, total)` is called once per chunk. `cancelled()` is polled
    /// before every chunk; returning `true` aborts the import and removes the
    /// partially written project file.
    pub fn create(
        path: &Path,
        image: &[u8],
        original_name: &str,
        format: &str,
        progress: &mut dyn FnMut(u64, u64),
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Self> {
        if path.exists() {
            return Err(Error::storage(format!(
                "project already exists: {}",
                path.display()
            )));
        }
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)?;
        }

        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        )
        .map_err(db_err)?;
        configure_writer(&conn, true)?;
        schema::initialize(&conn)?;

        let mut conn = conn;
        let sha256 = match Self::import_image(&mut conn, image, progress, cancelled) {
            Ok(sha) => sha,
            Err(e) => {
                drop(conn);
                remove_project_files(path);
                return Err(e);
            }
        };

        let now = unix_now();
        let meta = [
            ("original_name", original_name.to_owned()),
            ("format", format.to_owned()),
            ("image_len", image.len().to_string()),
            ("sha256", sha256),
            ("created_at", now.to_string()),
        ];
        for (key, value) in meta {
            conn.execute(
                "INSERT INTO meta(key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value.as_bytes()],
            )
            .map_err(db_err)?;
        }

        Ok(Project {
            conn,
            path: path.to_path_buf(),
        })
    }

    /// Convenience wrapper around [`Project::create`] with no progress/cancel.
    pub fn create_quiet(
        path: &Path,
        image: &[u8],
        original_name: &str,
        format: &str,
    ) -> Result<Self> {
        Self::create(path, image, original_name, format, &mut |_, _| {}, &|| {
            false
        })
    }

    fn import_image(
        conn: &mut Connection,
        image: &[u8],
        progress: &mut dyn FnMut(u64, u64),
        cancelled: &dyn Fn() -> bool,
    ) -> Result<String> {
        let total = image.len() as u64;
        let tx = conn.transaction().map_err(db_err)?;
        let mut hasher = Sha256::new();
        let mut done: u64 = 0;
        {
            let mut stmt = tx
                .prepare("INSERT INTO image_chunk(off, data) VALUES (?1, ?2)")
                .map_err(db_err)?;
            for (index, chunk) in image.chunks(IMAGE_CHUNK_SIZE as usize).enumerate() {
                if cancelled() {
                    // Dropping `tx` rolls the whole import back.
                    return Err(Error::Cancelled);
                }
                hasher.update(chunk);
                let off = index as u64 * IMAGE_CHUNK_SIZE;
                stmt.execute(params![off as i64, chunk]).map_err(db_err)?;
                done += chunk.len() as u64;
                progress(done, total);
            }
        }
        tx.commit().map_err(db_err)?;
        Ok(to_hex(&hasher.finalize()))
    }

    /// Open an existing project, running migrations if needed.
    pub fn open(path: &Path) -> Result<Self> {
        let conn =
            Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE).map_err(db_err)?;
        configure_writer(&conn, false)?;
        schema::initialize(&conn)?;
        Ok(Project {
            conn,
            path: path.to_path_buf(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Raw access for `recraft-core` repositories. Storage owns the schema; core
    /// owns the semantic queries.
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }

    pub fn image_len(&self) -> Result<u64> {
        image_len(&self.conn)
    }

    pub fn sha256(&self) -> Result<Option<String>> {
        meta_string(&self.conn, "sha256")
    }

    pub fn meta_string(&self, key: &str) -> Result<Option<String>> {
        meta_string(&self.conn, key)
    }

    pub fn set_meta_string(&self, key: &str, value: &str) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO meta(key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value.as_bytes()],
            )
            .map_err(db_err)?;
        Ok(())
    }

    /// Read `len` bytes starting at file offset `offset`, clamped to the image.
    ///
    /// Missing chunks end the read early instead of erroring: callers treat that
    /// as "no data mapped here", which is the normal case for sparse regions.
    pub fn read_image(&self, offset: u64, len: usize) -> Result<Vec<u8>> {
        let total = self.image_len()?;
        if offset >= total || len == 0 {
            return Ok(Vec::new());
        }
        let end = offset.saturating_add(len as u64).min(total);
        let mut out = Vec::with_capacity((end - offset) as usize);
        let mut cursor = offset;
        while cursor < end {
            let chunk_index = cursor / IMAGE_CHUNK_SIZE;
            let chunk_off = chunk_index * IMAGE_CHUNK_SIZE;
            let Some(chunk) = self.read_chunk_raw(chunk_off)? else {
                break;
            };
            let inner = (cursor - chunk_off) as usize;
            if inner >= chunk.len() {
                break;
            }
            let take = (chunk.len() - inner).min((end - cursor) as usize);
            out.extend_from_slice(&chunk[inner..inner + take]);
            cursor += take as u64;
        }
        Ok(out)
    }

    fn read_chunk_raw(&self, off: u64) -> Result<Option<Vec<u8>>> {
        self.conn
            .query_row(
                "SELECT data FROM image_chunk WHERE off = ?1",
                [off as i64],
                |row| row.get::<_, Vec<u8>>(0),
            )
            .optional()
            .map_err(db_err)
    }

    // ------------------------------------------------------------- journal --

    /// Columns every journal query returns, in [`JournalEntry`] order.
    const JOURNAL_COLUMNS: &'static str = "seq, ts, cmd, payload, origin, batch";

    /// Append a user command to the undo journal and return its sequence number.
    pub fn append_journal(&self, cmd: &str, payload: &[u8]) -> Result<i64> {
        self.append_journal_from(cmd, payload, "user", None)
    }

    /// Append a command with explicit provenance and batch grouping.
    ///
    /// `origin` records which front end issued it (`gui`, `cli`, `mcp`, `ai`, `user`);
    /// `batch` groups the commands of one bulk call so undo can revert them as a unit.
    pub fn append_journal_from(
        &self,
        cmd: &str,
        payload: &[u8],
        origin: &str,
        batch: Option<i64>,
    ) -> Result<i64> {
        self.conn
            .execute(
                "INSERT INTO journal(ts, cmd, payload, origin, batch) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![unix_now(), cmd, payload, origin, batch],
            )
            .map_err(db_err)?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Next batch id (one more than the highest ever used).
    pub fn next_batch_id(&self) -> Result<i64> {
        self.conn
            .query_row(
                "SELECT COALESCE(MAX(batch), 0) + 1 FROM journal",
                [],
                |row| row.get(0),
            )
            .map_err(db_err)
    }

    /// Entries with `seq > after`, oldest first.
    ///
    /// `include_undone = false` returns only commands that are currently applied
    /// (the normal replay path when reopening a project).
    pub fn journal_since(&self, after: i64, include_undone: bool) -> Result<Vec<JournalEntry>> {
        let sql = if include_undone {
            format!(
                "SELECT {} FROM journal WHERE seq > ?1 ORDER BY seq",
                Self::JOURNAL_COLUMNS
            )
        } else {
            format!(
                "SELECT {} FROM journal WHERE seq > ?1 AND undone = 0 ORDER BY seq",
                Self::JOURNAL_COLUMNS
            )
        };
        let mut stmt = self.conn.prepare(&sql).map_err(db_err)?;
        let rows = stmt.query_map([after], journal_row).map_err(db_err)?;
        rows.map(|row| row.map_err(db_err)).collect()
    }

    /// One journal record by sequence number.
    pub fn journal_entry(&self, seq: i64) -> Result<Option<JournalEntry>> {
        self.conn
            .query_row(
                &format!(
                    "SELECT {} FROM journal WHERE seq = ?1",
                    Self::JOURNAL_COLUMNS
                ),
                [seq],
                journal_row,
            )
            .optional()
            .map_err(db_err)
    }

    /// Newest record that is currently applied.
    pub fn last_active_entry(&self) -> Result<Option<JournalEntry>> {
        self.conn
            .query_row(
                &format!(
                    "SELECT {} FROM journal WHERE undone = 0 ORDER BY seq DESC LIMIT 1",
                    Self::JOURNAL_COLUMNS
                ),
                [],
                journal_row,
            )
            .optional()
            .map_err(db_err)
    }

    /// Every still-applied record of one batch, newest first.
    pub fn active_batch(&self, batch: i64) -> Result<Vec<JournalEntry>> {
        let mut stmt = self
            .conn
            .prepare(&format!(
                "SELECT {} FROM journal WHERE batch = ?1 AND undone = 0 ORDER BY seq DESC",
                Self::JOURNAL_COLUMNS
            ))
            .map_err(db_err)?;
        let rows = stmt.query_map([batch], journal_row).map_err(db_err)?;
        rows.map(|row| row.map_err(db_err)).collect()
    }

    /// Mark one record as undone and return it.
    pub fn undo_entry(&self, seq: i64) -> Result<Option<JournalEntry>> {
        self.conn
            .execute("UPDATE journal SET undone = 1 WHERE seq = ?1", [seq])
            .map_err(db_err)?;
        self.journal_entry(seq)
    }

    /// Mark the newest applied command as undone and return it.
    pub fn undo_last_active(&self) -> Result<Option<JournalEntry>> {
        let entry = self.last_active_entry()?;
        let Some(entry) = entry else {
            return Ok(None);
        };
        let seq = entry.seq;
        self.conn
            .execute("UPDATE journal SET undone = 1 WHERE seq = ?1", [seq])
            .map_err(db_err)?;
        Ok(Some(entry))
    }

    /// Mark the oldest undone command as applied again and return it (redo).
    pub fn redo_next_undone(&self) -> Result<Option<JournalEntry>> {
        let entry = self
            .conn
            .query_row(
                &format!(
                    "SELECT {} FROM journal WHERE undone = 1 ORDER BY seq ASC LIMIT 1",
                    Self::JOURNAL_COLUMNS
                ),
                [],
                journal_row,
            )
            .optional()
            .map_err(db_err)?;
        let Some(entry) = entry else {
            return Ok(None);
        };
        self.conn
            .execute("UPDATE journal SET undone = 0 WHERE seq = ?1", [entry.seq])
            .map_err(db_err)?;
        Ok(Some(entry))
    }

    /// Delete all journal records newer than `seq` (clears the redo stack when a
    /// new command is issued after an undo).
    pub fn truncate_journal_after(&self, seq: i64) -> Result<usize> {
        self.conn
            .execute("DELETE FROM journal WHERE seq > ?1", [seq])
            .map_err(db_err)
    }

    pub fn journal_len(&self) -> Result<i64> {
        self.conn
            .query_row("SELECT COUNT(*) FROM journal", [], |row| row.get(0))
            .map_err(db_err)
    }

    /// Flush the WAL into the main database file (called on "save" / app exit).
    pub fn checkpoint(&self) -> Result<()> {
        let _: (i64, i64, i64) = self
            .conn
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .map_err(db_err)?;
        Ok(())
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        // Best effort: persist WAL into the main file so a copy of the .rcraft file
        // is self-contained.
        let _ = self.checkpoint();
    }
}

/// Reader connection (used by UI/MCP threads).
///
/// Opens read-write at the OS level (WAL needs write access to the `-shm` file)
/// but enforces `query_only`, so no accidental writes are possible.
pub struct ReadOnlyProject {
    conn: Connection,
}

impl ReadOnlyProject {
    pub fn open(path: &Path) -> Result<Self> {
        let conn =
            Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE).map_err(db_err)?;
        conn.busy_timeout(BUSY_TIMEOUT).map_err(db_err)?;
        pragma(&conn, "PRAGMA query_only=ON")?;
        pragma(&conn, &format!("PRAGMA cache_size=-{CACHE_SIZE_KIB}"))?;
        pragma(&conn, &format!("PRAGMA mmap_size={MMAP_SIZE}"))?;
        let version = schema::schema_version(&conn)?;
        if version.is_none() {
            return Err(Error::storage(format!(
                "not a ReCraft project: {}",
                path.display()
            )));
        }
        Ok(ReadOnlyProject { conn })
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn image_len(&self) -> Result<u64> {
        image_len(&self.conn)
    }

    pub fn read_image(&self, offset: u64, len: usize) -> Result<Vec<u8>> {
        let total = self.image_len()?;
        if offset >= total || len == 0 {
            return Ok(Vec::new());
        }
        let end = offset.saturating_add(len as u64).min(total);
        let mut out = Vec::with_capacity((end - offset) as usize);
        let mut cursor = offset;
        while cursor < end {
            let chunk_off = (cursor / IMAGE_CHUNK_SIZE) * IMAGE_CHUNK_SIZE;
            let chunk: Option<Vec<u8>> = self
                .conn
                .query_row(
                    "SELECT data FROM image_chunk WHERE off = ?1",
                    [chunk_off as i64],
                    |row| row.get(0),
                )
                .optional()
                .map_err(db_err)?;
            let Some(chunk) = chunk else { break };
            let inner = (cursor - chunk_off) as usize;
            if inner >= chunk.len() {
                break;
            }
            let take = (chunk.len() - inner).min((end - cursor) as usize);
            out.extend_from_slice(&chunk[inner..inner + take]);
            cursor += take as u64;
        }
        Ok(out)
    }
}

fn meta_string(conn: &Connection, key: &str) -> Result<Option<String>> {
    let raw: Option<Vec<u8>> = conn
        .query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| {
            row.get(0)
        })
        .optional()
        .map_err(db_err)?;
    match raw {
        None => Ok(None),
        Some(raw) => String::from_utf8(raw)
            .map(Some)
            .map_err(|e| Error::storage(e.to_string())),
    }
}

fn image_len(conn: &Connection) -> Result<u64> {
    if let Some(text) = meta_string(conn, "image_len")? {
        return text
            .parse::<u64>()
            .map_err(|_| Error::storage(format!("invalid image_len meta value: {text}")));
    }
    // Fallback for projects created by very old builds.
    let max_off: Option<i64> = conn
        .query_row("SELECT MAX(off) FROM image_chunk", [], |row| row.get(0))
        .map_err(db_err)?;
    let Some(max_off) = max_off else { return Ok(0) };
    let last_len: i64 = conn
        .query_row(
            "SELECT LENGTH(data) FROM image_chunk WHERE off = ?1",
            [max_off],
            |row| row.get(0),
        )
        .map_err(db_err)?;
    Ok((max_off.max(0) as u64) + last_len.max(0) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn pseudo_random(len: usize, seed: u64) -> Vec<u8> {
        let mut state = seed | 1;
        let mut out = vec![0u8; len];
        let mut i = 0;
        while i < len {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let take = (len - i).min(state.to_le_bytes().len());
            out[i..i + take].copy_from_slice(&state.to_le_bytes()[..take]);
            i += take;
        }
        out
    }

    fn sha256_hex(bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        to_hex(&hasher.finalize())
    }

    #[test]
    fn create_read_roundtrip_across_chunks() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sample.rcraft");
        let image = pseudo_random(1_000_003, 0x5eed);
        let mut progress_calls = 0u32;
        let project = Project::create(
            &path,
            &image,
            "sample.exe",
            "pe32+",
            &mut |done, total| {
                assert!(done <= total);
                progress_calls += 1;
            },
            &|| false,
        )
        .unwrap();

        assert!(progress_calls >= 4);
        assert_eq!(project.image_len().unwrap(), image.len() as u64);
        assert_eq!(
            project.sha256().unwrap().as_deref(),
            Some(sha256_hex(&image).as_str())
        );
        assert_eq!(
            project.meta_string("original_name").unwrap().as_deref(),
            Some("sample.exe")
        );

        let boundary = IMAGE_CHUNK_SIZE - 10;
        let want = &image[boundary as usize..boundary as usize + 20];
        assert_eq!(project.read_image(boundary, 20).unwrap(), want);

        let tail = project.read_image(image.len() as u64 - 5, 100).unwrap();
        assert_eq!(tail, &image[image.len() - 5..]);
        assert!(
            project
                .read_image(image.len() as u64, 10)
                .unwrap()
                .is_empty()
        );
        assert!(project.read_image(0, 0).unwrap().is_empty());

        drop(project);
        assert!(path.exists());

        let reopened = Project::open(&path).unwrap();
        assert_eq!(reopened.image_len().unwrap(), image.len() as u64);
        assert_eq!(
            reopened.read_image(IMAGE_CHUNK_SIZE, 16).unwrap(),
            &image[IMAGE_CHUNK_SIZE as usize..IMAGE_CHUNK_SIZE as usize + 16]
        );
    }

    #[test]
    fn cancelled_import_removes_partial_project() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cancelled.rcraft");
        let image = pseudo_random(4 * IMAGE_CHUNK_SIZE as usize, 7);
        let cancelled = Cell::new(false);
        let result = Project::create(
            &path,
            &image,
            "x.exe",
            "pe32",
            &mut |done, _| {
                if done >= 2 * IMAGE_CHUNK_SIZE {
                    cancelled.set(true);
                }
            },
            &|| cancelled.get(),
        );
        assert!(matches!(result, Err(Error::Cancelled)));
        assert!(!path.exists(), "partial project must be removed");
    }

    #[test]
    fn journal_undo_redo_truncate() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("journal.rcraft");
        let project = Project::create_quiet(&path, b"MZ", "x.exe", "pe32").unwrap();

        let a = project.append_journal("rename", b"a").unwrap();
        let b = project.append_journal("comment", b"b").unwrap();
        let c = project.append_journal("rename", b"c").unwrap();
        assert!(a < b && b < c);
        assert_eq!(project.journal_len().unwrap(), 3);

        let undone = project.undo_last_active().unwrap().unwrap();
        assert_eq!(undone.seq, c);
        assert_eq!(project.journal_since(0, false).unwrap().len(), 2);
        assert_eq!(project.journal_since(0, true).unwrap().len(), 3);

        let redone = project.redo_next_undone().unwrap().unwrap();
        assert_eq!(redone.seq, c);
        assert_eq!(project.journal_since(0, false).unwrap().len(), 3);

        project.undo_last_active().unwrap();
        project.undo_last_active().unwrap();
        assert_eq!(project.journal_since(0, false).unwrap().len(), 1);
        assert_eq!(project.truncate_journal_after(a).unwrap(), 2);
        assert_eq!(project.journal_len().unwrap(), 1);
    }

    #[test]
    fn reopen_preserves_journal_and_user_data() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("persist.rcraft");
        {
            let project =
                Project::create_quiet(&path, &pseudo_random(4096, 1), "a.exe", "pe32+").unwrap();
            project.append_journal("rename", b"main").unwrap();
            project
                .conn()
                .execute(
                    "INSERT INTO name(addr, text, kind, flags) VALUES (4198400, 'main', 1, 0)",
                    [],
                )
                .unwrap();
        }

        let project = Project::open(&path).unwrap();
        assert_eq!(project.journal_since(0, false).unwrap().len(), 1);
        let name: String = project
            .conn()
            .query_row("SELECT text FROM name WHERE addr = 4198400", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(name, "main");
        let version = schema::schema_version(project.conn()).unwrap();
        assert_eq!(version, Some(schema::CURRENT_VERSION));
    }

    #[test]
    fn reader_connection_sees_committed_data() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("reader.rcraft");
        let project = Project::create_quiet(
            &path,
            &pseudo_random(IMAGE_CHUNK_SIZE as usize, 3),
            "r.exe",
            "pe32",
        )
        .unwrap();
        project
            .conn()
            .execute(
                "INSERT INTO name(addr, text, kind, flags) VALUES (4096, 'sub_1000', 0, 0)",
                [],
            )
            .unwrap();

        let reader = ReadOnlyProject::open(&path).unwrap();
        assert_eq!(reader.image_len().unwrap(), IMAGE_CHUNK_SIZE);
        let count: i64 = reader
            .conn()
            .query_row("SELECT COUNT(*) FROM name", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
        let write_attempt = reader.conn().execute(
            "INSERT INTO name(addr, text, kind) VALUES (8192, 'x', 0)",
            [],
        );
        assert!(write_attempt.is_err(), "reader must be query_only");
    }

    #[test]
    fn create_refuses_to_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("exists.rcraft");
        Project::create_quiet(&path, b"MZ", "a", "pe32").unwrap();
        let second = Project::create_quiet(&path, b"MZ", "b", "pe32");
        assert!(second.is_err());
    }

    /// Manual benchmark, excluded from CI:
    /// `$env:RECRAFT_BENCH_MB=1024; cargo test -p recraft-storage -- --ignored bench_import --nocapture`
    #[test]
    #[ignore = "manual performance benchmark"]
    fn bench_import() {
        let mb: usize = std::env::var("RECRAFT_BENCH_MB")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(256);
        let image = pseudo_random(mb * 1024 * 1024, 0xbeef);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bench.rcraft");

        let t0 = std::time::Instant::now();
        let project = Project::create_quiet(&path, &image, "bench.bin", "raw").unwrap();
        let create = t0.elapsed();
        assert_eq!(project.image_len().unwrap(), image.len() as u64);

        let t1 = std::time::Instant::now();
        for i in 0..1000u64 {
            let off = (i * 7919) % image.len() as u64;
            let _ = project.read_image(off, 4096).unwrap();
        }
        let reads = t1.elapsed();
        drop(project);

        let t2 = std::time::Instant::now();
        let reopened = Project::open(&path).unwrap();
        let open = t2.elapsed();
        assert_eq!(reopened.image_len().unwrap(), image.len() as u64);
        let db_mib = std::fs::metadata(&path).unwrap().len() / (1024 * 1024);
        drop(reopened);

        println!(
            "bench {mb} MiB: create {create:?}, 1000 random 4KiB reads {reads:?}, reopen {open:?}, db {db_mib} MiB"
        );
    }
}
