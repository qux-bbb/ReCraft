// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! The read-only query layer: one implementation of every read tool, shared by the MCP
//! server and the in-app AI agent.
//!
//! Both front ends used to carry their own copy of these queries (different names, caps and
//! result shapes), so a fix in one never reached the other. Everything a caller may read
//! through a tool lives here; the transport (JSON-RPC or an LLM tool loop) and the argument
//! parsing stay with the caller.
//!
//! # Budgets
//!
//! A tool call is bounded by a [`Budget`]: the row cap for list tools and the line/byte caps
//! for `get_pseudocode`. MCP and the agent use different budgets (a model turn must not be
//! handed 5 000 rows) but the same specs and the same executor, so the two surfaces cannot
//! drift apart again.

#![forbid(unsafe_code)]

mod tools;

use recraft_common::{Address, Error, Result};
use recraft_core::Session;
use serde_json::{Value, json};

pub use tools::{MAX_LIMIT, MAX_PSEUDOCODE_BYTES, MAX_PSEUDOCODE_LINES};

/// One XREF row kind: a call.
pub(crate) const XREF_CALL: i64 = 1;
/// One XREF row kind: a data reference.
pub(crate) const XREF_DATA: i64 = 3;

/// Per-call limits for a front end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget {
    /// Maximum rows a list-shaped tool may return.
    pub rows: usize,
    /// Default and maximum lines `get_pseudocode` returns per page.
    pub pseudo_lines: usize,
    /// Default and maximum bytes `get_pseudocode` returns per page.
    pub pseudo_bytes: usize,
}

impl Budget {
    /// MCP: the caller is a program, and the caps are what a client can accept as one result.
    pub const fn mcp() -> Self {
        Budget {
            rows: MAX_LIMIT,
            pseudo_lines: 2000,
            pseudo_bytes: 256 * 1024,
        }
    }

    /// In-app agent: the caller is a model with a context window, so the caps are much lower.
    pub const fn agent() -> Self {
        Budget {
            rows: 200,
            pseudo_lines: 400,
            pseudo_bytes: 64 * 1024,
        }
    }
}

/// Tool names served by this crate, in the order they are advertised.
pub const READ_TOOLS: [&str; 13] = [
    "list_functions",
    "get_disassembly",
    "get_pseudocode",
    "xrefs_to",
    "xrefs_from",
    "list_strings",
    "list_imports",
    "list_exports",
    "get_metadata",
    "callgraph",
    "search_bytes",
    "function_summary",
    "find_api_callers",
];

/// Whether `name` is one of the shared read tools.
pub fn is_read_tool(name: &str) -> bool {
    READ_TOOLS.contains(&name)
}

/// Execute one read tool. Errors are strings because every caller reports them as tool
/// results, not as transport failures.
pub fn call(
    session: &mut Session,
    name: &str,
    arguments: &Value,
    budget: &Budget,
) -> std::result::Result<Value, String> {
    let budget = *budget;
    match name {
        "list_functions" => tools::list_functions(session, arguments, budget),
        "get_disassembly" => tools::disassembly(session, arguments),
        "get_pseudocode" => tools::pseudocode(session, arguments, budget),
        "xrefs_to" => tools::xrefs(session, arguments, budget, true),
        "xrefs_from" => tools::xrefs(session, arguments, budget, false),
        "list_strings" => tools::strings(session, arguments, budget),
        "list_imports" => tools::imports(session),
        "list_exports" => tools::exports(session, arguments, budget),
        "get_metadata" => tools::metadata(session),
        "callgraph" => tools::callgraph(session, arguments, budget),
        "search_bytes" => tools::search_bytes(session, arguments, budget),
        "function_summary" => tools::summary(session, arguments),
        "find_api_callers" => tools::api_callers(session, arguments, budget),
        other => Err(format!("unknown tool: {other}")),
    }
}

