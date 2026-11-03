// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Model Context Protocol server exposing the session API as tools and resources.
//!
//! Transport: newline-delimited JSON-RPC 2.0 over stdio (MCP stdio transport). The
//! server is **read-only** in this milestone: every tool observes the project, never
//! mutates it. Write tools (`rename`, `set_comment`, `apply_type`, ...) arrive with the
//! propose/apply protocol in M3.
//!
//! Tools: `list_functions`, `get_disassembly`, `get_pseudocode`, `xrefs_to`,
//! `xrefs_from`, `list_strings`, `list_imports`, `callgraph`, `search_bytes`,
//! `function_summary`.
//!
//! Resources: `recraft://project/{id}/functions|strings|imports` and
//! `recraft://project/{id}/pseudocode/{func}`.
//!
//! Every list-shaped tool reports `total` (matches before the row cap) and `truncated`
//! (rows were dropped), and filters run before the cap: a filtered query never answers
//! "no match" just because the match sits past the default window.

#![forbid(unsafe_code)]

use std::io::{BufRead, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

use recraft_common::{Address, Error, Result};
use recraft_core::Session;
use serde_json::{Value, json};

const PROTOCOL_VERSION: &str = "2024-11-05";
/// Protocol revisions this server speaks; `initialize` echoes the client's choice when it is
/// one of these, and falls back to [`PROTOCOL_VERSION`] otherwise.
const SUPPORTED_PROTOCOL_VERSIONS: [&str; 2] = ["2024-11-05", "2025-06-18"];
/// Server instructions. Clients copy this into the system prompt, so it states the read-only
/// contract and the truncation convention rather than describing the project.
const SERVER_INSTRUCTIONS: &str = "ReCraft serves one read-only reverse-engineering project. \
Start with list_functions (its filter is a case-insensitive name substring), then \
function_summary or get_pseudocode for one function; addresses accept 0x hex, decimal, a \
symbol name, or 'entry'. Large payloads report truncated:true together with the untruncated \
size; narrow the query or raise maxLines/maxBytes instead of assuming the result is complete.";
/// Default `get_pseudocode` caps. A single L2 function in notepad.exe renders 9 855 lines
/// (~1.5 MB), which exceeds what most MCP clients accept as one tool result.
const DEFAULT_PSEUDOCODE_MAX_LINES: usize = 2000;
const DEFAULT_PSEUDOCODE_MAX_BYTES: usize = 256 * 1024;
const MAX_PSEUDOCODE_LINES: usize = 100_000;
const MAX_PSEUDOCODE_BYTES: usize = 8 * 1024 * 1024;
const XREF_CALL: i64 = 1;
const XREF_DATA: i64 = 3;
const MAX_LIMIT: usize = 5000;

type ToolResult = std::result::Result<Value, String>;

/// A project opened for MCP access.
pub struct McpServer {
    session: Session,
    project_id: String,
}

impl McpServer {
    pub fn open(path: &Path) -> Result<Self> {
        let session = Session::open(path)?;
        let project_id = session
            .path()
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_else(|| "project".to_owned());
        Ok(McpServer {
            session,
            project_id,
        })
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
                    "instructions": SERVER_INSTRUCTIONS,
                }))
            }
            "notifications/initialized" | "notifications/cancelled" => return None,
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({"tools": tools()})),
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
        let outcome: ToolResult = match name {
            "list_functions" => self.tool_list_functions(&arguments),
            "get_disassembly" => self.tool_disassembly(&arguments),
            "get_pseudocode" => self.tool_pseudocode(&arguments),
            "xrefs_to" => self.tool_xrefs(&arguments, true),
            "xrefs_from" => self.tool_xrefs(&arguments, false),
            "list_strings" => self.tool_strings(&arguments),
            "list_imports" => self.tool_imports(&arguments),
            "callgraph" => self.tool_callgraph(&arguments),
            "search_bytes" => self.tool_search(&arguments),
            "function_summary" => self.tool_summary(&arguments),
            other => return Err((-32602, format!("unknown tool: {other}"))),
        };
        match outcome {
            Ok(value) => Ok(json!({
                "content": [{"type": "text", "text": value.to_string()}],
                "isError": false,
            })),
            Err(message) => Ok(json!({
                "content": [{"type": "text", "text": format!("error: {message}")}],
                "isError": true,
            })),
        }
    }

    fn tool_list_functions(&self, arguments: &Value) -> ToolResult {
        let filter = arguments
            .get("filter")
            .and_then(Value::as_str)
            .map(str::to_ascii_lowercase);
        let limit = limit_of(arguments, 500);
        let functions = self.session.functions().map_err(|e| e.to_string())?;
        let total_available = functions.len();
        let mut items = Vec::new();
        let mut matched = 0usize;
        for function in &functions {
            // Without a filter the scan stops at the limit; with one it has to reach the end,
            // because the match may sit anywhere in the (address-ordered) table.
            if filter.is_none() && items.len() >= limit {
                break;
            }
            let name = self
                .session
                .name_at(function.start)
                .map_err(|e| e.to_string())?;
            if let Some(filter) = &filter
                && !name
                    .as_deref()
                    .map(|name| name.to_ascii_lowercase().contains(filter))
                    .unwrap_or(false)
            {
                continue;
            }
            matched += 1;
            // The filter runs *before* the limit: `limit` caps returned rows, never the scan,
            // otherwise a default-500 query silently answers "no match" for later entries.
            if items.len() < limit {
                items.push(json!({
                    "name": name,
                    "start": hex(function.start),
                    "end": hex(function.end),
                    "size": function.end.get().saturating_sub(function.start.get()),
                }));
            }
        }
        let total = if filter.is_some() {
            matched
        } else {
            total_available
        };
        Ok(json!({
            "functions": items,
            "count": items.len(),
            "total": total,
            "truncated": total > items.len(),
        }))
    }

    fn tool_disassembly(&mut self, arguments: &Value) -> ToolResult {
        let start = self
            .resolve_address(arguments.get("addr"))
            .map_err(|e| e.to_string())?;
        let count = arguments
            .get("count")
            .and_then(Value::as_u64)
            .map(|count| (count as usize).clamp(1, 4096))
            .unwrap_or(64);
        let lines = self
            .session
            .disasm(start, count)
            .map_err(|e| e.to_string())?;
        let items: Vec<Value> = lines
            .iter()
            .map(|line| {
                json!({
                    "addr": hex(line.addr),
                    "bytes": line.bytes().iter().map(|byte| format!("{byte:02x}")).collect::<Vec<_>>().join(" "),
                    "mnemonic": line.mnemonic,
                    "operands": line.operands,
                    "name": line.name,
                    "comment": line.comment,
                })
            })
            .collect();
        Ok(json!({"start": hex(start), "lines": items, "count": items.len()}))
    }

    fn tool_pseudocode(&mut self, arguments: &Value) -> ToolResult {
        let addr = self
            .resolve_address(arguments.get("addr"))
            .map_err(|e| e.to_string())?;
        let level = arguments
            .get("level")
            .and_then(Value::as_str)
            .unwrap_or("l1")
            .to_ascii_lowercase();
        let functions = self.session.functions().map_err(|e| e.to_string())?;
        let function = functions
            .iter()
            .find(|function| addr >= function.start && addr < function.end)
            .map(|function| function.start)
            .unwrap_or(addr);
        let code = match level.as_str() {
            "l0" => recraft_decompile::pseudocode_l0(&mut self.session, function, 2000),
            "l2" => recraft_decompile::pseudocode_l2(&mut self.session, function),
            _ => recraft_decompile::pseudocode_l1(&mut self.session, function),
        }
        .map_err(|e| e.to_string())?;
        let name = self.session.name_at(function).map_err(|e| e.to_string())?;
        // Cap the response: one huge L2 function otherwise returns megabytes over the wire and
        // can exceed the client's tool-result budget outright.
        let max_lines = cap_of(
            arguments,
            "maxLines",
            DEFAULT_PSEUDOCODE_MAX_LINES,
            1,
            MAX_PSEUDOCODE_LINES,
        );
        let max_bytes = cap_of(
            arguments,
            "maxBytes",
            DEFAULT_PSEUDOCODE_MAX_BYTES,
            1024,
            MAX_PSEUDOCODE_BYTES,
        );
        let total_lines = code.lines.len();
        let mut lines = Vec::new();
        let mut text = String::new();
        for line in &code.lines {
            if lines.len() >= max_lines
                || (!lines.is_empty() && text.len() + line.text.len() + 1 > max_bytes)
            {
                break;
            }
            text.push_str(&line.text);
            text.push('\n');
            lines.push(json!({"addr": hex(line.addr), "text": line.text}));
        }
        let kept = lines.len();
        let truncated = kept < total_lines;
        let mut payload = json!({
            "function": hex(function),
            "name": name,
            "level": level,
            "lines": lines,
            "text": text,
            "total_lines": total_lines,
            "truncated": truncated,
        });
        if truncated {
            payload["hint"] = json!(format!(
                "truncated to {kept} of {total_lines} lines; raise maxLines/maxBytes, or export the full text with `recraft-cli decompile --out`"
            ));
        }
        Ok(payload)
    }

    fn tool_xrefs(&self, arguments: &Value, incoming: bool) -> ToolResult {
        let addr = self
            .resolve_address(arguments.get("addr"))
            .map_err(|e| e.to_string())?;
        let limit = limit_of(arguments, MAX_LIMIT);
        let entries = if incoming {
            self.session.xrefs_to(addr)
        } else {
            self.session.xrefs_from(addr)
        }
        .map_err(|e| e.to_string())?;
        let total = entries.len();
        let items: Vec<Value> = entries
            .iter()
            .take(limit)
            .map(|entry| {
                json!({
                    "from": hex(entry.from),
                    "to": hex(entry.to),
                    "kind": xref_kind_name(entry.kind),
                })
            })
            .collect();
        Ok(json!({
            "addr": hex(addr),
            "xrefs": items,
            "count": items.len(),
            "total": total,
            "truncated": total > items.len(),
        }))
    }

    fn tool_strings(&mut self, arguments: &Value) -> ToolResult {
        let filter = arguments
            .get("filter")
            .and_then(Value::as_str)
            .map(str::to_ascii_lowercase);
        let limit = limit_of(arguments, 500);
        let strings = self.session.strings().map_err(|e| e.to_string())?;
        let total_available = strings.len();
        let mut items = Vec::new();
        let mut matched = 0usize;
        for entry in &strings {
            if filter.is_none() && items.len() >= limit {
                break;
            }
            let text = self
                .session
                .read_bytes(entry.addr, (entry.len as usize).max(2))
                .map(|bytes| decode_string(&bytes, entry.enc))
                .unwrap_or_default();
            if let Some(filter) = &filter
                && !text.to_ascii_lowercase().contains(filter)
            {
                continue;
            }
            matched += 1;
            // Same convention as `list_functions`: filtering precedes the row cap.
            if items.len() < limit {
                items.push(json!({
                    "addr": hex(entry.addr),
                    "length": entry.len,
                    "encoding": if entry.enc == 2 { "utf-16le" } else { "ascii" },
                    "text": text,
                }));
            }
        }
        let total = if filter.is_some() {
            matched
        } else {
            total_available
        };
        Ok(json!({
            "strings": items,
            "count": items.len(),
            "total": total,
            "truncated": total > items.len(),
        }))
    }

    fn tool_imports(&self, _arguments: &Value) -> ToolResult {
        let image = self.session.image();
        let modules: Vec<Value> = image
            .imports
            .iter()
            .map(|module| {
                let functions: Vec<Value> = module
                    .functions
                    .iter()
                    .map(|function| {
                        json!({
                            "name": function.name,
                            "ordinal": function.ordinal,
                            "hint": function.hint,
                            "iat": hex(function.iat_va),
                        })
                    })
                    .collect();
                json!({
                    "module": module.name,
                    "iat": hex(module.iat_va),
                    "functions": functions,
                })
            })
            .collect();
        Ok(json!({"modules": modules}))
    }

    fn tool_callgraph(&self, arguments: &Value) -> ToolResult {
        let limit = limit_of(arguments, 2000);
        let functions = self.session.functions().map_err(|e| e.to_string())?;
        let index = FunctionIndex::new(&functions);
        let calls = self
            .session
            .all_xrefs(Some(XREF_CALL))
            .map_err(|e| e.to_string())?;
        let mut edges = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        for call in calls {
            let Some(from) = index.containing(call.from) else {
                continue;
            };
            let Some(to) = index.containing(call.to) else {
                continue;
            };
            if from == to || !seen.insert((from, to)) {
                continue;
            }
            if edges.len() >= limit {
                break;
            }
            edges.push(json!({
                "from": hex(functions[from].start),
                "from_name": self.session.name_at(functions[from].start).ok().flatten(),
                "to": hex(functions[to].start),
                "to_name": self.session.name_at(functions[to].start).ok().flatten(),
            }));
        }
        let nodes_truncated = functions.len() > limit;
        let nodes: Vec<Value> = functions
            .iter()
            .take(limit)
            .map(|function| {
                json!({
                    "start": hex(function.start),
                    "name": self.session.name_at(function.start).ok().flatten(),
                })
            })
            .collect();
        Ok(json!({
            "nodes": nodes,
            "node_total": functions.len(),
            "edges": edges,
            "truncated": edges.len() >= limit || nodes_truncated,
        }))
    }

    fn tool_search(&mut self, arguments: &Value) -> ToolResult {
        let limit = limit_of(arguments, 20).min(1000);
        let needle = if let Some(hex) = arguments.get("hex").and_then(Value::as_str) {
            parse_hex_bytes(hex)?
        } else if let Some(ascii) = arguments.get("ascii").and_then(Value::as_str) {
            ascii.as_bytes().to_vec()
        } else {
            return Err("search_bytes needs `hex` or `ascii`".to_owned());
        };
        if needle.is_empty() {
            return Err("empty pattern".to_owned());
        }
        let hits = self
            .session
            .search_bytes(&needle, limit)
            .map_err(|e| e.to_string())?;
        let items: Vec<Value> = hits
            .iter()
            .map(|addr| json!({"addr": hex(*addr)}))
            .collect();
        Ok(json!({"hits": items, "count": items.len(), "truncated": items.len() >= limit}))
    }

    fn tool_summary(&mut self, arguments: &Value) -> ToolResult {
        let addr = self
            .resolve_address(arguments.get("addr"))
            .map_err(|e| e.to_string())?;
        let functions = self.session.functions().map_err(|e| e.to_string())?;
        let index = FunctionIndex::new(&functions);
        let Some(function) = index.containing(addr) else {
            return Err(format!("no function contains {}", hex(addr)));
        };
        let function = &functions[function];
        let name = self
            .session
            .name_at(function.start)
            .map_err(|e| e.to_string())?;

        let callers = self
            .session
            .xrefs_to(function.start)
            .map_err(|e| e.to_string())?
            .iter()
            .filter(|entry| entry.kind == XREF_CALL)
            .count();

        let strings = self.session.strings().map_err(|e| e.to_string())?;
        let string_addrs: std::collections::BTreeSet<u64> =
            strings.iter().map(|entry| entry.addr.get()).collect();

        let calls = self
            .session
            .all_xrefs(Some(XREF_CALL))
            .map_err(|e| e.to_string())?;
        let data_refs = self
            .session
            .all_xrefs(Some(XREF_DATA))
            .map_err(|e| e.to_string())?;
        let mut call_targets: Vec<Value> = Vec::new();
        let mut call_sites = Vec::new();
        let mut seen_targets = std::collections::BTreeSet::new();
        for call in &calls {
            if call.from < function.start || call.from >= function.end {
                continue;
            }
            call_sites.push(call.from);
            if seen_targets.insert(call.to.get()) {
                call_targets.push(json!({
                    "addr": hex(call.to),
                    "name": self.session.name_at(call.to).ok().flatten(),
                }));
            }
        }
        let mut referenced_strings = Vec::new();
        for data in &data_refs {
            if data.from < function.start || data.from >= function.end {
                continue;
            }
            if string_addrs.contains(&data.to.get()) {
                let text = self
                    .session
                    .read_bytes(data.to, 64)
                    .map(|bytes| decode_string(&bytes, 1))
                    .unwrap_or_default();
                referenced_strings.push(json!({"addr": hex(data.to), "text": text}));
            }
        }
        let mut api_tags = Vec::new();
        for site in call_sites {
            for (rule, category) in self.session.api_tags_at(site).map_err(|e| e.to_string())? {
                api_tags.push(json!({
                    "addr": hex(site),
                    "rule": rule,
                    "category": category,
                }));
            }
        }

        let mut instruction_count = 0usize;
        let mut cursor = function.start;
        let mut truncated = false;
        while cursor < function.end {
            if instruction_count >= 20_000 {
                truncated = true;
                break;
            }
            let Ok(Some(insn)) = self.session.insn_at(cursor) else {
                break;
            };
            instruction_count += 1;
            cursor = insn.next_addr();
        }

        Ok(json!({
            "name": name,
            "start": hex(function.start),
            "end": hex(function.end),
            "size": function.end.get().saturating_sub(function.start.get()),
            "instructions": instruction_count,
            "truncated": truncated,
            "callers": callers,
            "calls": call_targets,
            "strings": referenced_strings,
            "api_tags": api_tags,
            "segment": self
                .session
                .segment_containing(function.start)
                .map(|segment| segment.name.clone()),
        }))
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
        let (mime, result) = if rest == "functions" {
            (
                "application/json",
                self.tool_list_functions(&json!({"limit": MAX_LIMIT})),
            )
        } else if rest == "strings" {
            (
                "application/json",
                self.tool_strings(&json!({"limit": MAX_LIMIT})),
            )
        } else if rest == "imports" {
            ("application/json", self.tool_imports(&json!({})))
        } else if let Some(target) = rest.strip_prefix("pseudocode/") {
            let value = json!({"addr": target, "level": "l1"});
            ("text/plain", self.tool_pseudocode(&value))
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
                        let kept = value
                            .get("lines")
                            .and_then(Value::as_array)
                            .map(Vec::len)
                            .unwrap_or(0);
                        text.push_str(&format!(
                            "// truncated: {kept} of {total} lines; raise maxLines/maxBytes\n"
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
        let Some(value) = value else {
            return Ok(self.session.entry_point());
        };
        match value {
            Value::Number(number) => number
                .as_u64()
                .map(Address::new)
                .ok_or_else(|| Error::internal("address must be a u64")),
            Value::String(raw) => {
                let raw = raw.trim();
                if let Some(hex) = raw.strip_prefix("0x").or_else(|| raw.strip_prefix("0X")) {
                    return u64::from_str_radix(hex, 16)
                        .map(Address::new)
                        .map_err(|_| Error::internal(format!("bad hex address: {raw}")));
                }
                if let Ok(parsed) = raw.parse::<u64>() {
                    return Ok(Address::new(parsed));
                }
                if raw.eq_ignore_ascii_case("entry") {
                    return Ok(self.session.entry_point());
                }
                if let Some(addr) = self.session.addr_of_name(raw)? {
                    return Ok(addr);
                }
                Err(Error::internal(format!("unknown address or name: {raw}")))
            }
            _ => Err(Error::internal("address must be a number or string")),
        }
    }
}

fn rpc_error(id: Value, code: i64, message: impl Into<String>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {"code": code, "message": message.into()},
    })
}

