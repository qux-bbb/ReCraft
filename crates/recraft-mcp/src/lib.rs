// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Model Context Protocol server exposing the session API as tools and resources.
//!
//! Transport: newline-delimited JSON-RPC 2.0 over stdio (MCP stdio transport), plus the
//! optional HTTP mode. The server is **read-only by default**: `--write` opts into the
//! journaled write tools (`rename`, `set_comment`, `set_prototype`, `apply_type`,
//! `define_function`) and `undo`, so a plain `serve-mcp` never mutates the project and
//! every enabled write stays revertible.
//!
//! Read tools: `list_functions`, `get_disassembly`, `get_pseudocode`, `xrefs_to`,
//! `xrefs_from`, `list_strings`, `list_imports`, `callgraph`, `search_bytes`,
//! `function_summary`.
//!
//! Write tools (`--write` only): `rename`, `set_comment`, `set_prototype`, `apply_type`,
//! `define_function`, `batch`, `undo`. Every one goes through the same command bus as the
//! GUI and is journaled with its origin, so `undo` can report exactly what it reverted.
//!
//! Resources: `recraft://project/{id}/functions|strings|imports` and
//! `recraft://project/{id}/pseudocode/{func}`.
//!
//! Every list-shaped tool reports `total` (matches before paging), `truncated` and
//! `next_offset`, and filters run before the offset and the row cap: a filtered query never
//! answers "no match" just because the match sits past the default window, and a capped
//! result is a page, not a dead end. `get_pseudocode` pages the same way with `startLine`.

#![forbid(unsafe_code)]

use std::io::{BufRead, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use recraft_common::{Address, Error, Result};
use recraft_core::{Command, CommentKind, Effect, Session};
use recraft_query::{Budget, MAX_LIMIT, hex};
use serde_json::{Value, json};

const PROTOCOL_VERSION: &str = "2024-11-05";
/// Protocol revisions this server speaks; `initialize` echoes the client's choice when it is
/// one of these, and falls back to [`PROTOCOL_VERSION`] otherwise.
const SUPPORTED_PROTOCOL_VERSIONS: [&str; 2] = ["2024-11-05", "2025-06-18"];
/// Server instructions. Clients copy this into the system prompt, so it states the
/// read-only contract and the truncation convention rather than describing the project.
const SERVER_INSTRUCTIONS_READ_ONLY: &str = "ReCraft serves one read-only reverse-engineering project. \
Start with list_functions (its filter is a case-insensitive name substring), then \
function_summary or get_pseudocode for one function; addresses accept 0x hex, decimal, a \
symbol name, or 'entry'. Large payloads report truncated:true together with the untruncated \
size; narrow the query or raise maxLines/maxBytes instead of assuming the result is complete.";
/// Instructions for `--write`: same query guidance, plus the mutation contract.
const SERVER_INSTRUCTIONS_WRITABLE: &str = "ReCraft serves one reverse-engineering project with \
read tools and journaled write tools. Query with list_functions (case-insensitive name \
substring), function_summary or get_pseudocode; addresses accept 0x hex, decimal, a symbol \
name, or 'entry'. Write tools (rename, set_comment, set_prototype, apply_type, \
define_function) mutate user data through the same command bus as the GUI: every call is \
applied immediately and recorded in the project journal with its origin, and undo reverts the \
newest change (or a whole batch) and reports what it reverted. Use batch to apply several \
edits atomically so one undo removes them all. Prefer reading first (pseudocode/strings/xrefs) \
and use a short, specific name; confirm with list_functions or get_pseudocode after a write. \
Large payloads report truncated:true with next_offset/next_start_line; follow those fields \
instead of assuming the result is complete.";

fn instructions(write: bool) -> &'static str {
    if write {
        SERVER_INSTRUCTIONS_WRITABLE
    } else {
        SERVER_INSTRUCTIONS_READ_ONLY
    }
}
type ToolResult = std::result::Result<Value, String>;

/// A project opened for MCP access.
pub struct McpServer {
    session: Session,
    project_id: String,
    /// `true` when `--write` enabled the journaled write tools.
    write: bool,
}

impl McpServer {
    /// Open read-only (the default): write tools are not registered at all.
    pub fn open(path: &Path) -> Result<Self> {
        Self::open_mode(path, false)
    }

    /// Open with the journaled write tools enabled (`serve-mcp --write`).
    pub fn open_writable(path: &Path) -> Result<Self> {
        Self::open_mode(path, true)
    }

    fn open_mode(path: &Path, write: bool) -> Result<Self> {
        let mut session = Session::open(path)?;
        // Provenance for the shared journal: a GUI user and an MCP client must stay
        // distinguishable in the audit trail and in `undo` results.
        session.set_origin("mcp");
        let project_id = session
            .path()
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_else(|| "project".to_owned());
        Ok(McpServer {
            session,
            project_id,
            write,
        })
    }

    /// Whether this server exposes the write tools.
    pub const fn is_writable(&self) -> bool {
        self.write
    }

    /// Serve newline-delimited JSON-RPC on stdin/stdout until EOF.
    pub fn serve_stdio(mut self) -> Result<()> {
        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout().lock();
        for line in stdin.lock().lines() {
            let line = line.map_err(Error::from)?;
            if line.trim().is_empty() {
                continue;
            }
            let Some(response) = self.handle_line(&line) else {
                continue;
            };
            writeln!(stdout, "{response}").map_err(Error::from)?;
            stdout.flush().map_err(Error::from)?;
        }
        Ok(())
    }

    /// Handle one JSON-RPC line; returns the serialized response (None for notifications).
    pub fn handle_line(&mut self, line: &str) -> Option<String> {
        let message: Value = match serde_json::from_str(line) {
            Ok(message) => message,
            Err(error) => {
                return Some(
                    json!({
                        "jsonrpc": "2.0",
                        "id": Value::Null,
                        "error": {"code": -32700, "message": format!("parse error: {error}")},
                    })
                    .to_string(),
                );
            }
        };
        self.handle_message(&message).map(|value| value.to_string())
    }

    pub fn handle_message(&mut self, message: &Value) -> Option<Value> {
        let id = message.get("id").cloned();
        let Some(method) = message.get("method").and_then(Value::as_str) else {
            return Some(rpc_error(
                id.unwrap_or(Value::Null),
                -32600,
                "invalid request: missing method",
            ));
        };
        let params = message.get("params").cloned().unwrap_or(Value::Null);

        let outcome: std::result::Result<Value, (i64, String)> = match method {
            "initialize" => {
                // Echo the revision the client asked for when we speak it, so a 2025-06-18
                // client does not have to downgrade to the oldest revision we know.
                let version = params
                    .get("protocolVersion")
                    .and_then(Value::as_str)
                    .filter(|version| SUPPORTED_PROTOCOL_VERSIONS.contains(version))
                    .unwrap_or(PROTOCOL_VERSION);
                Ok(json!({
                    "protocolVersion": version,
                    "capabilities": {
                        "tools": {"listChanged": false},
                        "resources": {"subscribe": false, "listChanged": false},
                    },
                    "serverInfo": {"name": "recraft", "version": env!("CARGO_PKG_VERSION")},
                    "instructions": instructions(self.write),
                }))
            }
            "notifications/initialized" | "notifications/cancelled" => return None,
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({"tools": tools(self.write)})),
            "tools/call" => self.call_tool(&params),
            "resources/list" => Ok(json!({"resources": self.resources()})),
            "resources/templates/list" => {
                Ok(json!({"resourceTemplates": self.resource_templates()}))
            }
            "resources/read" => self.read_resource(&params),
            other => Err((-32601, format!("method not found: {other}"))),
        };

