// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Minimal HTTP transport for LLM providers.
//!
//! DESIGN §9.3: the default build never talks to the network. This module only runs
//! when the user explicitly enables a provider **and** the request is made, and it is
//! split behind [`Transport`] so providers are testable without sockets.
//!
//! - `TcpHttpTransport` speaks plain `http://` (local Ollama at `http://127.0.0.1:11434`).
//! - `UreqTransport` speaks `https://` via rustls (`ureq`), for OpenAI-compatible
//!   endpoints such as `https://api.openai.com`.
//! - [`transport_for`] picks the right one from the endpoint URL.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use recraft_common::{Error, Result};
use serde_json::Value;

/// One HTTP request as assembled by a provider.
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: &'static str,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

/// One HTTP response.
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
}

/// Blocking HTTP transport (injectable for tests).
pub trait Transport {
    fn send(&mut self, request: &HttpRequest) -> Result<HttpResponse>;

    /// Send a request and forward the response body as it arrives. The default keeps
    /// non-streaming transports working by delivering the whole body once.
    fn send_stream(
        &mut self,
        request: &HttpRequest,
        on_chunk: &mut dyn FnMut(&str),
    ) -> Result<HttpResponse> {
        let response = self.send(request)?;
        if response.status == 200 {
            on_chunk(&response.body);
        }
        Ok(HttpResponse {
            status: response.status,
            body: String::new(),
        })
    }
}

/// Which transport serves an endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportKind {
    Tcp,
    Tls,
}

/// Pick a transport for a provider endpoint (`https://` uses rustls, everything else
/// uses the plain TCP transport).
pub fn transport_kind(url: &str) -> TransportKind {
    if url
        .trim_start()
        .to_ascii_lowercase()
        .starts_with("https://")
    {
        TransportKind::Tls
    } else {
        TransportKind::Tcp
    }
}

/// Boxed transport for an endpoint URL.
pub fn transport_for(url: &str) -> Box<dyn Transport> {
    match transport_kind(url) {
        TransportKind::Tls => Box::new(UreqTransport),
        TransportKind::Tcp => Box::new(TcpHttpTransport),
    }
}

/// HTTPS transport backed by `ureq`/rustls.
#[derive(Debug, Default)]
pub struct UreqTransport;

impl Transport for UreqTransport {
    fn send(&mut self, request: &HttpRequest) -> Result<HttpResponse> {
        if !matches!(transport_kind(&request.url), TransportKind::Tls) {
            return Err(Error::unsupported(
                "UreqTransport only handles https:// endpoints",
            ));
        }
        if request.method != "POST" {
            return Err(Error::unsupported(
                "provider transport only supports POST requests",
            ));
        }
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .build()
            .into();
        let mut builder = agent.post(request.url.as_str());
        for (name, value) in &request.headers {
            builder = builder.header(name.as_str(), value.as_str());
        }
        let mut response = builder
            .send(request.body.as_str())
            .map_err(|error| Error::internal(format!("https request failed: {error}")))?;
        let status = response.status().as_u16();
        let body = response
            .body_mut()
            .read_to_string()
            .map_err(|error| Error::internal(format!("https response body: {error}")))?;
        Ok(HttpResponse { status, body })
    }

    fn send_stream(
        &mut self,
        request: &HttpRequest,
        on_chunk: &mut dyn FnMut(&str),
    ) -> Result<HttpResponse> {
        if !matches!(transport_kind(&request.url), TransportKind::Tls) {
            return Err(Error::unsupported(
                "UreqTransport only handles https:// endpoints",
            ));
        }
        if request.method != "POST" {
            return Err(Error::unsupported(
                "provider transport only supports POST requests",
            ));
        }
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .build()
            .into();
        let mut builder = agent.post(request.url.as_str());
        for (name, value) in &request.headers {
            builder = builder.header(name.as_str(), value.as_str());
        }
        let mut response = builder
            .send(request.body.as_str())
            .map_err(|error| Error::internal(format!("https request failed: {error}")))?;
        let status = response.status().as_u16();
        if status != 200 {
            let body = response
                .body_mut()
                .read_to_string()
                .map_err(|error| Error::internal(format!("https response body: {error}")))?;
            return Ok(HttpResponse { status, body });
        }
        let mut reader = response.body_mut().as_reader();
        let mut buffer = [0u8; 4096];
        let mut pending = Vec::new();
        loop {
            let read = reader
                .read(&mut buffer)
                .map_err(|error| Error::internal(format!("https response stream: {error}")))?;
            if read == 0 {
                break;
            }
            pending.extend_from_slice(&buffer[..read]);
            // A chunk may end in the middle of a multi-byte character; only forward
            // the valid UTF-8 prefix and keep the incomplete suffix for the next read.
            match std::str::from_utf8(&pending) {
                Ok(text) => {
                    on_chunk(text);
                    pending.clear();
                }
                Err(error) => {
                    let valid = error.valid_up_to();
                    if valid > 0 {
                        let text = String::from_utf8_lossy(&pending[..valid]).into_owned();
                        on_chunk(&text);
                        pending.drain(..valid);
                    }
                }
            }
        }
        if !pending.is_empty() {
            on_chunk(&String::from_utf8_lossy(&pending));
        }
        Ok(HttpResponse {
            status,
            body: String::new(),
        })
    }
}