/// Canonical JSON Schemas for the shared tools, bounded by `budget`.
///
/// Every spec carries a `title`, `annotations` and a (permissive) `outputSchema`: a client
/// can tell these are read-only and idempotent without guessing from the name, and can consume
/// the structured result instead of re-parsing the text block.
pub fn specs(budget: &Budget) -> Vec<Value> {
    let rows = budget.rows;
    let pseudo_lines = budget.pseudo_lines;
    let pseudo_bytes = budget.pseudo_bytes;
    let mut listed = vec![
        json!({
            "name": "list_functions",
            "description": "List discovered functions with names and ranges. `filter` runs before `offset`/`limit`; the result reports `total`, `truncated` and `next_offset` — pass it back as `offset` to read the next page.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "filter": {"type": "string", "description": "Case-insensitive name substring."},
                    "limit": {"type": "integer", "minimum": 1, "maximum": rows},
                    "offset": {"type": "integer", "minimum": 0, "description": "Rows to skip; use `next_offset` from the previous page."},
                },
            },
        }),
        json!({
            "name": "get_disassembly",
            "description": "Linear disassembly from an address.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "addr": {"type": ["integer", "string"], "description": "Address, hex string, name, or 'entry'."},
                    "count": {"type": "integer", "minimum": 1, "maximum": 4096},
                },
            },
        }),
        json!({
            "name": "get_pseudocode",
            "description": "Pseudo-C for the function containing an address (L0/L1/L2, L2 by default). Long functions are capped per page; read the rest with `startLine` from `next_start_line`.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "addr": {"type": ["integer", "string"]},
                    "level": {"type": "string", "enum": ["l0", "l1", "l2"], "description": "Default l2 (structured C with reconstructed locals)."},
                    "maxLines": {"type": "integer", "minimum": 1, "maximum": pseudo_lines},
                    "maxBytes": {"type": "integer", "minimum": 1024, "maximum": pseudo_bytes},
                    "startLine": {"type": "integer", "minimum": 1, "description": "1-based first line to return."},
                    "includeLines": {"type": "boolean", "description": "Also return `lines: [{addr,text}]` (duplicates `text`; off by default)."},
                },
            },
        }),
        json!({
            "name": "xrefs_to",
            "description": "References pointing at an address; pages with `offset`/`limit` and reports `total`/`truncated`/`next_offset`.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "addr": {"type": ["integer", "string"]},
                    "limit": {"type": "integer", "minimum": 1, "maximum": rows},
                    "offset": {"type": "integer", "minimum": 0},
                },
                "required": ["addr"],
            },
        }),
        json!({
            "name": "xrefs_from",
            "description": "References originating at an address; pages with `offset`/`limit` and reports `total`/`truncated`/`next_offset`.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "addr": {"type": ["integer", "string"]},
                    "limit": {"type": "integer", "minimum": 1, "maximum": rows},
                    "offset": {"type": "integer", "minimum": 0},
                },
                "required": ["addr"],
            },
        }),
        json!({
            "name": "list_strings",
            "description": "Extracted strings with encoding and address. `filter` runs before `offset`/`limit`; the result reports `total`, `truncated` and `next_offset`.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "filter": {"type": "string"},
                    "limit": {"type": "integer", "minimum": 1, "maximum": rows},
                    "offset": {"type": "integer", "minimum": 0},
                },
            },
        }),
        json!({
            "name": "list_imports",
            "description": "Imported modules and functions (IAT addresses).",
            "inputSchema": {"type": "object"},
        }),
        json!({
            "name": "list_exports",
            "description": "Export table (DLL entry surface): name, ordinal, address, forwarder. `filter` runs before `offset`/`limit`.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "filter": {"type": "string", "description": "Case-insensitive export name substring."},
                    "limit": {"type": "integer", "minimum": 1, "maximum": rows},
                    "offset": {"type": "integer", "minimum": 0},
                },
            },
        }),
        json!({
            "name": "get_metadata",
            "description": "Image metadata: format, architecture, image base, entry point, sizes, subsystem, timestamp, sections, counts and parser warnings.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "addr": {"type": ["integer", "string"], "description": "Ignored; metadata is image-wide."},
                },
            },
        }),
        json!({
            "name": "callgraph",
            "description": "Function-level call graph derived from persisted call xrefs. Pages both `nodes` and `edges` with `offset`/`limit` and reports `node_total`/`edge_total`.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": {"type": "integer", "minimum": 1, "maximum": rows},
                    "offset": {"type": "integer", "minimum": 0},
                },
            },
        }),
        json!({
            "name": "search_bytes",
            "description": "Search mapped readable segments for a byte pattern (hex or ASCII). `total` is the number of hits inspected, not the number that exist.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "hex": {"type": "string", "description": "Hex bytes, e.g. '8B 44 24 04'."},
                    "ascii": {"type": "string"},
                    "limit": {"type": "integer", "minimum": 1, "maximum": 1000},
                    "offset": {"type": "integer", "minimum": 0, "maximum": 1000},
                },
            },
        }),
        json!({
            "name": "function_summary",
            "description": "Compact summary of one function: range, instruction count, prototype, callers, calls, referenced strings, API tags.",
            "inputSchema": {
                "type": "object",
                "properties": {"addr": {"type": ["integer", "string"]}},
            },
        }),
        json!({
            "name": "find_api_callers",
            "description": "Functions that call an imported API (name substring, e.g. 'CreateFileW').",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "api": {"type": "string", "description": "Import name substring (case-insensitive)."},
                    "limit": {"type": "integer", "minimum": 1, "maximum": rows},
                },
                "required": ["api"],
            },
        }),
    ];
    for spec in &mut listed {
        let name = spec["name"].as_str().unwrap_or_default().to_owned();
        spec["title"] = json!(title_of(&name));
        // A client can auto-approve these without guessing from the name.
        spec["annotations"] = json!({
            "readOnlyHint": true,
            "destructiveHint": false,
            "idempotentHint": true,
            "openWorldHint": false,
        });
        // Every read tool returns an object; the permissive schema lets a 2025-06-18 client
        // consume `structuredContent` instead of re-parsing the text block.
        spec["outputSchema"] = json!({"type": "object", "additionalProperties": true});
    }
    listed
}

