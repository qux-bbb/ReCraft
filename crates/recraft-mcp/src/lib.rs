// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Model Context Protocol server exposing the session API as tools and resources.
//!
//! Transport: newline-delimited JSON-RPC 2.0 over stdio (MCP stdio transport), plus the
//! optional HTTP mode. The server is **read-only by default**: `--write` opts into the
//! journaled write tools (`rename`, `set_comment`, `set_prototype`, `apply_type`,
//! `define_function`) and `undo`, so a plain `serve-mcp` never mutates user data. The only
//! thing read-only mode writes is the derived `pseudocode` cache, which is rebuildable and
//! dropped on every edit.
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

use std::io::{BufRead, Write};
use std::path::Path;

use recraft_common::{Error, Result};
use recraft_core::Session;
use recraft_query::Budget;
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

mod http;
mod resources;
mod schema;
mod write;

#[cfg(test)]
pub(crate) use http::serve_http_listener_mode;
pub use http::{serve_http, serve_http_writable};
pub(crate) use schema::tools;
pub(crate) use write::is_write_tool;

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
                        "prompts": {"listChanged": false},
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
            "prompts/list" => Ok(json!({"prompts": self.prompts()})),
            "prompts/get" => self.read_prompt(&params),
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
                "define_struct" => self.tool_define_struct(&arguments),
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
}

fn rpc_error(id: Value, code: i64, message: impl Into<String>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {"code": code, "message": message.into()},
    })
}

pub(crate) fn bearer_matches(authorization: Option<&str>, token: &str) -> bool {
    let expected = format!("Bearer {token}");
    bytes_equal_ct(
        authorization.unwrap_or_default().as_bytes(),
        expected.as_bytes(),
    )
}

pub(crate) fn bytes_equal_ct(left: &[u8], right: &[u8]) -> bool {
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
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use recraft_query::MAX_LIMIT;
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
    fn metadata_and_exports_describe_the_image() {
        let Some(mut server) = notepad_project() else {
            return;
        };
        let meta = tool_json(&mut server, "get_metadata", json!({}));
        assert_eq!(meta["format"], "pe32+", "{meta}");
        assert_eq!(meta["arch"], "x86-64", "{meta}");
        assert!(
            meta["entry_point"].as_str().unwrap().starts_with("0x"),
            "{meta}"
        );
        assert!(
            meta["sections"].as_array().unwrap().len() >= 2,
            "metadata must list the mapped sections: {meta}"
        );
        assert!(meta["counts"]["functions"].as_u64().unwrap() > 0, "{meta}");

        let exports = tool_json(&mut server, "list_exports", json!({}));
        assert!(exports["exports"].is_array(), "{exports}");
        assert_eq!(exports["count"], exports["total"], "{exports}");
        // notepad.exe exports nothing, but the shape must hold for a DLL too.
        assert_eq!(exports["truncated"], false, "{exports}");
    }

    #[test]
    fn prompts_render_with_project_facts() {
        let Some(mut server) = notepad_project() else {
            return;
        };
        let listed = call(&mut server, "prompts/list", json!({}));
        let names: Vec<&str> = listed["result"]["prompts"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|prompt| prompt["name"].as_str())
            .collect();
        for expected in [
            "triage_sample",
            "explain_function",
            "propose_names",
            "ctf_outline",
        ] {
            assert!(
                names.contains(&expected),
                "missing prompt {expected}: {names:?}"
            );
        }

        let triage = call(&mut server, "prompts/get", json!({"name": "triage_sample"}));
        let text = triage["result"]["messages"][0]["content"]["text"]
            .as_str()
            .unwrap();
        assert!(text.contains("Image: PE32+"), "{text}");
        assert!(text.contains("functions"), "{text}");
        assert!(text.contains("0x"), "{text}");

        let explain = call(
            &mut server,
            "prompts/get",
            json!({"name": "explain_function", "arguments": {"addr": "entry"}}),
        );
        assert!(
            explain["result"]["messages"][0]["content"]["text"]
                .as_str()
                .unwrap()
                .contains("entry"),
            "{explain}"
        );

        let missing = call(
            &mut server,
            "prompts/get",
            json!({"name": "explain_function"}),
        );
        assert_eq!(missing["error"]["code"], -32602, "{missing}");
        let unknown = call(&mut server, "prompts/get", json!({"name": "nope"}));
        assert_eq!(unknown["error"]["code"], -32602, "{unknown}");
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
            "define_struct",
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
    fn define_struct_then_apply_type_round_trip() {
        let Some((mut server, _project)) = notepad_writable() else {
            return;
        };
        let first = first_function(&mut server);
        let defined = tool_json(
            &mut server,
            "define_struct",
            json!({"name": "HDR", "size": 16, "fields": ["0:hdr:u32", "4:code:u32"]}),
        );
        assert_eq!(defined["tool"], "define_struct", "{defined}");
        assert_eq!(defined["name"], "HDR", "{defined}");
        assert!(defined["seq"].as_u64().unwrap() >= 1, "{defined}");

        // Defining a struct twice is a replace, not an error.
        let replaced = tool_json(
            &mut server,
            "define_struct",
            json!({"name": "HDR", "size": 32,
                   "fields": [{"offset": 0, "name": "hdr", "kind": "u32"}]}),
        );
        assert_eq!(replaced["name"], "HDR", "{replaced}");

        // The layout can now be bound to an argument slot; before this tool existed the
        // server could only apply structs the GUI had defined.
        let applied = tool_json(
            &mut server,
            "apply_type",
            json!({"func": first, "slot": 0, "struct": "HDR"}),
        );
        assert_eq!(applied["value"], "HDR", "{applied}");

        // Undo the binding, then the (re)definition.
        tool_json(&mut server, "undo", json!({}));
        let undone = tool_json(&mut server, "undo", json!({}));
        assert_eq!(undone["undone"], true, "{undone}");
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
                // Canonical batch-script grammar (same as `recraft-cli batch`).
                {"type": "rename", "addr": first, "name": "batch_one"},
                // The MCP-style pair is still accepted for convenience.
                {"tool": "rename", "arguments": {"addr": second, "name": "batch_two"}},
                {"type": "comment", "addr": first, "text": "batch note", "kind": "regular"},
            ]}),
        );
        assert_eq!(applied["count"], 3, "{applied}");
        assert!(applied["seq"].as_u64().unwrap() >= 3, "{applied}");
        assert_eq!(
            applied["commands"],
            json!(["rename", "rename", "comment"]),
            "{applied}"
        );

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