/// Plain-HTTP transport over `std::net::TcpStream`.
#[derive(Debug, Default)]
pub struct TcpHttpTransport;

impl Transport for TcpHttpTransport {
    fn send(&mut self, request: &HttpRequest) -> Result<HttpResponse> {
        let Some(rest) = request.url.strip_prefix("http://") else {
            return Err(Error::unsupported(
                "only plain http:// LLM endpoints are supported (local Ollama); https needs a TLS stack",
            ));
        };
        let (authority, path) = match rest.split_once('/') {
            Some((authority, path)) => (authority, format!("/{path}")),
            None => (rest, "/".to_owned()),
        };
        let (host, port) = match authority.split_once(':') {
            Some((host, port)) => (host, port.parse::<u16>().unwrap_or(80)),
            None => (authority, 80),
        };
        let mut stream = TcpStream::connect((host, port)).map_err(Error::from)?;
        stream
            .set_read_timeout(Some(Duration::from_secs(120)))
            .map_err(Error::from)?;
        stream
            .set_write_timeout(Some(Duration::from_secs(30)))
            .map_err(Error::from)?;

        let mut request_text = format!(
            "{} {path} HTTP/1.1\r\nHost: {authority}\r\n",
            request.method
        );
        for (name, value) in &request.headers {
            request_text.push_str(&format!("{name}: {value}\r\n"));
        }
        request_text.push_str(&format!("Content-Length: {}\r\n", request.body.len()));
        request_text.push_str("Connection: close\r\n\r\n");
        request_text.push_str(&request.body);
        stream
            .write_all(request_text.as_bytes())
            .map_err(Error::from)?;
        stream.flush().map_err(Error::from)?;

        let mut bytes = Vec::new();
        stream.read_to_end(&mut bytes).map_err(Error::from)?;
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let Some((head, body)) = text.split_once("\r\n\r\n") else {
            return Err(Error::internal("malformed HTTP response"));
        };
        let status = head
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|code| code.parse::<u16>().ok())
            .ok_or_else(|| Error::internal("malformed HTTP status line"))?;
        let chunked = head
            .to_ascii_lowercase()
            .contains("transfer-encoding: chunked");
        let body = if chunked {
            decode_chunked(body)?
        } else {
            body.to_owned()
        };
        Ok(HttpResponse { status, body })
    }
}

fn decode_chunked(body: &str) -> Result<String> {
    let mut out = String::new();
    let mut rest = body;
    while let Some((size_line, tail)) = rest.split_once("\r\n") {
        let size = usize::from_str_radix(size_line.trim(), 16)
            .map_err(|_| Error::internal("malformed chunk size"))?;
        if size == 0 {
            break;
        }
        if tail.len() < size {
            return Err(Error::internal("truncated chunked response"));
        }
        out.push_str(&tail[..size]);
        rest = tail[size..].trim_start_matches("\r\n");
    }
    Ok(out)
}

/// Extract `message.content` from an Ollama `/api/chat` response.
pub fn parse_ollama_response(body: &str) -> Result<String> {
    let value: Value = serde_json::from_str(body)
        .map_err(|error| Error::internal(format!("ollama JSON: {error}")))?;
    value
        .get("message")
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| Error::internal("ollama response has no message.content"))
}