/// Human-readable label a client may show instead of the machine name.
fn title_of(name: &str) -> &'static str {
    match name {
        "list_functions" => "List functions",
        "get_disassembly" => "Disassembly",
        "get_pseudocode" => "Pseudocode",
        "xrefs_to" => "References to address",
        "xrefs_from" => "References from address",
        "list_strings" => "Strings",
        "list_imports" => "Imports",
        "list_exports" => "Exports",
        "get_metadata" => "Image metadata",
        "callgraph" => "Call graph",
        "search_bytes" => "Search bytes",
        "function_summary" => "Function summary",
        "find_api_callers" => "Find API callers",
        _ => "ReCraft tool",
    }
}

/// `0x…` rendering used by every result.
pub fn hex(addr: Address) -> String {
    format!("{:#x}", addr.get())
}

/// Resolve `0x…`, decimal, a symbol name or `entry`; a missing value means the entry point.
pub fn resolve_address(session: &Session, value: Option<&Value>) -> Result<Address> {
    let Some(value) = value else {
        return Ok(session.entry_point());
    };
    match value {
        Value::Number(number) => number
            .as_u64()
            .map(Address::new)
            .ok_or_else(|| Error::internal("address must be a u64")),
        // One syntax for every front end: CLI arguments, batch scripts and tools share it.
        Value::String(raw) => recraft_core::resolve_address(session, raw),
        _ => Err(Error::internal("address must be a number or string")),
    }
}