fn hex(addr: Address) -> String {
    format!("{:#x}", addr.get())
}

fn limit_of(arguments: &Value, default: usize) -> usize {
    arguments
        .get("limit")
        .and_then(Value::as_u64)
        .map(|limit| (limit as usize).clamp(1, MAX_LIMIT))
        .unwrap_or(default)
}

/// Read an integer cap (`maxLines`, `maxBytes`) and clamp it into range.
fn cap_of(arguments: &Value, key: &str, default: usize, min: usize, max: usize) -> usize {
    arguments
        .get(key)
        .and_then(Value::as_u64)
        .map(|value| (value as usize).clamp(min, max))
        .unwrap_or(default)
}

/// Constant-time bearer-token check so a wrong token cannot be guessed from response timing.
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

fn xref_kind_name(kind: i64) -> &'static str {
    match kind {
        1 => "call",
        2 => "jump",
        3 => "data",
        _ => "other",
    }
}

fn parse_hex_bytes(text: &str) -> std::result::Result<Vec<u8>, String> {
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

fn decode_string(bytes: &[u8], encoding: i64) -> String {
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

/// Sorted function ranges for "which function contains this address" queries.
struct FunctionIndex<'a> {
    functions: &'a [recraft_core::FunctionInfo],
}

impl<'a> FunctionIndex<'a> {
    fn new(functions: &'a [recraft_core::FunctionInfo]) -> Self {
        FunctionIndex { functions }
    }

    fn containing(&self, addr: Address) -> Option<usize> {
        self.functions
            .iter()
            .position(|function| addr >= function.start && addr < function.end)
    }
}

// ---------------------------------------------------------------- tools ----

fn tools() -> Vec<Value> {
    vec![
        json!({
            "name": "list_functions",
            "description": "List discovered functions with names and ranges. `filter` runs before `limit`; the result reports `total` and `truncated`.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "filter": {"type": "string", "description": "Case-insensitive name substring."},
                    "limit": {"type": "integer", "minimum": 1, "maximum": MAX_LIMIT},
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
            "description": "Pseudo-C for the function containing an address (L0/L1/L2). Long functions are capped; check `truncated`/`total_lines` and raise `maxLines`/`maxBytes` for more.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "addr": {"type": ["integer", "string"]},
                    "level": {"type": "string", "enum": ["l0", "l1", "l2"]},
                    "maxLines": {"type": "integer", "minimum": 1, "maximum": MAX_PSEUDOCODE_LINES},
                    "maxBytes": {"type": "integer", "minimum": 1024, "maximum": MAX_PSEUDOCODE_BYTES},
                },
            },
        }),
        json!({
            "name": "xrefs_to",
            "description": "References pointing at an address; reports `total`/`truncated` when capped by `limit`.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "addr": {"type": ["integer", "string"]},
                    "limit": {"type": "integer", "minimum": 1, "maximum": MAX_LIMIT},
                },
                "required": ["addr"],
            },
        }),
        json!({
            "name": "xrefs_from",
            "description": "References originating at an address; reports `total`/`truncated` when capped by `limit`.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "addr": {"type": ["integer", "string"]},
                    "limit": {"type": "integer", "minimum": 1, "maximum": MAX_LIMIT},
                },
                "required": ["addr"],
            },
        }),
        json!({
            "name": "list_strings",
            "description": "Extracted strings with encoding and address. `filter` runs before `limit`; the result reports `total` and `truncated`.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "filter": {"type": "string"},
                    "limit": {"type": "integer", "minimum": 1, "maximum": MAX_LIMIT},
                },
            },
        }),
        json!({
            "name": "list_imports",
            "description": "Imported modules and functions (IAT addresses).",
            "inputSchema": {"type": "object"},
        }),
        json!({
            "name": "callgraph",
            "description": "Function-level call graph derived from persisted call xrefs.",
            "inputSchema": {
                "type": "object",
                "properties": {"limit": {"type": "integer", "minimum": 1, "maximum": MAX_LIMIT}},
            },
        }),
        json!({
            "name": "search_bytes",
            "description": "Search mapped readable segments for a byte pattern (hex or ASCII).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "hex": {"type": "string", "description": "Hex bytes, e.g. '8B 44 24 04'."},
                    "ascii": {"type": "string"},
                    "limit": {"type": "integer", "minimum": 1, "maximum": 1000},
                },
            },
        }),
        json!({
            "name": "function_summary",
            "description": "Compact summary of one function: callers, calls, strings, API tags.",
            "inputSchema": {
                "type": "object",
                "properties": {"addr": {"type": ["integer", "string"]}},
            },
        }),
    ]
}