        match (id, outcome) {
            (Some(id), Ok(result)) => Some(json!({"jsonrpc": "2.0", "id": id, "result": result})),
            (Some(id), Err((code, message))) => Some(rpc_error(id, code, message)),
            // Notifications never get a response, even on failure.
            (None, _) => None,
        }
    }

    // ------------------------------------------------------------- tools ----

    fn call_tool(&mut self, params: &Value) -> std::result::Result<Value, (i64, String)> {
        let Some(name) = params.get("name").and_then(Value::as_str) else {
            return Err((-32602, "tools/call requires a tool name".to_owned()));
        };
        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));
        if is_write_tool(name) && !self.write {
            return Ok(json!({
                "content": [{
                    "type": "text",
                    "text": "error: write tools are disabled for this server; start it with `serve-mcp --write` to enable them.",
                }],
                "isError": true,
            }));
        }
        let outcome: ToolResult = if recraft_query::is_read_tool(name) {
            recraft_query::call(&mut self.session, name, &arguments, &Budget::mcp())
        } else {
            match name {
                "rename" => self.tool_rename(&arguments),
                "set_comment" => self.tool_set_comment(&arguments),
                "set_prototype" => self.tool_set_prototype(&arguments),
                "apply_type" => self.tool_apply_type(&arguments),
                "define_function" => self.tool_define_function(&arguments),
                "batch" => self.tool_batch(&arguments),
                "undo" => self.tool_undo(&arguments),
                other => return Err((-32602, format!("unknown tool: {other}"))),
            }
        };
        match outcome {
            Ok(value) => Ok(json!({
                "content": [{"type": "text", "text": value.to_string()}],
                // 2025-06-18 clients read the structured form; older ones still get the JSON
                // text block. Every tool declares an object `outputSchema`.
                "structuredContent": value,
                "isError": false,
            })),
            Err(message) => Ok(json!({
                "content": [{"type": "text", "text": format!("error: {message}")}],
                "isError": true,
            })),
        }
    }

    // ------------------------------------------------------- write tools ----
    //
    // Registered only with `--write`. Each tool resolves its arguments, builds one
    // `Command`, and applies it through `Session::execute`, which journals the inverse
    // in the same transaction as the change. The result reports the previous value and
    // the dirty ranges, so the caller can verify the change or call `undo`.

    fn tool_rename(&mut self, arguments: &Value) -> ToolResult {
        let edit = Edit::parse(self, "rename", arguments)?;
        self.apply_edit(edit)
    }

    fn tool_set_comment(&mut self, arguments: &Value) -> ToolResult {
        let edit = Edit::parse(self, "set_comment", arguments)?;
        self.apply_edit(edit)
    }

    fn tool_set_prototype(&mut self, arguments: &Value) -> ToolResult {
        let edit = Edit::parse(self, "set_prototype", arguments)?;
        self.apply_edit(edit)
    }

    fn tool_apply_type(&mut self, arguments: &Value) -> ToolResult {
        let edit = Edit::parse(self, "apply_type", arguments)?;
        self.apply_edit(edit)
    }

    fn tool_define_function(&mut self, arguments: &Value) -> ToolResult {
        let edit = Edit::parse(self, "define_function", arguments)?;
        self.apply_edit(edit)
    }

    /// Apply one parsed edit and report the previous value, the new value and dirty ranges.
    fn apply_edit(&mut self, edit: Edit) -> ToolResult {
        let previous = edit.previous(&self.session)?;
        let effect = self
            .session
            .execute(edit.command.clone())
            .map_err(|e| e.to_string())?;
        let seq = self.session.last_journal_seq().map_err(|e| e.to_string())?;
        if let Command::DefineFunc { start, end } = &edit.command {
            return Ok(json!({
                "tool": edit.tool,
                "seq": seq,
                "addr": hex(*start),
                "previous_end": previous,
                "end": hex(*end),
                "dirty": dirty_ranges(&effect),
                "undo": true,
                "note": "applied to the project journal; call undo to revert",
            }));
        }
        Ok(change_payload(
            edit.tool,
            seq,
            edit.addr,
            previous,
            edit.value(),
            &effect,
        ))
    }

    /// Apply several edits as one journal batch (all or nothing, reverted by one `undo`).
    fn tool_batch(&mut self, arguments: &Value) -> ToolResult {
        let Some(entries) = arguments.get("commands").and_then(Value::as_array) else {
            return Err("batch needs a `commands` array".to_owned());
        };
        if entries.is_empty() {
            return Err("batch has no commands".to_owned());
        }
        let mut edits = Vec::with_capacity(entries.len());
        for (index, entry) in entries.iter().enumerate() {
            let tool = entry
                .get("tool")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("command #{}: needs a string `tool`", index + 1))?;
            let arguments = entry.get("arguments").cloned().unwrap_or_else(|| json!({}));
            let edit = Edit::parse(self, tool, &arguments)
                .map_err(|error| format!("command #{} ({tool}): {error}", index + 1))?;
            edits.push(edit);
        }
        let commands: Vec<Command> = edits.iter().map(|edit| edit.command.clone()).collect();
        let effects = self
            .session
            .execute_batch(&commands)
            .map_err(|e| e.to_string())?;
        let mut dirty = Vec::new();
        for effect in &effects {
            dirty.extend(dirty_ranges(effect));
        }
        Ok(json!({
            "tool": "batch",
            "seq": self.session.last_journal_seq().map_err(|e| e.to_string())?,
            "count": edits.len(),
            "commands": edits.iter().map(|edit| edit.tool).collect::<Vec<_>>(),
            "dirty": dirty,
            "undo": true,
            "note": "applied as one journal batch; a single undo reverts all of it",
        }))
    }

    fn tool_undo(&mut self, arguments: &Value) -> ToolResult {
        let seq = arguments.get("seq").and_then(Value::as_i64);
        let dry_run = arguments
            .get("dryRun")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if dry_run {
            let Some(records) = self.session.undo_preview().map_err(|e| e.to_string())? else {
                return Ok(json!({"undone": false, "dry_run": true, "message": "nothing to undo"}));
            };
            return Ok(json!({
                "dry_run": true,
                "would_undo": undo_records(&records),
            }));
        }
        let report = match seq {
            Some(seq) => Some(self.session.undo_seq(seq).map_err(|e| e.to_string())?),
            None => self.session.undo_report().map_err(|e| e.to_string())?,
        };
        Ok(match report {
            Some(report) => json!({
                "undone": true,
                "records": undo_records(&report.records),
                "dirty": dirty_ranges(&report.effect),
            }),
            None => json!({
                "undone": false,
                "message": "nothing to undo",
            }),
        })
    }

    // --------------------------------------------------------- resources ----

    fn resources(&self) -> Vec<Value> {
        let base = format!("recraft://project/{}", self.project_id);
        vec![
            json!({
                "uri": format!("{base}/functions"),
                "name": "functions",
                "mimeType": "application/json",
            }),
            json!({
                "uri": format!("{base}/strings"),
                "name": "strings",
                "mimeType": "application/json",
            }),
            json!({
                "uri": format!("{base}/imports"),
                "name": "imports",
                "mimeType": "application/json",
            }),
        ]
    }

    fn resource_templates(&self) -> Vec<Value> {
        let base = format!("recraft://project/{}", self.project_id);
        vec![json!({
            "uriTemplate": format!("{base}/pseudocode/{{func}}"),
            "name": "pseudocode",
            "mimeType": "text/plain",
        })]
    }

    fn read_resource(&mut self, params: &Value) -> std::result::Result<Value, (i64, String)> {
        let Some(uri) = params.get("uri").and_then(Value::as_str) else {
            return Err((-32602, "resources/read requires a uri".to_owned()));
        };
        let prefix = format!("recraft://project/{}/", self.project_id);
        let Some(rest) = uri.strip_prefix(&prefix) else {
            return Err((-32602, format!("unknown resource uri: {uri}")));
        };
        let read = |session: &mut Session, name: &str, arguments: Value| {
            recraft_query::call(session, name, &arguments, &Budget::mcp())
        };
        let (mime, result) = if rest == "functions" {
            (
                "application/json",
                read(
                    &mut self.session,
                    "list_functions",
                    json!({"limit": MAX_LIMIT}),
                ),
            )
        } else if rest == "strings" {
            (
                "application/json",
                read(
                    &mut self.session,
                    "list_strings",
                    json!({"limit": MAX_LIMIT}),
                ),
            )
        } else if rest == "imports" {
            (
                "application/json",
                read(&mut self.session, "list_imports", json!({})),
            )
        } else if let Some(target) = rest.strip_prefix("pseudocode/") {
            let value = json!({"addr": target, "level": "l2"});
            (
                "text/plain",
                read(&mut self.session, "get_pseudocode", value),
            )
        } else {
            return Err((-32602, format!("unknown resource uri: {uri}")));
        };
        match result {
            Ok(value) => {
                let text = if mime == "text/plain" {
                    let mut text = value
                        .get("text")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                        .unwrap_or_else(|| value.to_string());
                    // A resource has no `truncated` field to read, so keep the cut visible as a
                    // trailing pseudo-C comment instead of serving a silently short document.
                    if value.get("truncated").and_then(Value::as_bool) == Some(true) {
                        let total = value
                            .get("total_lines")
                            .and_then(Value::as_u64)
                            .unwrap_or(0);
                        let end = value.get("end_line").and_then(Value::as_u64).unwrap_or(0);
                        text.push_str(&format!(
                            "// truncated: lines 1-{end} of {total}; read the rest with get_pseudocode(startLine={})\n",
                            end + 1
                        ));
                    }
                    text
                } else {
                    value.to_string()
                };
                Ok(json!({
                    "contents": [{"uri": uri, "mimeType": mime, "text": text}],
                }))
            }
            Err(message) => Err((-32603, format!("resource error: {message}"))),
        }
    }

    // ----------------------------------------------------------- helpers ----

    fn resolve_address(&self, value: Option<&Value>) -> Result<Address> {
        // Address syntax is part of the shared query contract, not of the transport.
        recraft_query::resolve_address(&self.session, value)
    }
}

