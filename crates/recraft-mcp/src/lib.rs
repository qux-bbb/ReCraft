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

#![forbid(unsafe_code)]

use std::io::{BufRead, Write};
use std::path::Path;

use recraft_common::{Address, Error, Result};
use recraft_core::Session;
use serde_json::{Value, json};

const PROTOCOL_VERSION: &str = "2024-11-05";
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
            "initialize" => Ok(json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {
                    "tools": {"listChanged": false},
                    "resources": {"subscribe": false, "listChanged": false},
                },
                "serverInfo": {"name": "recraft", "version": env!("CARGO_PKG_VERSION")},
            })),
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
        let mut items = Vec::new();
        for function in functions.into_iter().take(limit) {
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
            items.push(json!({
                "name": name,
                "start": hex(function.start),
                "end": hex(function.end),
                "size": function.end.get().saturating_sub(function.start.get()),
            }));
        }
        Ok(json!({"functions": items, "count": items.len()}))
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
        let lines: Vec<Value> = code
            .lines
            .iter()
            .map(|line| json!({"addr": hex(line.addr), "text": line.text}))
            .collect();
        Ok(json!({
            "function": hex(function),
            "name": name,
            "level": level,
            "lines": lines,
            "text": code.text(),
        }))
    }

    fn tool_xrefs(&self, arguments: &Value, incoming: bool) -> ToolResult {
        let addr = self
            .resolve_address(arguments.get("addr"))
            .map_err(|e| e.to_string())?;
        let entries = if incoming {
            self.session.xrefs_to(addr)
        } else {
            self.session.xrefs_from(addr)
        }
        .map_err(|e| e.to_string())?;
        let items: Vec<Value> = entries
            .iter()
            .map(|entry| {
                json!({
                    "from": hex(entry.from),
                    "to": hex(entry.to),
                    "kind": xref_kind_name(entry.kind),
                })
            })
            .collect();
        Ok(json!({"addr": hex(addr), "xrefs": items, "count": items.len()}))
    }

    fn tool_strings(&mut self, arguments: &Value) -> ToolResult {
        let filter = arguments
            .get("filter")
            .and_then(Value::as_str)
            .map(str::to_ascii_lowercase);
        let limit = limit_of(arguments, 500);
        let strings = self.session.strings().map_err(|e| e.to_string())?;
        let mut items = Vec::new();
        for entry in strings.into_iter().take(limit) {
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
            items.push(json!({
                "addr": hex(entry.addr),
                "length": entry.len,
                "encoding": if entry.enc == 2 { "utf-16le" } else { "ascii" },
                "text": text,
            }));
        }
        Ok(json!({"strings": items, "count": items.len()}))
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
        Ok(json!({"nodes": nodes, "edges": edges, "truncated": edges.len() >= limit}))
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
        let mut hits = Vec::new();
        let segments = self.session.segments().to_vec();
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
                let chunk = self
                    .session
                    .read_bytes(Address::new(start + offset), len)
                    .map_err(|e| e.to_string())?;
                for index in 0..=chunk.len().saturating_sub(needle.len()) {
                    if chunk[index..index + needle.len()] == needle[..] {
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
                    value
                        .get("text")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                        .unwrap_or_else(|| value.to_string())
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
            "description": "List discovered functions with names and ranges.",
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
            "description": "Pseudo-C for the function containing an address (L0/L1/L2).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "addr": {"type": ["integer", "string"]},
                    "level": {"type": "string", "enum": ["l0", "l1", "l2"]},
                },
            },
        }),
        json!({
            "name": "xrefs_to",
            "description": "References pointing at an address.",
            "inputSchema": {
                "type": "object",
                "properties": {"addr": {"type": ["integer", "string"]}},
                "required": ["addr"],
            },
        }),
        json!({
            "name": "xrefs_from",
            "description": "References originating at an address.",
            "inputSchema": {
                "type": "object",
                "properties": {"addr": {"type": ["integer", "string"]}},
                "required": ["addr"],
            },
        }),
        json!({
            "name": "list_strings",
            "description": "Extracted strings with encoding and address.",
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

    fn call(server: &mut McpServer, method: &str, params: Value) -> Value {
        server
            .handle_message(&json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}))
            .expect("response expected")
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
}
