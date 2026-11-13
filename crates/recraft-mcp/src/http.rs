// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! HTTP transport: stateful MCP sessions over `POST /mcp`.
//!
//! Each MCP session owns one [`McpServer`] (and therefore its own registry of open
//! projects). `initialize` creates a session and returns its id in the `Mcp-Session-Id`
//! response header; later requests carry that header back. A client that never sends the
//! header shares one default session, which keeps simple remote clients working.
//!
//! The transport stays local-first: bearer token, loopback `Origin`, protocol-version and
//! size checks are unchanged from the stateless version. Sessions are serialised by their
//! own mutex, so a long decompile in one session does not block another session or `ping`
//! on a different one.

use std::collections::HashMap;
use std::io::{BufRead, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use recraft_common::{Error, Result};
use serde_json::{Value, json};

use crate::{McpServer, SUPPORTED_PROTOCOL_VERSIONS, bearer_matches};

// ------------------------------------------------------------------ HTTP -----

/// Serve JSON-RPC over HTTP (`POST /mcp`) with bearer-token authentication (read-only).
///
/// Without `project` the server starts with no open project; the client calls
/// `open_program` to add one.
pub fn serve_http(project: Option<&Path>, bind: &str, token: &str) -> Result<()> {
    serve_http_mode(project, bind, token, false)
}

/// Same as [`serve_http`], with `--write` enabling the journaled write tools.
pub fn serve_http_writable(project: Option<&Path>, bind: &str, token: &str) -> Result<()> {
    serve_http_mode(project, bind, token, true)
}

fn serve_http_mode(project: Option<&Path>, bind: &str, token: &str, write: bool) -> Result<()> {
    let listener = TcpListener::bind(bind).map_err(Error::from)?;
    let local = listener.local_addr().map_err(Error::from)?;
    let mode = if write { "write" } else { "read-only" };
    eprintln!(
        "[mcp] http listening on http://{local}/mcp ({mode}, stateful sessions, bearer token required)"
    );
    serve_http_listener_mode(
        listener,
        project.map(Path::to_path_buf),
        token.to_owned(),
        write,
    )
}

/// Maximum number of requests handled at the same time; each one owns a `Session`.
const MAX_HTTP_CONNECTIONS: usize = 32;
static HTTP_CONNECTIONS: AtomicUsize = AtomicUsize::new(0);

/// Sessions idle for longer than this are dropped on the next request.
const SESSION_IDLE: Duration = Duration::from_secs(30 * 60);
/// Upper bound on live sessions, so a client cannot pin memory forever.
const MAX_HTTP_SESSIONS: usize = 16;
/// Session id used by clients that do not implement `Mcp-Session-Id`.
const DEFAULT_SESSION: &str = "default";

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

/// One MCP session: a server plus the registry/active instance it owns.
struct HttpSession {
    server: Mutex<McpServer>,
    last_used: Mutex<Instant>,
}

impl HttpSession {
    fn new(server: McpServer) -> Self {
        HttpSession {
            server: Mutex::new(server),
            last_used: Mutex::new(Instant::now()),
        }
    }

    fn touch(&self) {
        if let Ok(mut last_used) = self.last_used.lock() {
            *last_used = Instant::now();
        }
    }
}

/// Session table shared by every connection thread.
struct HttpState {
    sessions: Mutex<HashMap<String, Arc<HttpSession>>>,
    /// Project seeded into each new session; `None` means "start empty and let the client
    /// call `open_program`".
    seed: Option<PathBuf>,
    token: String,
    write: bool,
}

impl HttpState {
    fn new_server(&self) -> Result<McpServer> {
        match &self.seed {
            Some(path) => McpServer::open_http(path, self.write),
            None => Ok(McpServer::empty_http(self.write)),
        }
    }

    /// Drop sessions that have not been used recently.
    fn prune(&self) {
        let now = Instant::now();
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.retain(|_, session| {
                session
                    .last_used
                    .lock()
                    .map(|last_used| now.duration_since(*last_used) < SESSION_IDLE)
                    .unwrap_or(false)
            });
        }
    }

    fn get(&self, id: &str) -> Option<Arc<HttpSession>> {
        self.sessions
            .lock()
            .ok()
            .and_then(|sessions| sessions.get(id).cloned())
    }

    fn get_or_create(&self, id: &str) -> std::result::Result<Arc<HttpSession>, String> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "session registry poisoned".to_owned())?;
        if let Some(session) = sessions.get(id) {
            return Ok(Arc::clone(session));
        }
        if sessions.len() >= MAX_HTTP_SESSIONS {
            return Err("too many MCP sessions; retry after an idle session expires".to_owned());
        }
        let server = self
            .new_server()
            .map_err(|error| format!("cannot open project: {error}"))?;
        let session = Arc::new(HttpSession::new(server));
        sessions.insert(id.to_owned(), Arc::clone(&session));
        Ok(session)
    }

    fn create(&self, id: &str) -> std::result::Result<Arc<HttpSession>, String> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "session registry poisoned".to_owned())?;
        if !sessions.contains_key(id) && sessions.len() >= MAX_HTTP_SESSIONS {
            return Err("too many MCP sessions; retry after an idle session expires".to_owned());
        }
        let server = self
            .new_server()
            .map_err(|error| format!("cannot open project: {error}"))?;
        let session = Arc::new(HttpSession::new(server));
        sessions.insert(id.to_owned(), Arc::clone(&session));
        Ok(session)
    }

    fn remove(&self, id: &str) -> bool {
        self.sessions
            .lock()
            .map(|mut sessions| sessions.remove(id).is_some())
            .unwrap_or(false)
    }
}