/// One write command parsed from tool arguments, before it is applied.
///
/// Both the single write tools and `batch` build through this type, so an argument shape
/// cannot drift between "rename one address" and "rename twenty in one call".
struct Edit {
    tool: &'static str,
    addr: Address,
    command: Command,
}

impl Edit {
    fn parse(
        server: &McpServer,
        tool: &str,
        arguments: &Value,
    ) -> std::result::Result<Edit, String> {
        let resolve =
            |value: Option<&Value>| server.resolve_address(value).map_err(|e| e.to_string());
        Ok(match tool {
            "rename" => {
                let addr = resolve(arguments.get("addr"))?;
                let name = optional_text(arguments, "name")?;
                Edit {
                    tool: "rename",
                    addr,
                    command: Command::Rename { addr, name },
                }
            }
            "set_comment" => {
                let addr = resolve(arguments.get("addr"))?;
                let kind = comment_kind(arguments)?;
                let text = optional_text(arguments, "text")?;
                Edit {
                    tool: "set_comment",
                    addr,
                    command: Command::SetComment { addr, kind, text },
                }
            }
            "set_prototype" => {
                let addr = resolve(arguments.get("addr"))?;
                let text = optional_text(arguments, "text")?;
                Edit {
                    tool: "set_prototype",
                    addr,
                    command: Command::SetPrototype { addr, text },
                }
            }
            "apply_type" => {
                let func = resolve(arguments.get("func"))?;
                let slot = arguments
                    .get("slot")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| "apply_type requires `slot` (0 = first argument)".to_owned())?;
                let slot = u8::try_from(slot)
                    .map_err(|_| format!("slot {slot} is out of range (0-255)"))?;
                let struct_name = optional_text(arguments, "struct")?;
                Edit {
                    tool: "apply_type",
                    addr: func,
                    command: Command::ApplyType {
                        func,
                        slot,
                        struct_name,
                    },
                }
            }
            "define_function" => {
                let start = resolve(arguments.get("start"))?;
                let end = resolve(arguments.get("end"))?;
                if end <= start {
                    return Err(format!(
                        "define_function needs end > start ({} >= {})",
                        hex(end),
                        hex(start)
                    ));
                }
                Edit {
                    tool: "define_function",
                    addr: start,
                    command: Command::DefineFunc { start, end },
                }
            }
            other => return Err(format!("unknown write tool: {other}")),
        })
    }

    /// Value of the field this edit overwrites, formatted for the result payload.
    fn previous(&self, session: &Session) -> std::result::Result<Option<String>, String> {
        let value = match &self.command {
            Command::Rename { addr, .. } => session.name_at(*addr),
            Command::SetComment { addr, kind, .. } => session.comment_at(*addr, *kind),
            Command::SetPrototype { addr, .. } => session.prototype_at(*addr),
            Command::ApplyType { func, slot, .. } => session.arg_type(*func, *slot),
            Command::DefineFunc { start, .. } => session.func_at(*start).map(|end| end.map(hex)),
            _ => return Err("unsupported edit".to_owned()),
        };
        value.map_err(|error| error.to_string())
    }

    /// Value the edit writes, formatted for the result payload.
    fn value(&self) -> Option<String> {
        match &self.command {
            Command::Rename { name, .. } => name.clone(),
            Command::SetComment { text, .. } => text.clone(),
            Command::SetPrototype { text, .. } => text.clone(),
            Command::ApplyType { struct_name, .. } => struct_name.clone(),
            _ => None,
        }
    }
}

/// Journal records reverted by one undo, as they appear in a tool result.
fn undo_records(records: &[recraft_core::UndoRecord]) -> Vec<Value> {
    records
        .iter()
        .map(|record| {
            json!({
                "seq": record.seq,
                "cmd": record.cmd,
                "origin": record.origin,
                "ts": record.ts,
                "batch": record.batch,
            })
        })
        .collect()
}

fn rpc_error(id: Value, code: i64, message: impl Into<String>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {"code": code, "message": message.into()},
    })
}

fn bearer_matches(authorization: Option<&str>, token: &str) -> bool {
    let expected = format!("Bearer {token}");
    bytes_equal_ct(
        authorization.unwrap_or_default().as_bytes(),
        expected.as_bytes(),
    )
}

fn bytes_equal_ct(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut difference = 0u8;
    for (a, b) in left.iter().zip(right) {
        difference |= a ^ b;
    }
    difference == 0
}

/// Tools that mutate the project; only registered (and callable) with `--write`.
fn is_write_tool(name: &str) -> bool {
    matches!(
        name,
        "rename"
            | "set_comment"
            | "set_prototype"
            | "apply_type"
            | "define_function"
            | "batch"
            | "undo"
    )
}

/// Read an optional string argument: missing is an error, `null` means "clear".
fn optional_text(arguments: &Value, key: &str) -> std::result::Result<Option<String>, String> {
    match arguments.get(key) {
        None => Err(format!("missing `{key}` (a string, or null to clear)")),
        Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.clone())),
        Some(_) => Err(format!("`{key}` must be a string or null")),
    }
}

fn comment_kind(arguments: &Value) -> std::result::Result<CommentKind, String> {
    match arguments
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("regular")
    {
        "regular" => Ok(CommentKind::Regular),
        "repeatable" => Ok(CommentKind::Repeatable),
        "anterior" => Ok(CommentKind::Anterior),
        other => Err(format!(
            "unknown comment kind {other:?}; use regular, repeatable or anterior"
        )),
    }
}

/// Format the ranges a command invalidated, e.g. `["0x140001008..+1"]`.
fn dirty_ranges(effect: &Effect) -> Vec<String> {
    effect
        .dirty
        .iter()
        .map(|range| format!("{}..+{}", hex(range.start), range.len()))
        .collect()
}

/// Shared shape of a single write result: previous value, new value, dirty ranges.
fn change_payload(
    tool: &str,
    seq: i64,
    addr: Address,
    previous: Option<String>,
    value: Option<String>,
    effect: &Effect,
) -> Value {
    json!({
        "tool": tool,
        "seq": seq,
        "addr": hex(addr),
        "previous": previous,
        "value": value,
        "dirty": dirty_ranges(effect),
        "undo": true,
        "note": "applied to the project journal; call undo to revert",
    })
}

// ---------------------------------------------------------------- tools ----

fn tools(write: bool) -> Vec<Value> {
    // The read surface lives in `recraft-query`, shared with the in-app agent: one schema,
    // one executor, no per-front-end wording or caps to drift.
    let mut tools = recraft_query::specs(&Budget::mcp());
    if write {
        tools.extend(write_tools());
    }
    tools
}