// ------------------------------------------------------------------ HTTP -----

/// Serve JSON-RPC over HTTP (`POST /mcp`) with bearer-token authentication.
pub fn serve_http(project: &Path, bind: &str, token: &str) -> Result<()> {
    let listener = TcpListener::bind(bind).map_err(Error::from)?;
    let local = listener.local_addr().map_err(Error::from)?;
    eprintln!("[mcp] http listening on http://{local}/mcp (bearer token required)");
    serve_http_listener(listener, project.to_path_buf(), token.to_owned())
}

pub(crate) fn serve_http_listener(
    listener: TcpListener,
    project: PathBuf,
    token: String,
) -> Result<()> {
    let mut server = McpServer::open(&project)?;
    for stream in listener.incoming() {
        let Ok(mut stream) = stream else {
            continue;
        };
        let _ = handle_http_connection(&mut stream, &mut server, &token);
    }
    Ok(())
}

struct HttpRequest {
    method: String,
    path: String,
    authorization: Option<String>,
    body: String,
}

fn handle_http_connection(
    stream: &mut TcpStream,
    server: &mut McpServer,
    token: &str,
) -> Result<()> {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    let Some(request) = read_http_request(stream)? else {
        return Ok(());
    };
    let (status, body) = if request.method == "GET" && request.path == "/health" {
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
    } else {
        match serde_json::from_str::<Value>(&request.body) {
            Err(error) => (
                400,
                json!({"error": format!("parse error: {error}")}).to_string(),
            ),
            Ok(message) => match server.handle_message(&message) {
                Some(response) => (200, response.to_string()),
                None => (202, String::new()),
            },
        }
    };
    write_http_response(stream, status, &body)
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
        body: String::from_utf8_lossy(&body).into_owned(),
    }))
}