pub(crate) fn serve_http_listener_mode(
    listener: TcpListener,
    project: Option<PathBuf>,
    token: String,
    write: bool,
) -> Result<()> {
    // Fail fast when a seeded project cannot be opened at all instead of answering 500 on
    // every initialize.
    if let Some(project) = &project {
        drop(McpServer::open_http(project, write)?);
    }
    let state = Arc::new(HttpState {
        sessions: Mutex::new(HashMap::new()),
        seed: project,
        token,
        write,
    });
    for stream in listener.incoming() {
        let Ok(stream) = stream else {
            continue;
        };
        let state = Arc::clone(&state);
        // One thread per connection; sessions serialise their own tool calls.
        std::thread::spawn(move || {
            let mut stream = stream;
            let _ = handle_http_connection(&mut stream, &state);
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
    session_id: Option<String>,
    body: String,
}

/// One HTTP response plus the session id to echo (only `initialize` needs it).
struct HttpResponse {
    status: u16,
    body: String,
    session_id: Option<String>,
}

impl HttpResponse {
    fn new(status: u16, body: impl Into<String>) -> Self {
        HttpResponse {
            status,
            body: body.into(),
            session_id: None,
        }
    }
}

fn handle_http_connection(stream: &mut TcpStream, state: &Arc<HttpState>) -> Result<()> {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    let Some(request) = read_http_request(stream)? else {
        return Ok(());
    };
    let response = if !origin_allowed(request.origin.as_deref()) {
        // Only a browser can be tricked into DNS rebinding, so a non-loopback Origin is
        // refused even when the token is right.
        eprintln!(
            "[mcp] rejected request with Origin {:?}",
            request.origin.as_deref().unwrap_or_default()
        );
        HttpResponse::new(403, json!({"error": "origin not allowed"}).to_string())
    } else if request.method == "GET" && request.path == "/health" {
        HttpResponse::new(200, r#"{"status":"ok"}"#)
    } else if request.path != "/mcp" {
        HttpResponse::new(404, r#"{"error":"not found"}"#)
    } else if request.method == "GET" {
        // No SSE stream in this transport; MCP clients fall back to POST responses.
        HttpResponse::new(405, r#"{"error":"method not allowed"}"#)
    } else if !bearer_matches(request.authorization.as_deref(), &state.token) {
        HttpResponse::new(401, r#"{"error":"missing or invalid bearer token"}"#)
    } else if let Some(version) = request
        .protocol_version
        .as_deref()
        .filter(|version| !SUPPORTED_PROTOCOL_VERSIONS.contains(version))
    {
        HttpResponse::new(
            400,
            json!({"error": format!("unsupported MCP-Protocol-Version: {version}")}).to_string(),
        )
    } else if request.method == "DELETE" {
        match request.session_id.as_deref() {
            Some(id) if state.remove(id) => HttpResponse::new(200, r#"{"closed":true}"#),
            Some(_) => HttpResponse::new(404, r#"{"error":"unknown session"}"#),
            None => HttpResponse::new(400, r#"{"error":"Mcp-Session-Id required"}"#),
        }
    } else if request.method != "POST" {
        HttpResponse::new(405, r#"{"error":"method not allowed"}"#)
    } else {
        match ConnectionSlot::acquire() {
            None => HttpResponse::new(503, r#"{"error":"too many concurrent requests"}"#),
            Some(slot) => {
                state.prune();
                let response = handle_rpc(state, request.session_id.as_deref(), &request.body);
                drop(slot);
                response
            }
        }
    };
    write_http_response(
        stream,
        response.status,
        &response.body,
        response.session_id.as_deref(),
    )
}

fn handle_rpc(state: &HttpState, requested_session: Option<&str>, body: &str) -> HttpResponse {
    let message: Value = match serde_json::from_str(body) {
        Ok(message) => message,
        Err(error) => {
            return HttpResponse::new(
                400,
                json!({"error": format!("parse error: {error}")}).to_string(),
            );
        }
    };
    let method = message
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if method == "initialize" {
        // Headerless clients all live in the default session; clients that understand
        // `Mcp-Session-Id` name their own. Re-initializing a session gives it a fresh
        // registry (and drops the old one when the last request finishes).
        let id = requested_session.unwrap_or(DEFAULT_SESSION).to_owned();
        let session = match state.create(&id) {
            Ok(session) => session,
            Err(error) => return HttpResponse::new(500, json!({"error": error}).to_string()),
        };
        let mut response = respond(&session, &message);
        response.session_id = Some(id);
        return response;
    }

    let session = match requested_session {
        Some(id) => match state.get(id) {
            Some(session) => session,
            None => return HttpResponse::new(404, r#"{"error":"unknown session"}"#),
        },
        // Compatibility path for clients that ignore `Mcp-Session-Id`: one shared server.
        None => match state.get_or_create(DEFAULT_SESSION) {
            Ok(session) => session,
            Err(error) => return HttpResponse::new(500, json!({"error": error}).to_string()),
        },
    };
    respond(&session, &message)
}

fn respond(session: &Arc<HttpSession>, message: &Value) -> HttpResponse {
    session.touch();
    let mut server = match session.server.lock() {
        Ok(server) => server,
        Err(_) => {
            return HttpResponse::new(500, r#"{"error":"MCP session state is unavailable"}"#);
        }
    };
    match server.handle_message(message) {
        Some(response) => HttpResponse::new(200, response.to_string()),
        None => HttpResponse::new(202, String::new()),
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
    let mut session_id = None;
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
        } else if name == "mcp-session-id" {
            session_id = Some(value.to_owned());
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
        session_id,
        body: String::from_utf8_lossy(&body).into_owned(),
    }))
}

fn write_http_response(
    stream: &mut TcpStream,
    status: u16,
    body: &str,
    session_id: Option<&str>,
) -> Result<()> {
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
    let session = session_id
        .map(|id| format!("Mcp-Session-Id: {id}\r\n"))
        .unwrap_or_default();
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\n{session}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).map_err(Error::from)?;
    stream.flush().map_err(Error::from)?;
    Ok(())
}