fn write_tools() -> Vec<Value> {
    let mut tools = vec![
        json!({
            "name": "rename",
            "description": "Rename the address (function, data, import) through the journaled command bus. `name: null` clears the user name. Fails on duplicate or reserved names. Undoable.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "addr": {"type": ["integer", "string"], "description": "Address, hex string, name, or 'entry'."},
                    "name": {"type": ["string", "null"], "description": "New name; null clears it."},
                },
                "required": ["addr", "name"],
            },
        }),
        json!({
            "name": "set_comment",
            "description": "Set or clear a comment at an address. `text: null` deletes it. Undoable.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "addr": {"type": ["integer", "string"]},
                    "text": {"type": ["string", "null"]},
                    "kind": {"type": "string", "enum": ["regular", "repeatable", "anterior"], "description": "Comment slot; default regular."},
                },
                "required": ["addr", "text"],
            },
        }),
        json!({
            "name": "set_prototype",
            "description": "Set or clear the C prototype of a function. `text: null` deletes it. Undoable.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "addr": {"type": ["integer", "string"]},
                    "text": {"type": ["string", "null"], "description": "C declaration, e.g. 'int __cdecl sub_140001008(int a, char *b)'."},
                },
                "required": ["addr", "text"],
            },
        }),
        json!({
            "name": "apply_type",
            "description": "Bind a user-defined struct layout to a function argument slot (0 = first argument). `struct: null` clears the binding; the struct must already exist (define it in the GUI first). Undoable.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "func": {"type": ["integer", "string"]},
                    "slot": {"type": "integer", "minimum": 0, "maximum": 255},
                    "struct": {"type": ["string", "null"]},
                },
                "required": ["func", "slot", "struct"],
            },
        }),
        json!({
            "name": "define_function",
            "description": "Define (or resize) a function at `start` with the given exclusive `end`. Undoable.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "start": {"type": ["integer", "string"]},
                    "end": {"type": ["integer", "string"]},
                },
                "required": ["start", "end"],
            },
        }),
        json!({
            "name": "batch",
            "description": "Apply several write tools as one journal batch: either all of them land or none does, and a single `undo` reverts the whole set. Each entry is {\"tool\": \"rename\", \"arguments\": {...}} with the same arguments the single tool takes.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "commands": {
                        "type": "array",
                        "minItems": 1,
                        "items": {
                            "type": "object",
                            "properties": {
                                "tool": {"type": "string", "enum": ["rename", "set_comment", "set_prototype", "apply_type", "define_function"]},
                                "arguments": {"type": "object"},
                            },
                            "required": ["tool"],
                        },
                    },
                },
                "required": ["commands"],
            },
        }),
        json!({
            "name": "undo",
            "description": "Revert the newest journaled command (or batch) and report which records were reverted. `dryRun` shows what would be undone; `seq` requires that record to be the newest applied one.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "seq": {"type": "integer", "description": "Only undo if this is the newest applied record."},
                    "dryRun": {"type": "boolean", "description": "Report what would be undone without changing anything."},
                },
            },
        }),
    ];
    for tool in &mut tools {
        let name = tool["name"].as_str().unwrap_or_default().to_owned();
        tool["title"] = json!(write_title_of(&name));
        // Writes overwrite user data, so a client should confirm them; `batch` and `undo`
        // are not idempotent (a second undo reverts the previous command).
        tool["annotations"] = json!({
            "readOnlyHint": false,
            "destructiveHint": true,
            "idempotentHint": !matches!(name.as_str(), "batch" | "undo"),
            "openWorldHint": false,
        });
        tool["outputSchema"] = json!({"type": "object", "additionalProperties": true});
    }
    tools
}

/// Human-readable label for a write tool.
fn write_title_of(name: &str) -> &'static str {
    match name {
        "rename" => "Rename",
        "set_comment" => "Set comment",
        "set_prototype" => "Set prototype",
        "apply_type" => "Apply struct type",
        "define_function" => "Define function",
        "batch" => "Batch edits",
        "undo" => "Undo",
        _ => "ReCraft write tool",
    }
}

// ------------------------------------------------------------------ HTTP -----

/// Serve JSON-RPC over HTTP (`POST /mcp`) with bearer-token authentication (read-only).
pub fn serve_http(project: &Path, bind: &str, token: &str) -> Result<()> {
    serve_http_mode(project, bind, token, false)
}

/// Same as [`serve_http`], with `--write` enabling the journaled write tools.
pub fn serve_http_writable(project: &Path, bind: &str, token: &str) -> Result<()> {
    serve_http_mode(project, bind, token, true)
}

fn serve_http_mode(project: &Path, bind: &str, token: &str, write: bool) -> Result<()> {
    let listener = TcpListener::bind(bind).map_err(Error::from)?;
    let local = listener.local_addr().map_err(Error::from)?;
    let mode = if write { "write" } else { "read-only" };
    eprintln!("[mcp] http listening on http://{local}/mcp ({mode}, bearer token required)");
    serve_http_listener_mode(listener, project.to_path_buf(), token.to_owned(), write)
}

/// Maximum number of requests handled at the same time; each one owns a `Session`.
const MAX_HTTP_CONNECTIONS: usize = 32;
static HTTP_CONNECTIONS: AtomicUsize = AtomicUsize::new(0);

/// RAII slot for the concurrency cap.
struct ConnectionSlot;

impl ConnectionSlot {
    fn acquire() -> Option<ConnectionSlot> {
        let current = HTTP_CONNECTIONS.fetch_add(1, Ordering::SeqCst);
        if current >= MAX_HTTP_CONNECTIONS {
            HTTP_CONNECTIONS.fetch_sub(1, Ordering::SeqCst);
            return None;
        }
        Some(ConnectionSlot)
    }
}

impl Drop for ConnectionSlot {
    fn drop(&mut self) {
        HTTP_CONNECTIONS.fetch_sub(1, Ordering::SeqCst);
    }
}

pub(crate) fn serve_http_listener_mode(
    listener: TcpListener,
    project: PathBuf,
    token: String,
    write: bool,
) -> Result<()> {
    // Fail fast when the project cannot be opened at all instead of answering 500 forever.
    drop(McpServer::open_mode(&project, write)?);
    for stream in listener.incoming() {
        let Ok(stream) = stream else {
            continue;
        };
        let project = project.clone();
        let token = token.clone();
        // One thread per connection, one `Session` per request: a long decompile on one
        // connection must not block `ping` or the next client. The previous single-threaded
        // loop serialised every request behind the one before it.
        std::thread::spawn(move || {
            let mut stream = stream;
            let _ = handle_http_connection(&mut stream, &project, &token, write);
        });
    }
    Ok(())
}

struct HttpRequest {
    method: String,
    path: String,
    authorization: Option<String>,
    origin: Option<String>,
    protocol_version: Option<String>,
    body: String,
}