/// Row limit for a list tool, clamped to `budget.rows`.
pub(crate) fn limit_of(arguments: &Value, default: usize, budget: Budget) -> usize {
    arguments
        .get("limit")
        .and_then(Value::as_u64)
        .map(|limit| (limit as usize).clamp(1, budget.rows))
        .unwrap_or(default.min(budget.rows))
}

/// Row offset for a list tool.
pub(crate) fn offset_of(arguments: &Value) -> usize {
    arguments
        .get("offset")
        .and_then(Value::as_u64)
        .map(|offset| (offset as usize).min(MAX_LIMIT))
        .unwrap_or(0)
}

/// Read an integer cap (`maxLines`, `maxBytes`) and clamp it into range.
pub(crate) fn cap_of(
    arguments: &Value,
    key: &str,
    default: usize,
    min: usize,
    max: usize,
) -> usize {
    arguments
        .get(key)
        .and_then(Value::as_u64)
        .map(|value| (value as usize).clamp(min, max))
        .unwrap_or(default.min(max))
}

/// `count`/`total`/`offset`/`truncated`/`next_offset` block shared by the list tools.
pub(crate) fn page(
    rows: usize,
    total: usize,
    offset: usize,
    limit: usize,
) -> std::result::Result<Value, String> {
    let next = offset.saturating_add(rows);
    Ok(json!({
        "count": rows,
        "total": total,
        "offset": offset,
        "limit": limit,
        "truncated": next < total,
        "next_offset": if next < total { json!(next) } else { Value::Null },
    }))
}

/// `FunctionIndex` over an address-sorted function table.
pub(crate) struct FunctionIndex<'a> {
    functions: &'a [recraft_core::FunctionInfo],
}

impl<'a> FunctionIndex<'a> {
    pub(crate) fn new(functions: &'a [recraft_core::FunctionInfo]) -> Self {
        FunctionIndex { functions }
    }

    /// Index of the function containing `addr`, if any.
    pub(crate) fn containing(&self, addr: Address) -> Option<usize> {
        self.functions
            .iter()
            .position(|function| addr >= function.start && addr < function.end)
    }

    pub(crate) fn get(&self, index: usize) -> &recraft_core::FunctionInfo {
        &self.functions[index]
    }
}

pub(crate) fn xref_kind_name(kind: i64) -> &'static str {
    match kind {
        1 => "call",
        2 => "jump",
        3 => "data",
        _ => "other",
    }
}

pub(crate) fn parse_hex_bytes(text: &str) -> std::result::Result<Vec<u8>, String> {
    let cleaned: String = text
        .chars()
        .filter(|ch| !ch.is_ascii_whitespace() && *ch != ',' && *ch != '\\')
        .collect();
    if cleaned.len() % 2 != 0 {
        return Err("hex pattern needs an even number of digits".to_owned());
    }
    let mut bytes = Vec::with_capacity(cleaned.len() / 2);
    for index in (0..cleaned.len()).step_by(2) {
        let byte = u8::from_str_radix(&cleaned[index..index + 2], 16)
            .map_err(|_| format!("bad hex byte: {}", &cleaned[index..index + 2]))?;
        bytes.push(byte);
    }
    Ok(bytes)
}

