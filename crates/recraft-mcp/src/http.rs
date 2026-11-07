// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! HTTP transport: one thread per connection, bearer token, Origin and protocol-version
//! checks, per-request `Session`.

use std::io::{BufRead, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use recraft_common::{Error, Result};
use serde_json::{Value, json};

use crate::{McpServer, SUPPORTED_PROTOCOL_VERSIONS, bearer_matches};

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
    drop(McpServer::open_http(&project, write)?);
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
    let mut server = match McpServer::open_http(project, write) {
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