/// Extract `choices[0].message.content` from an OpenAI-compatible response.
pub fn parse_openai_response(body: &str) -> Result<String> {
    let value: Value = serde_json::from_str(body)
        .map_err(|error| Error::internal(format!("openai JSON: {error}")))?;
    value
        .get("choices")
        .and_then(|choices| choices.get(0))
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| Error::internal("openai response has no choices[0].message.content"))
}

/// Test-only transport that records requests and replays scripted responses.
#[derive(Debug, Default, Clone)]
pub struct MockTransport {
    /// Shared request log (clone a handle before boxing the transport).
    requests: std::sync::Arc<std::sync::Mutex<Vec<HttpRequest>>>,
    responses: std::collections::VecDeque<HttpResponse>,
}

impl MockTransport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, status: u16, body: &str) {
        self.responses.push_back(HttpResponse {
            status,
            body: body.to_owned(),
        });
    }

    pub fn requests_handle(&self) -> std::sync::Arc<std::sync::Mutex<Vec<HttpRequest>>> {
        std::sync::Arc::clone(&self.requests)
    }
}

impl Transport for MockTransport {
    fn send(&mut self, request: &HttpRequest) -> Result<HttpResponse> {
        if let Ok(mut requests) = self.requests.lock() {
            requests.push(request.clone());
        }
        self.responses
            .pop_front()
            .ok_or_else(|| Error::internal("mock transport exhausted"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn parses_provider_response_shapes() {
        let ollama = r#"{"message":{"role":"assistant","content":"hello"}}"#;
        assert_eq!(parse_ollama_response(ollama).unwrap(), "hello");
        let openai = r#"{"choices":[{"message":{"content":"world"}}]}"#;
        assert_eq!(parse_openai_response(openai).unwrap(), "world");
        assert!(parse_openai_response("{}").is_err());
    }

    #[test]
    fn picks_transport_by_scheme() {
        assert_eq!(
            transport_kind("https://api.openai.com/v1/chat/completions"),
            TransportKind::Tls
        );
        assert_eq!(
            transport_kind("http://127.0.0.1:11434/api/chat"),
            TransportKind::Tcp
        );
        assert_eq!(transport_kind("HTTPS://example.com"), TransportKind::Tls);
    }

    #[test]
    fn plain_transport_rejects_https() {
        let mut transport = TcpHttpTransport;
        let error = transport
            .send(&HttpRequest {
                method: "POST",
                url: "https://api.example.com/v1/chat/completions".to_owned(),
                headers: Vec::new(),
                body: "{}".to_owned(),
            })
            .unwrap_err();
        assert!(error.to_string().contains("only plain http"));
    }

    #[test]
    fn tls_transport_rejects_plain_http() {
        let mut transport = UreqTransport;
        let error = transport
            .send(&HttpRequest {
                method: "POST",
                url: "http://127.0.0.1:11434/api/chat".to_owned(),
                headers: Vec::new(),
                body: "{}".to_owned(),
            })
            .unwrap_err();
        assert!(error.to_string().contains("only handles https"));
    }

    #[test]
    #[ignore = "network access: verifies the TLS handshake against crates.io"]
    fn https_tls_handshake_to_crates_io() {
        let mut transport = UreqTransport;
        let result = transport.send(&HttpRequest {
            method: "POST",
            url: "https://crates.io/".to_owned(),
            headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
            body: "{}".to_owned(),
        });
        match result {
            // Any HTTP status proves the TLS handshake completed; POST / is not a
            // supported route, so an API error is expected.
            Ok(response) => assert!(response.status >= 200),
            Err(error) => {
                let text = error.to_string();
                assert!(
                    text.to_ascii_lowercase().contains("status"),
                    "TLS/connect failure: {text}"
                );
            }
        }
    }

    #[test]
    fn talks_http_to_a_local_server() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let address = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buffer = [0u8; 4096];
            let _ = stream.read(&mut buffer);
            let body = r#"{"message":{"content":"local"}}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(response.as_bytes());
        });
        let mut transport = TcpHttpTransport;
        let response = transport
            .send(&HttpRequest {
                method: "POST",
                url: format!("http://{address}/api/chat"),
                headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
                body: "{}".to_owned(),
            })
            .expect("response");
        handle.join().unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(parse_ollama_response(&response.body).unwrap(), "local");
    }
}