pub(crate) fn decode_string(bytes: &[u8], encoding: i64) -> String {
    let mut end = bytes.len();
    if encoding == 2 {
        while end >= 2 && bytes[end - 2] == 0 && bytes[end - 1] == 0 {
            end -= 2;
        }
        let units: Vec<u16> = bytes[..end]
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        String::from_utf16_lossy(&units)
    } else {
        while end > 0 && bytes[end - 1] == 0 {
            end -= 1;
        }
        String::from_utf8_lossy(&bytes[..end]).into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// A real sample when one is available (Windows dev/CI), otherwise the test is skipped.
    fn project() -> Option<Session> {
        let bytes = std::fs::read("C:/Windows/System32/notepad.exe").ok()?;
        let dir = TempDir::new().ok()?;
        let path = dir.path().join("query.rcraft");
        let mut session = Session::create(&path, &bytes, &mut |_, _| {}, &|| false).ok()?;
        let analysis = recraft_analysis::analyze(&mut session).ok()?;
        recraft_analysis::persist(&mut session, &analysis).ok()?;
        let strings = recraft_analysis::extract_strings(&mut session, 4, 200_000).ok()?;
        recraft_analysis::persist_strings(&mut session, &strings).ok()?;
        // The project file must outlive the session.
        std::mem::forget(dir);
        Some(session)
    }

    #[test]
    fn specs_cover_every_read_tool_exactly_once() {
        let listed = specs(&Budget::mcp());
        let mut names: Vec<&str> = listed
            .iter()
            .filter_map(|spec| spec["name"].as_str())
            .collect();
        names.sort_unstable();
        let mut expected = READ_TOOLS.to_vec();
        expected.sort_unstable();
        assert_eq!(names, expected);
        for spec in &listed {
            assert!(
                spec["description"]
                    .as_str()
                    .is_some_and(|text| !text.is_empty())
            );
            assert_eq!(spec["inputSchema"]["type"], "object", "{spec}");
        }
        // The budget is visible in the schema, so a model sees the real cap.
        let agent = specs(&Budget::agent());
        let list = agent
            .iter()
            .find(|spec| spec["name"] == "list_functions")
            .expect("list_functions spec");
        assert_eq!(list["inputSchema"]["properties"]["limit"]["maximum"], 200);
    }

    #[test]
    fn pseudocode_defaults_to_the_structural_level() {
        let Some(mut session) = project() else {
            return;
        };
        let entry = session.entry_point();
        let value = call(
            &mut session,
            "get_pseudocode",
            &json!({"addr": format!("{:#x}", entry.get())}),
            &Budget::mcp(),
        )
        .expect("pseudocode");
        assert_eq!(value["level"], "l2", "the default level must match the GUI");
        assert!(value["text"].as_str().is_some_and(|text| !text.is_empty()));
        let paged = call(
            &mut session,
            "get_pseudocode",
            &json!({"addr": format!("{:#x}", entry.get()), "maxLines": 1}),
            &Budget::mcp(),
        )
        .expect("paged pseudocode");
        assert_eq!(paged["returned_lines"], 1);
        assert_eq!(paged["next_start_line"], 2);
    }

    #[test]
    fn the_agent_budget_caps_rows_below_the_mcp_budget() {
        let Some(mut session) = project() else {
            return;
        };
        let args = json!({"limit": MAX_LIMIT});
        let wide = call(&mut session, "list_functions", &args, &Budget::mcp()).expect("mcp rows");
        let narrow =
            call(&mut session, "list_functions", &args, &Budget::agent()).expect("agent rows");
        assert!(narrow["count"].as_u64().unwrap() <= Budget::agent().rows as u64);
        assert!(wide["count"].as_u64().unwrap() >= narrow["count"].as_u64().unwrap());
    }

    #[test]
    fn pseudocode_is_cached_after_the_first_read() {
        let Some(mut session) = project() else {
            return;
        };
        let addr = format!("{:#x}", session.entry_point().get());
        let first = call(
            &mut session,
            "get_pseudocode",
            &json!({"addr": addr}),
            &Budget::mcp(),
        )
        .expect("first read");
        assert_eq!(first["cached"], false);
        let second = call(
            &mut session,
            "get_pseudocode",
            &json!({"addr": addr}),
            &Budget::mcp(),
        )
        .expect("second read");
        assert_eq!(
            second["cached"], true,
            "second read must come from the cache"
        );
        assert_eq!(first["text"], second["text"]);

        // A write invalidates the cached text.
        call(
            &mut session,
            "list_functions",
            &json!({"limit": 1}),
            &Budget::mcp(),
        )
        .expect("list");
        let name = session.name_at(session.entry_point()).unwrap();
        session
            .execute(recraft_core::Command::Rename {
                addr: session.entry_point(),
                name: Some("cached_entry".to_owned()),
            })
            .unwrap();
        let third = call(
            &mut session,
            "get_pseudocode",
            &json!({"addr": addr}),
            &Budget::mcp(),
        )
        .expect("third read");
        assert_eq!(third["cached"], false, "a rename must drop the cache");
        assert_ne!(third["name"], json!(name));
    }

    /// Manual probe against a large sample: `cargo test -p recraft-query -- --ignored
    /// --nocapture`. Times the row-heavy and decompile-heavy paths on `.tmp/shell32.rcraft`.
    #[test]
    #[ignore = "needs .tmp/shell32.rcraft (33 MB, 31 803 functions)"]
    fn large_sample_probe() {
        use std::time::Instant;
        let path = std::path::Path::new("../../.tmp/shell32.rcraft");
        if !path.exists() {
            return;
        }
        let mut session = Session::open(path).expect("open shell32");
        let budget = Budget::mcp();

        // Query cost of the two ways to fill a 5 000-row page.
        let functions = session.functions().expect("functions");
        let started = Instant::now();
        for function in functions.iter().take(5000) {
            let _ = session.name_at(function.start).expect("name");
        }
        println!(
            "name_at x5000            {:6.1} ms",
            started.elapsed().as_secs_f64() * 1000.0
        );
        let started = Instant::now();
        let named = session.functions_named().expect("named");
        println!(
            "functions_named (JOIN)   {:6.1} ms  rows={}",
            started.elapsed().as_secs_f64() * 1000.0,
            named.len()
        );

        for (label, arguments) in [
            ("list_functions(5000)", json!({"limit": 5000})),
            ("function_summary(entry)", json!({"addr": "entry"})),
        ] {
            let started = Instant::now();
            let value = call(
                &mut session,
                label.split('(').next().unwrap(),
                &arguments,
                &budget,
            )
            .expect("tool");
            println!(
                "{label:24} {:6.1} ms  rows={} truncated={}",
                started.elapsed().as_secs_f64() * 1000.0,
                value["count"].as_u64().unwrap_or_default(),
                value["truncated"],
            );
        }

        // Cold vs cached L2 on the biggest function (the expensive path).
        let page = call(
            &mut session,
            "list_functions",
            &json!({"limit": 1}),
            &budget,
        )
        .expect("probe list");
        let biggest = session
            .functions()
            .expect("functions")
            .into_iter()
            .max_by_key(|function| function.end.get().saturating_sub(function.start.get()))
            .map(|function| format!("{:#x}", function.start.get()))
            .unwrap_or_else(|| page["functions"][0]["start"].as_str().unwrap().to_owned());
        for attempt in ["cold", "cached"] {
            let started = Instant::now();
            let value = call(
                &mut session,
                "get_pseudocode",
                &json!({"addr": biggest, "maxLines": 2000}),
                &budget,
            )
            .expect("pseudocode");
            println!(
                "get_pseudocode(biggest) {attempt:6} {:7.1} ms  cached={} lines={}",
                started.elapsed().as_secs_f64() * 1000.0,
                value["cached"],
                value["total_lines"],
            );
        }
    }

    #[test]
    fn a_bad_address_is_an_error_not_a_panic() {
        let Some(mut session) = project() else {
            return;
        };
        for name in [
            "get_disassembly",
            "get_pseudocode",
            "xrefs_to",
            "xrefs_from",
            "function_summary",
        ] {
            let result = call(
                &mut session,
                name,
                &json!({"addr": "no_such_symbol"}),
                &Budget::agent(),
            );
            assert!(result.is_err(), "{name} accepted a bogus address");
            assert!(
                result.unwrap_err().contains("no_such_symbol"),
                "{name} lost the offending name in its error"
            );
        }
    }
}