/// Serve one HTTP connection. The transport is stateless and every client sends
/// `Connection: close`, so the `Session` is opened per request; that is what lets two tool
/// calls run at the same time.
fn handle_http_connection(
    stream: &mut TcpStream,
    project: &Path,
    token: &str,
    write: bool,
) -> Result<()> {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    let Some(request) = read_http_request(stream)? else {
        return Ok(());
    };
    let (status, body) = if !origin_allowed(request.origin.as_deref()) {
        // Only a browser can be tricked into DNS rebinding, so a non-loopback Origin is
        // refused even when the token is right.
        eprintln!(
            "[mcp] rejected request with Origin {:?}",
            request.origin.as_deref().unwrap_or_default()
        );
        (403, json!({"error": "origin not allowed"}).to_string())
    } else if request.method == "GET" && request.path == "/health" {
        (200, r#"{"status":"ok"}"#.to_owned())
    } else if request.method != "POST" {
        (405, r#"{"error":"method not allowed"}"#.to_owned())
    } else if request.path != "/mcp" {
        (404, r#"{"error":"not found"}"#.to_owned())
    } else if !bearer_matches(request.authorization.as_deref(), token) {
        (
            401,
            r#"{"error":"missing or invalid bearer token"}"#.to_owned(),
        )
    } else if let Some(version) = request
        .protocol_version
        .as_deref()
        .filter(|version| !SUPPORTED_PROTOCOL_VERSIONS.contains(version))
    {
        (
            400,
            json!({"error": format!("unsupported MCP-Protocol-Version: {version}")}).to_string(),
        )
    } else {
        match ConnectionSlot::acquire() {
            None => (
                503,
                r#"{"error":"too many concurrent requests"}"#.to_owned(),
            ),
            Some(slot) => {
                let response = handle_rpc(project, write, &request.body);
                drop(slot);
                response
            }
        }
    };
    write_http_response(stream, status, &body)
}

fn handle_rpc(project: &Path, write: bool, body: &str) -> (u16, String) {
    let mut server = match McpServer::open_mode(project, write) {
        Ok(server) => server,
        Err(error) => {
            return (
                500,
                json!({"error": format!("cannot open project: {error}")}).to_string(),
            );
        }
    };
    match serde_json::from_str::<Value>(body) {
        Err(error) => (
            400,
            json!({"error": format!("parse error: {error}")}).to_string(),
        ),
        Ok(message) => match server.handle_message(&message) {
            Some(response) => (200, response.to_string()),
            None => (202, String::new()),
        },
    }
}

/// Browsers send `Origin`; MCP clients on the same machine usually do not. Accept a missing
/// Origin and loopback origins only (DNS rebinding defence, MCP streamable-HTTP §Security).
fn origin_allowed(origin: Option<&str>) -> bool {
    let Some(origin) = origin else {
        return true;
    };
    let Some(rest) = origin
        .strip_prefix("http://")
        .or_else(|| origin.strip_prefix("https://"))
    else {
        return false;
    };
    let authority = rest.split('/').next().unwrap_or(rest);
    let host = if let Some(inner) = authority.strip_prefix('[') {
        // `[::1]:8080` -> `[::1]`
        match inner.split(']').next() {
            Some(host) => format!("[{host}]"),
            None => return false,
        }
    } else {
        authority.split(':').next().unwrap_or(authority).to_owned()
    };
    matches!(
        host.to_ascii_lowercase().as_str(),
        "127.0.0.1" | "localhost" | "[::1]"
    )
}

fn read_http_request(stream: &mut TcpStream) -> Result<Option<HttpRequest>> {
    let mut reader = std::io::BufReader::new(stream.try_clone().map_err(Error::from)?);
    let mut head = String::new();
    loop {
        let mut line = String::new();
        let read = reader.read_line(&mut line).map_err(Error::from)?;
        if read == 0 {
            return Ok(None);
        }
        head.push_str(&line);
        if head.ends_with("\r\n\r\n") {
            break;
        }
        if head.len() > 64 * 1024 {
            return Err(Error::internal("HTTP request headers too large"));
        }
    }
    let mut lines = head.split("\r\n");
    let request_line = lines.next().unwrap_or_default();
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_owned();
    let path = parts.next().unwrap_or_default().to_owned();
    let mut authorization = None;
    let mut origin = None;
    let mut protocol_version = None;
    let mut content_length = 0usize;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim().to_ascii_lowercase();
        let value = value.trim();
        if name == "authorization" {
            authorization = Some(value.to_owned());
        } else if name == "content-length" {
            content_length = value.parse().unwrap_or(0);
        } else if name == "origin" {
            origin = Some(value.to_owned());
        } else if name == "mcp-protocol-version" {
            protocol_version = Some(value.to_owned());
        }
    }
    if content_length > 4 * 1024 * 1024 {
        return Err(Error::internal("HTTP request body too large"));
    }
    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body).map_err(Error::from)?;
    Ok(Some(HttpRequest {
        method,
        path,
        authorization,
        origin,
        protocol_version,
        body: String::from_utf8_lossy(&body).into_owned(),
    }))
}

fn write_http_response(stream: &mut TcpStream, status: u16, body: &str) -> Result<()> {
    let reason = match status {
        200 => "OK",
        202 => "Accepted",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        500 => "Internal Server Error",
        503 => "Service Unavailable",
        _ => "OK",
    };
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).map_err(Error::from)?;
    stream.flush().map_err(Error::from)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn notepad_project() -> Option<McpServer> {
        let bytes = std::fs::read("C:/Windows/System32/notepad.exe").ok()?;
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("mcp.rcraft");
        {
            let mut session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
            let result = recraft_analysis::analyze(&mut session).unwrap();
            recraft_analysis::persist(&mut session, &result).unwrap();
            let strings = recraft_analysis::extract_strings(&mut session, 4, 200_000).unwrap();
            recraft_analysis::persist_strings(&mut session, &strings).unwrap();
        }
        let server = McpServer::open(&project).unwrap();
        // The project file must outlive the server; the process exits after the test.
        std::mem::forget(dir);
        Some(server)
    }

    fn notepad_writable() -> Option<(McpServer, PathBuf)> {
        let bytes = std::fs::read("C:/Windows/System32/notepad.exe").ok()?;
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("write.rcraft");
        {
            let mut session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
            let result = recraft_analysis::analyze(&mut session).unwrap();
            recraft_analysis::persist(&mut session, &result).unwrap();
        }
        let server = McpServer::open_writable(&project).unwrap();
        // The project file must outlive the server; the process exits after the test.
        std::mem::forget(dir);
        Some((server, project))
    }

    fn notepad_project_path() -> Option<PathBuf> {
        let bytes = std::fs::read("C:/Windows/System32/notepad.exe").ok()?;
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("http.rcraft");
        {
            let mut session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
            let result = recraft_analysis::analyze(&mut session).unwrap();
            recraft_analysis::persist(&mut session, &result).unwrap();
        }
        // The project file must outlive the test; the process exits afterwards.
        std::mem::forget(dir);
        Some(project)
    }

    fn http_request(
        addr: std::net::SocketAddr,
        path: &str,
        token: Option<&str>,
        body: &str,
    ) -> (u16, String) {
        http_request_with(addr, path, token, &[], body)
    }

    fn http_request_with(
        addr: std::net::SocketAddr,
        path: &str,
        token: Option<&str>,
        extra_headers: &[(&str, &str)],
        body: &str,
    ) -> (u16, String) {
        use std::io::{Read, Write};
        let mut stream = std::net::TcpStream::connect(addr).unwrap();
        let auth = token
            .map(|token| format!("Authorization: Bearer {token}\r\n"))
            .unwrap_or_default();
        let extra: String = extra_headers
            .iter()
            .map(|(name, value)| format!("{name}: {value}\r\n"))
            .collect();
        let request = format!(
            "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{auth}{extra}Connection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(request.as_bytes()).unwrap();
        let mut raw = String::new();
        stream.read_to_string(&mut raw).unwrap();
        let (head, response_body) = raw.split_once("\r\n\r\n").unwrap_or((&raw, ""));
        let status = head
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|code| code.parse().ok())
            .unwrap_or(0);
        (status, response_body.to_owned())
    }

    #[test]
    fn http_transport_checks_token_and_serves_jsonrpc() {
        let Some(project) = notepad_project_path() else {
            return;
        };
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let token = "secret-token";
        std::thread::spawn(move || {
            let _ = serve_http_listener_mode(listener, project, token.to_owned(), false);
        });

        let initialize = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
        assert_eq!(
            http_request(addr, "/mcp", None, initialize).0,
            401,
            "missing token must be rejected"
        );
        assert_eq!(
            http_request(addr, "/mcp", Some("wrong"), initialize).0,
            401,
            "wrong token must be rejected"
        );
        let (status, body) = http_request(addr, "/mcp", Some(token), initialize);
        assert_eq!(status, 200);
        assert!(body.contains("serverInfo"), "{body}");

        let call = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"list_functions","arguments":{"limit":2}}}"#;
        let (status, body) = http_request(addr, "/mcp", Some(token), call);
        assert_eq!(status, 200);
        assert!(body.contains("functions"), "{body}");

        // Health probe does not need the token, but unknown paths do not exist.
        use std::io::{Read, Write};
        let mut stream = std::net::TcpStream::connect(addr).unwrap();
        stream
            .write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .unwrap();
        let mut raw = String::new();
        stream.read_to_string(&mut raw).unwrap();
        assert!(raw.starts_with("HTTP/1.1 200"), "{raw}");
        assert_eq!(http_request(addr, "/nope", Some(token), initialize).0, 404);

        // An extra `Origin` is only honoured for loopback: a browser page on another site
        // must not reach the local server even with a valid token.
        assert_eq!(
            http_request_with(
                addr,
                "/mcp",
                Some(token),
                &[("Origin", "http://evil.example")],
                initialize
            )
            .0,
            403,
            "foreign Origin must be rejected"
        );
        assert_eq!(
            http_request_with(
                addr,
                "/mcp",
                Some(token),
                &[("Origin", "http://127.0.0.1:3000")],
                initialize
            )
            .0,
            200,
            "loopback Origin is the local UI case"
        );

        // The negotiated version is validated on every request that carries the header.
        assert_eq!(
            http_request_with(
                addr,
                "/mcp",
                Some(token),
                &[("MCP-Protocol-Version", "2099-01-01")],
                initialize
            )
            .0,
            400
        );
        assert_eq!(
            http_request_with(
                addr,
                "/mcp",
                Some(token),
                &[("MCP-Protocol-Version", "2025-06-18")],
                initialize
            )
            .0,
            200
        );
    }

    #[test]
    fn an_idle_connection_does_not_block_the_next_request() {
        let Some(project) = notepad_project_path() else {
            return;
        };
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            let _ = serve_http_listener_mode(listener, project, "idle".to_owned(), false);
        });

        // A connection that never sends anything used to stall the single-threaded loop for
        // the whole read timeout.
        let idle = std::net::TcpStream::connect(addr).unwrap();
        let (status, body) = http_request(
            addr,
            "/mcp",
            Some("idle"),
            r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#,
        );
        assert_eq!(status, 200, "{body}");
        assert!(
            body.contains("\"result\"") || body.contains("\"id\""),
            "{body}"
        );
        drop(idle);
    }

    fn call(server: &mut McpServer, method: &str, params: Value) -> Value {
        server
            .handle_message(&json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}))
            .expect("response expected")
    }

    /// Call a tool and parse the JSON payload out of `content[0].text`.
    fn tool_json(server: &mut McpServer, tool: &str, arguments: Value) -> Value {
        let response = call(
            server,
            "tools/call",
            json!({"name": tool, "arguments": arguments}),
        );
        let text = response["result"]["content"][0]["text"]
            .as_str()
            .expect("tool result text");
        serde_json::from_str(text).expect("tool result is JSON")
    }

    #[test]
    fn initialize_reports_server_info() {
        let Some(mut server) = notepad_project() else {
            return;
        };
        let response = call(&mut server, "initialize", json!({}));
        assert_eq!(response["result"]["serverInfo"]["name"], "recraft");
        assert_eq!(response["result"]["protocolVersion"], PROTOCOL_VERSION);
    }

    #[test]
    fn tools_list_covers_read_tools() {
        let Some(mut server) = notepad_project() else {
            return;
        };
        let response = call(&mut server, "tools/list", json!({}));
        let tools = response["result"]["tools"].as_array().unwrap();
        let names: Vec<&str> = tools
            .iter()
            .filter_map(|tool| tool["name"].as_str())
            .collect();
        for expected in [
            "list_functions",
            "get_disassembly",
            "get_pseudocode",
            "xrefs_to",
            "list_strings",
            "callgraph",
            "search_bytes",
            "function_summary",
            "find_api_callers",
        ] {
            assert!(names.contains(&expected), "missing tool {expected}");
        }
        // Every tool advertises its intent and a result schema, so a client can auto-approve
        // the read-only ones and consume `structuredContent`.
        for tool in tools {
            let name = tool["name"].as_str().unwrap();
            assert!(
                !tool["title"].as_str().unwrap_or_default().is_empty(),
                "{name}"
            );
            assert_eq!(
                tool["annotations"]["readOnlyHint"], true,
                "{name} should be read-only"
            );
            assert_eq!(tool["outputSchema"]["type"], "object", "{name}");
        }
    }

    #[test]
    fn tool_calls_return_project_data() {
        let Some(mut server) = notepad_project() else {
            return;
        };
        let functions = call(
            &mut server,
            "tools/call",
            json!({"name": "list_functions", "arguments": {"limit": 10}}),
        );
        let text = functions["result"]["content"][0]["text"].as_str().unwrap();
        let parsed: Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["count"], 10, "{text}");
        assert!(!parsed["functions"][0]["name"].is_null());
        // The same payload is available in structured form for 2025-06-18 clients.
        assert_eq!(
            functions["result"]["structuredContent"]["count"], 10,
            "{functions}"
        );
        assert_eq!(
            functions["result"]["structuredContent"], parsed,
            "structuredContent must mirror the text payload"
        );

        let disassembly = call(
            &mut server,
            "tools/call",
            json!({"name": "get_disassembly", "arguments": {"addr": "entry", "count": 8}}),
        );
        let text = disassembly["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let parsed: Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["count"], 8);

        let pseudo = call(
            &mut server,
            "tools/call",
            json!({"name": "get_pseudocode", "arguments": {"addr": "entry", "level": "l1"}}),
        );
        let text = pseudo["result"]["content"][0]["text"].as_str().unwrap();
        let parsed: Value = serde_json::from_str(text).unwrap();
        assert!(
            parsed["text"]
                .as_str()
                .unwrap()
                .contains("structure recovery"),
            "unexpected pseudocode: {text}"
        );

        let hits = call(
            &mut server,
            "tools/call",
            json!({"name": "search_bytes", "arguments": {"ascii": "This program", "limit": 5}}),
        );
        let text = hits["result"]["content"][0]["text"].as_str().unwrap();
        let parsed: Value = serde_json::from_str(text).unwrap();
        assert!(parsed["count"].as_u64().unwrap() >= 1, "{text}");
    }

    #[test]
    fn resources_read_exposes_collections() {
        let Some(mut server) = notepad_project() else {
            return;
        };
        let uri = format!("recraft://project/{}/functions", server.project_id);
        let response = call(&mut server, "resources/read", json!({"uri": uri}));
        let text = response["result"]["contents"][0]["text"].as_str().unwrap();
        let parsed: Value = serde_json::from_str(text).unwrap();
        assert!(parsed["count"].as_u64().unwrap() > 0);
    }

    #[test]
    fn xref_lists_report_their_totals() {
        let Some(mut server) = notepad_project() else {
            return;
        };
        let empty = tool_json(&mut server, "xrefs_to", json!({"addr": "entry"}));
        assert_eq!(empty["count"], 0, "{empty}");
        assert_eq!(empty["total"], 0, "{empty}");
        assert_eq!(empty["truncated"], false, "{empty}");

        let capped = tool_json(
            &mut server,
            "xrefs_from",
            json!({"addr": "start", "limit": 1}),
        );
        assert!(capped["count"].as_u64().unwrap() <= 1, "{capped}");
        assert!(capped["truncated"].is_boolean(), "{capped}");
    }

    #[test]
    fn pseudocode_resource_marks_its_truncation() {
        let Some(mut server) = notepad_project() else {
            return;
        };
        let project_id = server.project_id.clone();
        let all = tool_json(&mut server, "list_functions", json!({"limit": MAX_LIMIT}));
        let biggest = all["functions"]
            .as_array()
            .unwrap()
            .iter()
            .max_by_key(|function| function["size"].as_u64().unwrap_or(0))
            .unwrap()["start"]
            .as_str()
            .unwrap()
            .to_owned();
        let uri = format!("recraft://project/{project_id}/pseudocode/{biggest}");
        let response = call(&mut server, "resources/read", json!({"uri": uri}));
        let text = response["result"]["contents"][0]["text"].as_str().unwrap();
        assert!(
            text.contains("// truncated:"),
            "resource served a silently short document ({} bytes)",
            text.len()
        );
    }

    #[test]
    fn unknown_method_is_an_error_and_notifications_are_silent() {
        let Some(mut server) = notepad_project() else {
            return;
        };
        let response = call(&mut server, "does/not/exist", json!({}));
        assert_eq!(response["error"]["code"], -32601);
        assert!(
            server
                .handle_message(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}))
                .is_none()
        );
    }

    #[test]
    fn invalid_tool_name_is_reported() {
        let Some(mut server) = notepad_project() else {
            return;
        };
        let response = call(
            &mut server,
            "tools/call",
            json!({"name": "drop_database", "arguments": {}}),
        );
        assert_eq!(response["error"]["code"], -32602);
    }

    #[test]
    fn function_filter_runs_before_the_limit() {
        let Some(mut server) = notepad_project() else {
            return;
        };
        let all = tool_json(&mut server, "list_functions", json!({"limit": MAX_LIMIT}));
        let functions = all["functions"].as_array().unwrap();
        if functions.len() <= 500 {
            return;
        }
        // A match past the default 500-row window used to disappear entirely.
        let name = functions.last().unwrap()["name"]
            .as_str()
            .expect("analysis names every function")
            .to_owned();
        let filtered = tool_json(&mut server, "list_functions", json!({"filter": name}));
        assert_eq!(filtered["count"], 1, "{filtered}");
        assert_eq!(filtered["total"], 1, "{filtered}");
        assert_eq!(filtered["truncated"], false, "{filtered}");

        // The unfiltered default window must admit that it dropped rows.
        let windowed = tool_json(&mut server, "list_functions", json!({}));
        assert_eq!(windowed["truncated"], true, "{windowed}");
        assert_eq!(
            windowed["total"].as_u64().unwrap(),
            functions.len() as u64,
            "{windowed}"
        );
    }

    #[test]
    fn string_filter_runs_before_the_limit() {
        let Some(mut server) = notepad_project() else {
            return;
        };
        let all = tool_json(&mut server, "list_strings", json!({"limit": MAX_LIMIT}));
        let strings = all["strings"].as_array().unwrap();
        if strings.len() <= 500 {
            return;
        }
        let last_text = strings.last().unwrap()["text"].as_str().unwrap().to_owned();
        if last_text.is_empty() {
            return;
        }
        let filtered = tool_json(&mut server, "list_strings", json!({"filter": last_text}));
        assert!(filtered["count"].as_u64().unwrap() >= 1, "{filtered}");
        assert_eq!(filtered["truncated"], false, "{filtered}");

        let windowed = tool_json(&mut server, "list_strings", json!({}));
        assert_eq!(windowed["truncated"], true, "{windowed}");
        assert_eq!(
            windowed["total"].as_u64().unwrap(),
            strings.len() as u64,
            "{windowed}"
        );
    }

    #[test]
    fn pseudocode_reports_and_bounds_truncation() {
        let Some(mut server) = notepad_project() else {
            return;
        };
        let limited = tool_json(
            &mut server,
            "get_pseudocode",
            json!({"addr": "entry", "level": "l1", "maxLines": 2, "includeLines": true}),
        );
        assert_eq!(limited["truncated"], true, "{limited}");
        assert_eq!(limited["lines"].as_array().unwrap().len(), 2);
        assert_eq!(limited["returned_lines"], 2);
        assert!(limited["total_lines"].as_u64().unwrap() > 2);
        assert!(limited["hint"].as_str().unwrap().contains("startLine"));
        assert_eq!(limited["text"].as_str().unwrap().lines().count(), 2);
        assert_eq!(limited["next_start_line"], 3);

        // Without `includeLines` the payload must not carry the duplicate array.
        let compact = tool_json(
            &mut server,
            "get_pseudocode",
            json!({"addr": "entry", "level": "l1", "maxLines": 2}),
        );
        assert!(compact.get("lines").is_none(), "{compact}");
        assert!(compact["text"].as_str().unwrap().lines().count() <= 2);

        // The largest function must stay inside the default byte/line caps.
        let all = tool_json(&mut server, "list_functions", json!({"limit": MAX_LIMIT}));
        let biggest = all["functions"]
            .as_array()
            .unwrap()
            .iter()
            .max_by_key(|function| function["size"].as_u64().unwrap_or(0))
            .unwrap()["start"]
            .as_str()
            .unwrap()
            .to_owned();
        let full = tool_json(
            &mut server,
            "get_pseudocode",
            json!({"addr": biggest, "level": "l2"}),
        );
        let budget = Budget::mcp();
        let text = full["text"].as_str().unwrap();
        assert!(
            text.len() <= budget.pseudo_bytes,
            "L2 payload not capped: {} bytes",
            text.len()
        );
        let kept = full["returned_lines"].as_u64().unwrap();
        assert!(kept <= budget.pseudo_lines as u64);
        assert_eq!(
            full["truncated"],
            kept < full["total_lines"].as_u64().unwrap()
        );
    }

    #[test]
    fn pages_walk_past_the_row_and_line_caps() {
        let Some(mut server) = notepad_project() else {
            return;
        };
        let total = tool_json(&mut server, "list_functions", json!({"limit": 1}))["total"]
            .as_u64()
            .unwrap();
        assert!(total > 4, "notepad should have more than four functions");

        let first = tool_json(&mut server, "list_functions", json!({"limit": 3}));
        assert_eq!(first["count"], 3);
        assert_eq!(first["next_offset"], 3);
        let second = tool_json(
            &mut server,
            "list_functions",
            json!({"limit": 3, "offset": 3}),
        );
        assert_eq!(second["offset"], 3);
        let first_starts: Vec<&str> = first["functions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|function| function["start"].as_str().unwrap())
            .collect();
        let second_starts: Vec<&str> = second["functions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|function| function["start"].as_str().unwrap())
            .collect();
        assert!(
            first_starts
                .iter()
                .all(|start| !second_starts.contains(start)),
            "offset re-served rows: {first_starts:?} vs {second_starts:?}"
        );

        // Walking every page reaches every function exactly once.
        let mut seen = std::collections::BTreeSet::new();
        let mut offset = 0u64;
        loop {
            let page = tool_json(
                &mut server,
                "list_functions",
                json!({"limit": 100, "offset": offset}),
            );
            for function in page["functions"].as_array().unwrap() {
                assert!(
                    seen.insert(function["start"].as_str().unwrap().to_owned()),
                    "duplicate row while paging"
                );
            }
            match page["next_offset"].as_u64() {
                Some(next) => offset = next,
                None => break,
            }
        }
        assert_eq!(seen.len() as u64, total, "paging lost or duplicated rows");

        // Pseudocode pages stitch back into the text they were cut from. A mid-size function
        // keeps this test fast: the point is the paging arithmetic, not the decompiler.
        let all = tool_json(&mut server, "list_functions", json!({"limit": MAX_LIMIT}));
        let medium = all["functions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|function| function["size"].as_u64().unwrap_or(0) >= 2048)
            .min_by_key(|function| function["size"].as_u64().unwrap_or(0))
            .expect("notepad has a function larger than 2 KiB")["start"]
            .as_str()
            .unwrap()
            .to_owned();
        let page_one = tool_json(
            &mut server,
            "get_pseudocode",
            json!({"addr": medium, "level": "l2", "maxLines": 5}),
        );
        assert_eq!(page_one["truncated"], true, "{page_one}");
        assert!(page_one["total_lines"].as_u64().unwrap() > 5, "{page_one}");
        let next = page_one["next_start_line"].as_u64().unwrap();
        let page_two = tool_json(
            &mut server,
            "get_pseudocode",
            json!({"addr": medium, "level": "l2", "maxLines": 5, "startLine": next}),
        );
        assert_eq!(page_two["start_line"], next);
        assert!(
            page_two["total_lines"] == page_one["total_lines"],
            "line numbering changed between pages"
        );
        let empty_page = tool_json(
            &mut server,
            "get_pseudocode",
            json!({"addr": medium, "level": "l2", "startLine": 100_000}),
        );
        assert_eq!(empty_page["returned_lines"], 0, "{empty_page}");
    }

    #[test]
    fn initialize_echoes_supported_protocol_version() {
        let Some(mut server) = notepad_project() else {
            return;
        };
        let echoed = call(
            &mut server,
            "initialize",
            json!({"protocolVersion": "2025-06-18"}),
        );
        assert_eq!(echoed["result"]["protocolVersion"], "2025-06-18");
        let fallback = call(
            &mut server,
            "initialize",
            json!({"protocolVersion": "2099-01-01"}),
        );
        assert_eq!(fallback["result"]["protocolVersion"], PROTOCOL_VERSION);
        assert!(
            fallback["result"]["instructions"]
                .as_str()
                .unwrap()
                .contains("read-only"),
            "{fallback}"
        );
    }

    #[test]
    fn bearer_comparison_is_exact() {
        assert!(bearer_matches(Some("Bearer s3cret"), "s3cret"));
        assert!(!bearer_matches(None, "s3cret"));
        assert!(!bearer_matches(Some("Bearer s3cret"), "s3cret2"));
        assert!(!bearer_matches(Some("Bearer s3cret2"), "s3cret"));
        assert!(!bearer_matches(Some("Basic s3cret"), "s3cret"));
        assert!(bytes_equal_ct(b"", b""));
    }

    fn tool_names(server: &mut McpServer) -> Vec<String> {
        let response = call(server, "tools/list", json!({}));
        response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|tool| tool["name"].as_str().map(str::to_owned))
            .collect()
    }

    fn first_function(server: &mut McpServer) -> String {
        let functions = tool_json(server, "list_functions", json!({"limit": 1}));
        functions["functions"][0]["start"]
            .as_str()
            .expect("first function start")
            .to_owned()
    }

    #[test]
    fn write_tools_are_opt_in() {
        let Some(mut read_only) = notepad_project() else {
            return;
        };
        let names = tool_names(&mut read_only);
        assert!(!names.iter().any(|name| name == "rename"), "{names:?}");
        assert!(!names.iter().any(|name| name == "undo"), "{names:?}");

        let Some((mut writable, _project)) = notepad_writable() else {
            return;
        };
        let names = tool_names(&mut writable);
        for tool in [
            "rename",
            "set_comment",
            "set_prototype",
            "apply_type",
            "define_function",
            "batch",
            "undo",
        ] {
            assert!(
                names.iter().any(|name| name == tool),
                "missing write tool {tool}: {names:?}"
            );
        }
        // Write tools must not be advertised as read-only, and `undo`/`batch` must not claim
        // to be idempotent.
        let response = call(&mut writable, "tools/list", json!({}));
        for tool in response["result"]["tools"].as_array().unwrap() {
            let name = tool["name"].as_str().unwrap();
            if recraft_query::is_read_tool(name) {
                continue;
            }
            assert_eq!(tool["annotations"]["readOnlyHint"], false, "{name}");
            assert_eq!(tool["annotations"]["destructiveHint"], true, "{name}");
            if name == "undo" || name == "batch" {
                assert_eq!(tool["annotations"]["idempotentHint"], false, "{name}");
            }
        }
    }

    #[test]
    fn batch_applies_and_undoes_as_one_group() {
        let Some((mut server, _project)) = notepad_writable() else {
            return;
        };
        let first = first_function(&mut server);
        let all = tool_json(&mut server, "list_functions", json!({"limit": 2}));
        let second = all["functions"][1]["start"].as_str().unwrap().to_owned();

        let applied = tool_json(
            &mut server,
            "batch",
            json!({"commands": [
                {"tool": "rename", "arguments": {"addr": first, "name": "batch_one"}},
                {"tool": "rename", "arguments": {"addr": second, "name": "batch_two"}},
                {"tool": "set_comment", "arguments": {"addr": first, "text": "batch note"}},
            ]}),
        );
        assert_eq!(applied["count"], 3, "{applied}");
        assert!(applied["seq"].as_u64().unwrap() >= 3, "{applied}");

        let named = tool_json(&mut server, "list_functions", json!({"filter": "batch_"}));
        assert_eq!(named["count"], 2, "{named}");

        // A dry run reports the three records without reverting anything.
        let preview = tool_json(&mut server, "undo", json!({"dryRun": true}));
        let would = preview["would_undo"].as_array().unwrap();
        assert_eq!(would.len(), 3, "{preview}");
        assert!(would.iter().all(|record| record["origin"] == "mcp"));
        assert!(would.iter().all(|record| record["batch"].is_number()));
        assert_eq!(
            tool_json(&mut server, "list_functions", json!({"filter": "batch_"}))["count"],
            2,
            "dry run must not revert anything"
        );

        // One undo removes all three.
        let undone = tool_json(&mut server, "undo", json!({}));
        assert_eq!(undone["records"].as_array().unwrap().len(), 3, "{undone}");
        assert_eq!(
            tool_json(&mut server, "list_functions", json!({"filter": "batch_"}))["count"],
            0
        );
    }

    #[test]
    fn undo_seq_only_accepts_the_newest_record() {
        let Some((mut server, _project)) = notepad_writable() else {
            return;
        };
        let first = first_function(&mut server);
        let one = tool_json(
            &mut server,
            "rename",
            json!({"addr": first, "name": "seq_one"}),
        );
        let one_seq = one["seq"].as_i64().unwrap();
        let two = tool_json(
            &mut server,
            "rename",
            json!({"addr": first, "name": "seq_two"}),
        );
        let two_seq = two["seq"].as_i64().unwrap();
        assert!(two_seq > one_seq);

        let rejected = call(
            &mut server,
            "tools/call",
            json!({"name": "undo", "arguments": {"seq": one_seq}}),
        );
        assert_eq!(rejected["result"]["isError"], true, "{rejected}");
        assert!(
            rejected["result"]["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("applied later"),
            "{rejected}"
        );

        let undone = tool_json(&mut server, "undo", json!({"seq": two_seq}));
        assert_eq!(undone["records"][0]["seq"], two_seq, "{undone}");
        assert_eq!(undone["records"][0]["cmd"], "rename");
        assert_eq!(undone["records"][0]["origin"], "mcp");
        assert_eq!(
            tool_json(&mut server, "list_functions", json!({"filter": "seq_"}))["count"],
            1
        );
    }

    #[test]
    fn read_only_server_rejects_write_tools() {
        let Some(mut server) = notepad_project() else {
            return;
        };
        let response = call(
            &mut server,
            "tools/call",
            json!({"name": "rename", "arguments": {"addr": "entry", "name": "x"}}),
        );
        assert_eq!(response["result"]["isError"], true, "{response}");
        let text = response["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(text.contains("--write"), "{text}");
    }

    #[test]
    fn rename_is_journaled_and_undoable() {
        let Some((mut server, project)) = notepad_writable() else {
            return;
        };
        let addr = first_function(&mut server);
        let result = tool_json(
            &mut server,
            "rename",
            json!({"addr": addr, "name": "mcp_renamed"}),
        );
        assert_eq!(result["value"], "mcp_renamed", "{result}");
        assert_eq!(result["undo"], true, "{result}");
        let after = tool_json(
            &mut server,
            "list_functions",
            json!({"filter": "mcp_renamed"}),
        );
        assert_eq!(after["total"], 1, "{after}");

        let undone = tool_json(&mut server, "undo", json!({}));
        assert_eq!(undone["undone"], true, "{undone}");
        let after = tool_json(
            &mut server,
            "list_functions",
            json!({"filter": "mcp_renamed"}),
        );
        assert_eq!(after["total"], 0, "{after}");
        drop(server);

        // Both the rename and its undo reached the project file.
        let session = Session::open(&project).unwrap();
        assert!(session.addr_of_name("mcp_renamed").unwrap().is_none());
    }

    #[test]
    fn comment_and_prototype_round_trip() {
        let Some((mut server, _project)) = notepad_writable() else {
            return;
        };
        let addr = first_function(&mut server);
        let comment = tool_json(
            &mut server,
            "set_comment",
            json!({"addr": addr, "text": "entry point"}),
        );
        assert_eq!(comment["value"], "entry point", "{comment}");

        let prototype = tool_json(
            &mut server,
            "set_prototype",
            json!({"addr": addr, "text": "int __cdecl entry(void)"}),
        );
        assert_eq!(prototype["value"], "int __cdecl entry(void)", "{prototype}");

        // `null` clears the slot and reports what was there.
        let cleared = tool_json(
            &mut server,
            "set_comment",
            json!({"addr": addr, "text": null}),
        );
        assert_eq!(cleared["previous"], "entry point", "{cleared}");
        assert_eq!(cleared["value"], Value::Null, "{cleared}");
    }

    #[test]
    fn undo_with_an_empty_journal_reports_nothing_to_do() {
        let Some((mut server, _project)) = notepad_writable() else {
            return;
        };
        let undone = tool_json(&mut server, "undo", json!({}));
        assert_eq!(undone["undone"], false, "{undone}");
    }
}
