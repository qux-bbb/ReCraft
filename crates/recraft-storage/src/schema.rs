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
pub const CURRENT_VERSION: i64 = 5;

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

CREATE TABLE IF NOT EXISTS prototype (
    addr INTEGER PRIMARY KEY,
    text TEXT    NOT NULL
);

CREATE TABLE IF NOT EXISTS user_struct (
    name TEXT PRIMARY KEY,
    size INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS user_struct_field (
    struct_name TEXT    NOT NULL,
    off         INTEGER NOT NULL,
    name        TEXT    NOT NULL,
    kind        TEXT    NOT NULL,
    PRIMARY KEY (struct_name, off)
);

-- The extracted string table is derived data, but a user edit is not: an override keeps the
-- text the user typed (or marks the row deleted) and survives every rebuild, because
-- analysis rewrites `string` and never touches this table.
CREATE TABLE IF NOT EXISTS string_override (
    addr   INTEGER PRIMARY KEY,
    text   TEXT,
    hidden INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS func_arg_type (
    func_addr   INTEGER NOT NULL,
    slot        INTEGER NOT NULL,
    struct_name TEXT    NOT NULL,
    PRIMARY KEY (func_addr, slot)
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
    undone  INTEGER NOT NULL DEFAULT 0,
    -- Who issued the command (`gui`, `cli`, `mcp`, `ai`, `user`): the journal is shared
    -- by every front end, so the audit trail has to say which one wrote.
    origin  TEXT    NOT NULL DEFAULT 'user',
    -- Commands applied by one bulk call share a batch id; undo reverts the batch as a unit.
    batch   INTEGER
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
            migrate(conn, 0)?;
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
/// All DDL is additive and idempotent; v2 added the `prototype` table, v3 user struct
/// layouts + per-argument type bindings, v4 provenance (`origin`) and batch grouping on the
/// journal, v5 the string overrides (`string_override`). Columns cannot be added by
/// re-running `CREATE TABLE IF NOT EXISTS`, so a table that predates v4 gets an explicit
/// `ALTER TABLE` here.
fn migrate(conn: &Connection, from: i64) -> Result<()> {
    conn.execute_batch(SCHEMA_V1).map_err(db_err)?;
    if from < 4 {
        add_column_if_missing(conn, "journal", "origin", "TEXT NOT NULL DEFAULT 'user'")?;
        add_column_if_missing(conn, "journal", "batch", "INTEGER")?;
    }
    Ok(())
}

/// Whether `table` already has a column named `column`.
fn has_column(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    let mut statement = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(db_err)?;
    let mut rows = statement.query([]).map_err(db_err)?;
    while let Some(row) = rows.next().map_err(db_err)? {
        let name: String = row.get(1).map_err(db_err)?;
        if name == column {
            return Ok(true);
        }
    }
    Ok(false)
}

fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<()> {
    if has_column(conn, table, column)? {
        return Ok(());
    }
    conn.execute_batch(&format!(
        "ALTER TABLE {table} ADD COLUMN {column} {definition}"
    ))
    .map_err(db_err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn version_3_journal_gains_provenance_columns() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA_V1).unwrap();
        // Simulate a v3 project: the journal predates `origin`/`batch`, and it already holds
        // one record that must keep working after the migration.
        conn.execute_batch(
            "DROP TABLE journal;
             CREATE TABLE journal (
                 seq     INTEGER PRIMARY KEY AUTOINCREMENT,
                 ts      INTEGER NOT NULL,
                 cmd     TEXT    NOT NULL,
                 payload BLOB    NOT NULL,
                 undone  INTEGER NOT NULL DEFAULT 0
             );
             INSERT INTO journal(ts, cmd, payload) VALUES (1, 'rename', X'00');",
        )
        .unwrap();
        set_schema_version(&conn, 3).unwrap();

        initialize(&conn).unwrap();

        assert!(has_column(&conn, "journal", "origin").unwrap());
        assert!(has_column(&conn, "journal", "batch").unwrap());
        assert_eq!(schema_version(&conn).unwrap(), Some(CURRENT_VERSION));
        let (origin, batch): (String, Option<i64>) = conn
            .query_row(
                "SELECT origin, batch FROM journal WHERE seq = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(origin, "user", "old records get the default provenance");
        assert_eq!(batch, None);
    }

    #[test]
    fn version_4_gains_the_string_override_table() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA_V1).unwrap();
        // Simulate a v4 project: no `string_override` table yet.
        conn.execute_batch("DROP TABLE string_override;").unwrap();
        set_schema_version(&conn, 4).unwrap();

        initialize(&conn).unwrap();

        assert_eq!(schema_version(&conn).unwrap(), Some(CURRENT_VERSION));
        conn.execute(
            "INSERT INTO string_override(addr, text) VALUES (1, 'x')",
            [],
        )
        .unwrap();
    }

    #[test]
    fn initialize_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        assert_eq!(initialize(&conn).unwrap(), CURRENT_VERSION);
        assert_eq!(initialize(&conn).unwrap(), CURRENT_VERSION);
    }
}
