// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Database schema and migrations.
//!
//! Rules:
//! - Every schema change bumps [`CURRENT_VERSION`] and adds a migration step.
//! - "User data" tables (names, comments, patches, journal, ...) are authoritative.
//! - "Derived data" tables (flags pages, xrefs, strings, pseudocode, ...) may be
//!   dropped and rebuilt from the image plus user data at any time.

use recraft_common::{Error, Result};
use rusqlite::{Connection, OptionalExtension};

/// Current schema version. Stored in `meta.schema_version`.
pub const CURRENT_VERSION: i64 = 1;

/// DDL for schema version 1.
///
/// All statements are idempotent (`IF NOT EXISTS`) so a fresh database and a
/// migration from an older version can share the same path.
pub const SCHEMA_V1: &str = r#"
CREATE TABLE IF NOT EXISTS meta (
    key   TEXT PRIMARY KEY,
    value BLOB NOT NULL
);

-- Original image bytes, split into IMAGE_CHUNK_SIZE blocks keyed by file offset.
CREATE TABLE IF NOT EXISTS image_chunk (
    off  INTEGER PRIMARY KEY,
    data BLOB NOT NULL
);

-- Mapped segments (headers, sections, ...).
CREATE TABLE IF NOT EXISTS segment (
    id       INTEGER PRIMARY KEY,
    name     TEXT    NOT NULL,
    start    INTEGER NOT NULL,
    end      INTEGER NOT NULL,
    file_off INTEGER NOT NULL,
    kind     INTEGER NOT NULL,
    perm     INTEGER NOT NULL,
    bitness  INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS segment_start ON segment(start);

-- ---------------------------------------------------------------- user data --
CREATE TABLE IF NOT EXISTS name (
    addr  INTEGER PRIMARY KEY,
    text  TEXT    NOT NULL,
    kind  INTEGER NOT NULL,
    flags INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS name_text ON name(text);

CREATE TABLE IF NOT EXISTS comment (
    addr INTEGER NOT NULL,
    kind INTEGER NOT NULL,
    text TEXT    NOT NULL,
    PRIMARY KEY (addr, kind)
);

CREATE TABLE IF NOT EXISTS func (
    id        INTEGER PRIMARY KEY,
    start     INTEGER NOT NULL UNIQUE,
    end       INTEGER NOT NULL,
    flags     INTEGER NOT NULL DEFAULT 0,
    cc        INTEGER NOT NULL DEFAULT 0,
    name_addr INTEGER,
    ret_type  INTEGER
);
CREATE INDEX IF NOT EXISTS func_start ON func(start);

CREATE TABLE IF NOT EXISTS func_chunk (
    func_id INTEGER NOT NULL,
    start   INTEGER NOT NULL,
    end     INTEGER NOT NULL,
    PRIMARY KEY (func_id, start)
);

CREATE TABLE IF NOT EXISTS type_def (
    id   INTEGER PRIMARY KEY,
    decl TEXT    NOT NULL,
    size INTEGER NOT NULL,
    kind INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS struct_member (
    type_id INTEGER NOT NULL,
    off     INTEGER NOT NULL,
    name    TEXT    NOT NULL,
    member_type INTEGER NOT NULL,
    PRIMARY KEY (type_id, off)
);

CREATE TABLE IF NOT EXISTS var (
    func_id   INTEGER NOT NULL,
    stack_off INTEGER NOT NULL,
    name      TEXT    NOT NULL,
    type_id   INTEGER NOT NULL DEFAULT 0,
    kind      INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (func_id, stack_off)
);

CREATE TABLE IF NOT EXISTS patch (
    addr INTEGER PRIMARY KEY,
    orig BLOB    NOT NULL,
    new  BLOB    NOT NULL
);

CREATE TABLE IF NOT EXISTS bookmark (
    addr  INTEGER PRIMARY KEY,
    note  TEXT    NOT NULL,
    color INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS journal (
    seq     INTEGER PRIMARY KEY AUTOINCREMENT,
    ts      INTEGER NOT NULL,
    cmd     TEXT    NOT NULL,
    payload BLOB    NOT NULL,
    undone  INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS journal_undone ON journal(undone, seq);

CREATE TABLE IF NOT EXISTS config (
    key   TEXT PRIMARY KEY,
    value BLOB NOT NULL
);

-- ------------------------------------------------------------- derived data --
-- 64 KiB of image -> 32 KiB of flags (4 bits per byte).
CREATE TABLE IF NOT EXISTS flags_page (
    page_no INTEGER PRIMARY KEY,
    data    BLOB    NOT NULL
);

CREATE TABLE IF NOT EXISTS xref (
    from_addr INTEGER NOT NULL,
    to_addr   INTEGER NOT NULL,
    type      INTEGER NOT NULL,
    PRIMARY KEY (from_addr, to_addr, type)
);
CREATE INDEX IF NOT EXISTS xref_to ON xref(to_addr);

CREATE TABLE IF NOT EXISTS string (
    addr INTEGER PRIMARY KEY,
    len  INTEGER NOT NULL,
    enc  INTEGER NOT NULL,
    kind INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS api_tag (
    addr    INTEGER NOT NULL,
    rule_id TEXT    NOT NULL,
    category TEXT   NOT NULL,
    PRIMARY KEY (addr, rule_id)
);

CREATE TABLE IF NOT EXISTS pseudocode (
    func_id  INTEGER PRIMARY KEY,
    text     BLOB    NOT NULL,
    span_map BLOB    NOT NULL,
    version  TEXT    NOT NULL
);
"#;

fn db_err(e: rusqlite::Error) -> Error {
    Error::storage(e.to_string())
}

/// Read `meta.schema_version`, if present.
pub fn schema_version(conn: &Connection) -> Result<Option<i64>> {
    let raw: Option<Vec<u8>> = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(db_err)?;
    let Some(raw) = raw else { return Ok(None) };
    let text = String::from_utf8(raw).map_err(|e| Error::storage(e.to_string()))?;
    text.parse::<i64>()
        .map(Some)
        .map_err(|_| Error::storage(format!("invalid schema_version: {text}")))
}

fn set_schema_version(conn: &Connection, version: i64) -> Result<()> {
    let text = version.to_string();
    conn.execute(
        "INSERT INTO meta(key, value) VALUES ('schema_version', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [text.as_bytes()],
    )
    .map_err(db_err)?;
    Ok(())
}

/// Create the schema on a fresh database or migrate an existing one.
///
/// Returns the version the database is now at.
pub fn initialize(conn: &Connection) -> Result<i64> {
    // A brand-new SQLite file has no `meta` table at all, so `schema_version`
    // would fail with "no such table" instead of reporting "no version yet".
    let has_meta: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'meta')",
            [],
            |row| row.get(0),
        )
        .map_err(db_err)?;
    if !has_meta {
        conn.execute_batch(SCHEMA_V1).map_err(db_err)?;
        set_schema_version(conn, CURRENT_VERSION)?;
        return Ok(CURRENT_VERSION);
    }

    let current = schema_version(conn)?;
    match current {
        None => {
            conn.execute_batch(SCHEMA_V1).map_err(db_err)?;
            set_schema_version(conn, CURRENT_VERSION)?;
            Ok(CURRENT_VERSION)
        }
        Some(v) if v == CURRENT_VERSION => Ok(v),
        Some(v) if v < CURRENT_VERSION => {
            migrate(conn, v)?;
            set_schema_version(conn, CURRENT_VERSION)?;
            Ok(CURRENT_VERSION)
        }
        Some(v) => Err(Error::storage(format!(
            "project schema version {v} is newer than this build ({CURRENT_VERSION}); update ReCraft"
        ))),
    }
}

/// Run migrations from `from` up to [`CURRENT_VERSION`].
///
/// v1 is the initial schema, so the only supported upgrade path right now is a
/// no-op replay of the idempotent DDL (kept for symmetry with future versions).
fn migrate(conn: &Connection, from: i64) -> Result<()> {
    #[allow(clippy::single_match)]
    match from {
        0 => conn.execute_batch(SCHEMA_V1).map_err(db_err)?,
        _ => {
            // Future: `1 => { ... }`, `2 => { ... }`, each additive and idempotent.
            conn.execute_batch(SCHEMA_V1).map_err(db_err)?;
        }
    }
    Ok(())
}