fn write_http_response(stream: &mut TcpStream, status: u16, body: &str) -> Result<()> {
    let reason = match status {
        200 => "OK",
        202 => "Accepted",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        405 => "Method Not Allowed",
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
        use std::io::{Read, Write};
        let mut stream = std::net::TcpStream::connect(addr).unwrap();
        let auth = token
            .map(|token| format!("Authorization: Bearer {token}\r\n"))
            .unwrap_or_default();
        let request = format!(
            "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{auth}Connection: close\r\n\r\n{body}",
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
            let _ = serve_http_listener(listener, project, token.to_owned());
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
        ] {
            assert!(names.contains(&expected), "missing tool {expected}");
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
            json!({"addr": "entry", "level": "l1", "maxLines": 2}),
        );
        assert_eq!(limited["truncated"], true, "{limited}");
        assert_eq!(limited["lines"].as_array().unwrap().len(), 2);
        assert!(limited["total_lines"].as_u64().unwrap() > 2);
        assert!(limited["hint"].as_str().unwrap().contains("maxLines"));
        assert_eq!(limited["text"].as_str().unwrap().lines().count(), 2);

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
        let text = full["text"].as_str().unwrap();
        assert!(
            text.len() <= DEFAULT_PSEUDOCODE_MAX_BYTES,
            "L2 payload not capped: {} bytes",
            text.len()
        );
        let kept = full["lines"].as_array().unwrap().len();
        assert!(kept <= DEFAULT_PSEUDOCODE_MAX_LINES);
        assert_eq!(
            full["truncated"],
            (kept as u64) < full["total_lines"].as_u64().unwrap()
        );
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
}
