// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! LLM provider abstraction + prompt safety layer (DESIGN §9.3).
//!
//! Nothing here performs network I/O: this milestone makes the *contract* explicit and
//! testable so an HTTP transport can be added without touching the call sites.
//!
//! - [`ProviderSettings`] is disabled by default (heuristic/offline). Enabling an HTTP
//!   provider requires `RECRAFT_AI_PROVIDER` **and** an API key in the configured env
//!   var; otherwise the settings report `enabled = false`.
//! - [`build_explain_prompt`] / [`build_rename_prompt`] assemble minimal context
//!   (function metadata, API tags, referenced strings, pseudo-C), run [`redact`], and
//!   fit the result into a [`TokenBudget`].
//! - [`PromptCache`] and [`AuditLog`] are offline-safe bookkeeping structures used by
//!   the future transport.

use std::collections::BTreeMap;

use recraft_common::{Address, Error, Result};
use recraft_core::Session;

/// Which provider backend a session is configured to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    /// Deterministic heuristics only; never sends data anywhere.
    Heuristic,
    /// OpenAI-compatible HTTP endpoint (transport not implemented yet).
    OpenAiCompatible,
    /// Local Ollama HTTP endpoint (transport not implemented yet).
    Ollama,
}

impl ProviderKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            ProviderKind::Heuristic => "heuristic",
            ProviderKind::OpenAiCompatible => "openai",
            ProviderKind::Ollama => "ollama",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text.to_ascii_lowercase().as_str() {
            "heuristic" | "off" | "none" => Some(ProviderKind::Heuristic),
            "openai" | "openai-compatible" => Some(ProviderKind::OpenAiCompatible),
            "ollama" => Some(ProviderKind::Ollama),
            _ => None,
        }
    }
}

/// Provider configuration. Defaults are offline: heuristic, disabled, empty endpoint.
#[derive(Debug, Clone)]
pub struct ProviderSettings {
    pub kind: ProviderKind,
    pub endpoint: String,
    pub model: String,
    /// Name of the environment variable that holds the API key (never the key itself).
    pub api_key_env: String,
    pub max_tokens: usize,
    /// True only when a non-heuristic provider has an endpoint and its key env var is set.
    pub enabled: bool,
}

impl Default for ProviderSettings {
    fn default() -> Self {
        ProviderSettings {
            kind: ProviderKind::Heuristic,
            endpoint: String::new(),
            model: "gpt-4o-mini".to_owned(),
            api_key_env: "RECRAFT_AI_API_KEY".to_owned(),
            max_tokens: 1024,
            enabled: false,
        }
    }
}

impl ProviderSettings {
    /// Build settings from a lookup function (testable without touching the process env).
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Self {
        let mut settings = ProviderSettings::default();
        if let Some(kind) =
            lookup("RECRAFT_AI_PROVIDER").and_then(|value| ProviderKind::parse(&value))
        {
            settings.kind = kind;
        }
        if let Some(endpoint) = lookup("RECRAFT_AI_ENDPOINT") {
            settings.endpoint = endpoint;
        }
        if let Some(model) = lookup("RECRAFT_AI_MODEL") {
            settings.model = model;
        }
        if let Some(api_key_env) = lookup("RECRAFT_AI_API_KEY_ENV") {
            settings.api_key_env = api_key_env;
        }
        if let Some(max_tokens) =
            lookup("RECRAFT_AI_MAX_TOKENS").and_then(|value| value.parse().ok())
        {
            settings.max_tokens = max_tokens;
        }
        settings.enabled = settings.kind != ProviderKind::Heuristic
            && !settings.endpoint.is_empty()
            && lookup(&settings.api_key_env).is_some_and(|key| !key.is_empty());
        settings
    }

    /// Read the real process environment (defaults stay offline unless explicitly set).
    pub fn from_env() -> Self {
        ProviderSettings::from_lookup(|name| std::env::var(name).ok())
    }

    /// One-line description safe to show in UI/CLI (never prints a key).
    pub fn describe(&self) -> String {
        let state = if self.enabled { "enabled" } else { "disabled" };
        format!(
            "provider={} state={state} model={} endpoint={} key_env={} max_tokens={}",
            self.kind.as_str(),
            self.model,
            if self.endpoint.is_empty() {
                "<none>"
            } else {
                &self.endpoint
            },
            self.api_key_env,
            self.max_tokens
        )
    }
}

/// Token budget applied before any context leaves the process.
#[derive(Debug, Clone, Copy)]
pub struct TokenBudget {
    pub max_tokens: usize,
}

impl TokenBudget {
    pub fn new(max_tokens: usize) -> Self {
        TokenBudget {
            max_tokens: max_tokens.max(64),
        }
    }

    /// Rough estimate: ~4 characters per token, rounded up.
    pub fn estimate(text: &str) -> usize {
        text.chars().count().div_ceil(4)
    }

    /// Truncate `text` so its estimate fits the budget.
    pub fn fit(&self, text: &str) -> String {
        if Self::estimate(text) <= self.max_tokens {
            return text.to_owned();
        }
        let keep = self.max_tokens.saturating_mul(4);
        let mut fitted: String = text.chars().take(keep).collect();
        fitted.push_str("\n…[truncated]");
        fitted
    }
}

/// Prompt + minimal context sent to a provider.
#[derive(Debug, Clone)]
pub struct PromptRequest {
    pub system: String,
    pub user: String,
    pub estimated_tokens: usize,
}

impl PromptRequest {
    pub fn new(budget: TokenBudget, system: &str, user: &str) -> Self {
        let system = budget.fit(system);
        let user = budget.fit(user);
        let estimated_tokens = TokenBudget::estimate(&system) + TokenBudget::estimate(&user);
        PromptRequest {
            system,
            user,
            estimated_tokens,
        }
    }
}

const EXPLAIN_SYSTEM: &str = "You are ReCraft's reverse-engineering assistant. \
Explain what the given function does in plain language. Use only the supplied evidence \
(API calls, strings, pseudo-code); never invent behaviour. Mention uncertainty explicitly.";

const RENAME_SYSTEM: &str = "You are ReCraft's symbol-naming assistant. Propose concise \
snake_case names for the listed functions. Base every suggestion on the supplied API \
calls, strings and pseudo-code, and keep the evidence addresses in your rationale.";

/// Redact URLs, IPv4 addresses and e-mail addresses before context leaves the process.
/// Line structure is preserved so the prompt stays readable for review.
pub fn redact(text: &str) -> String {
    text.lines()
        .map(|line| {
            line.split_whitespace()
                .map(redact_word)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn redact_word(word: &str) -> String {
    let lower = word.to_ascii_lowercase();
    if lower.contains("http://") || lower.contains("https://") {
        return "<url>".to_owned();
    }
    if let Some(at) = word.find('@')
        && at > 0
        && word[at + 1..].contains('.')
        && !word[at + 1..].contains('\\')
    {
        return "<email>".to_owned();
    }
    // IPv4: four dot-separated decimal groups, ignoring trailing punctuation.
    let cleaned = word.trim_matches(|ch: char| !ch.is_ascii_digit() && ch != '.');
    let parts: Vec<&str> = cleaned.split('.').collect();
    if parts.len() == 4
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit()))
    {
        return "<ip>".to_owned();
    }
    word.to_owned()
}

/// Build the explanation prompt for one function.
pub fn build_explain_prompt(
    session: &mut Session,
    addr: Address,
    settings: &ProviderSettings,
) -> Result<PromptRequest> {
    let explanation = crate::explain(session, addr)?;
    let budget = TokenBudget::new(settings.max_tokens);
    let mut user = String::new();
    user.push_str(&format!("function: {}\n", explanation.summary));
    user.push_str("api calls:\n");
    for (site, rule, category) in explanation.apis.iter().take(24) {
        user.push_str(&format!("- {:#x} {rule} [{category}]\n", site.get()));
    }
    if !explanation.strings.is_empty() {
        user.push_str("strings:\n");
        for (site, text) in explanation.strings.iter().take(16) {
            user.push_str(&format!("- {:#x} {text}\n", site.get()));
        }
    }
    let pseudo = recraft_decompile::pseudocode_l2(session, explanation.start)?;
    user.push_str("pseudo-code:\n");
    for line in pseudo.lines.iter().take(80) {
        user.push_str(&line.text);
        user.push('\n');
    }
    let user = redact(&user);
    Ok(PromptRequest::new(budget, EXPLAIN_SYSTEM, &user))
}

/// Build a naming prompt for up to `limit` candidate functions (typically `sub_*`).
pub fn build_rename_prompt(
    session: &mut Session,
    candidates: &[Address],
    settings: &ProviderSettings,
) -> Result<PromptRequest> {
    let budget = TokenBudget::new(settings.max_tokens);
    let mut user = String::from("propose names for these functions:\n");
    for addr in candidates.iter().take(12) {
        let explanation = crate::explain(session, *addr)?;
        user.push_str(&format!(
            "\n- {:#x}: {}\n",
            explanation.start.get(),
            redact(&explanation.summary)
        ));
        for (site, rule, _) in explanation.apis.iter().take(8) {
            user.push_str(&format!("  api {:#x} {rule}\n", site.get()));
        }
    }
    Ok(PromptRequest::new(budget, RENAME_SYSTEM, &user))
}

/// In-memory prompt → response cache keyed by a stable hash of the request.
#[derive(Debug, Default)]
pub struct PromptCache {
    entries: BTreeMap<u64, String>,
    hits: usize,
    misses: usize,
}

impl PromptCache {
    pub fn new() -> Self {
        Self::default()
    }

    fn key(request: &PromptRequest) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        request.system.hash(&mut hasher);
        request.user.hash(&mut hasher);
        hasher.finish()
    }

    pub fn get(&mut self, request: &PromptRequest) -> Option<&str> {
        let key = Self::key(request);
        match self.entries.get(&key) {
            Some(value) => {
                self.hits += 1;
                Some(value.as_str())
            }
            None => {
                self.misses += 1;
                None
            }
        }
    }

    pub fn insert(&mut self, request: &PromptRequest, response: &str) {
        self.entries.insert(Self::key(request), response.to_owned());
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn stats(&self) -> (usize, usize) {
        (self.hits, self.misses)
    }
}

/// One AI interaction recorded for audit (who asked, which provider, how many tokens).
#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub provider: &'static str,
    pub function: Option<Address>,
    pub prompt_tokens: usize,
    pub response_tokens: usize,
    pub cached: bool,
}

/// Append-only in-memory audit log (DESIGN §9.3 requires traceability).
#[derive(Debug, Default)]
pub struct AuditLog {
    entries: Vec<AuditEntry>,
}

impl AuditLog {
    pub fn record(&mut self, entry: AuditEntry) {
        self.entries.push(entry);
    }

    pub fn entries(&self) -> &[AuditEntry] {
        &self.entries
    }

    pub fn total_prompt_tokens(&self) -> usize {
        self.entries.iter().map(|entry| entry.prompt_tokens).sum()
    }
}

/// The offline-safe provider trait a future HTTP transport implements.
pub trait Provider {
    fn name(&self) -> &'static str;
    fn complete(&mut self, request: &PromptRequest) -> Result<String>;
}

/// Local Ollama (`POST /api/chat`, `stream: false`).
pub struct OllamaProvider {
    settings: ProviderSettings,
    transport: Box<dyn crate::transport::Transport>,
}

impl OllamaProvider {
    pub fn new(
        settings: ProviderSettings,
        transport: Box<dyn crate::transport::Transport>,
    ) -> Self {
        OllamaProvider {
            settings,
            transport,
        }
    }
}

impl Provider for OllamaProvider {
    fn name(&self) -> &'static str {
        "ollama"
    }

    fn complete(&mut self, request: &PromptRequest) -> Result<String> {
        let url = format!("{}/api/chat", self.settings.endpoint.trim_end_matches('/'));
        let body = serde_json::json!({
            "model": self.settings.model,
            "stream": false,
            "options": {"num_predict": self.settings.max_tokens},
            "messages": [
                {"role": "system", "content": request.system},
                {"role": "user", "content": request.user},
            ],
        })
        .to_string();
        let response = self.transport.send(&crate::transport::HttpRequest {
            method: "POST",
            url,
            headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
            body,
        })?;
        if response.status != 200 {
            return Err(Error::internal(format!(
                "ollama HTTP {}: {}",
                response.status,
                response.body.chars().take(400).collect::<String>()
            )));
        }
        crate::transport::parse_ollama_response(&response.body)
    }
}

/// OpenAI-compatible chat completions (`POST {endpoint}/v1/chat/completions`).
pub struct OpenAiCompatibleProvider {
    settings: ProviderSettings,
    transport: Box<dyn crate::transport::Transport>,
    api_key: Option<String>,
}

impl OpenAiCompatibleProvider {
    pub fn new(
        settings: ProviderSettings,
        transport: Box<dyn crate::transport::Transport>,
        api_key: Option<String>,
    ) -> Self {
        OpenAiCompatibleProvider {
            settings,
            transport,
            api_key,
        }
    }
}

impl Provider for OpenAiCompatibleProvider {
    fn name(&self) -> &'static str {
        "openai-compatible"
    }

    fn complete(&mut self, request: &PromptRequest) -> Result<String> {
        let url = format!(
            "{}/v1/chat/completions",
            self.settings.endpoint.trim_end_matches('/')
        );
        let body = serde_json::json!({
            "model": self.settings.model,
            "max_tokens": self.settings.max_tokens,
            "messages": [
                {"role": "system", "content": request.system},
                {"role": "user", "content": request.user},
            ],
        })
        .to_string();
        let mut headers = vec![("Content-Type".to_owned(), "application/json".to_owned())];
        if let Some(key) = &self.api_key {
            headers.push(("Authorization".to_owned(), format!("Bearer {key}")));
        }
        let response = self.transport.send(&crate::transport::HttpRequest {
            method: "POST",
            url,
            headers,
            body,
        })?;
        if response.status != 200 {
            return Err(Error::internal(format!(
                "openai HTTP {}: {}",
                response.status,
                response.body.chars().take(400).collect::<String>()
            )));
        }
        crate::transport::parse_openai_response(&response.body)
    }
}

/// Build the provider selected by `settings`; disabled settings stay offline.
pub fn provider_from_settings(settings: &ProviderSettings) -> Box<dyn Provider> {
    if !settings.enabled || settings.kind == ProviderKind::Heuristic {
        return Box::new(HeuristicProvider);
    }
    match settings.kind {
        ProviderKind::Ollama => Box::new(OllamaProvider::new(
            settings.clone(),
            Box::new(crate::transport::TcpHttpTransport),
        )),
        ProviderKind::OpenAiCompatible => {
            let api_key = std::env::var(&settings.api_key_env).ok();
            Box::new(OpenAiCompatibleProvider::new(
                settings.clone(),
                Box::new(crate::transport::TcpHttpTransport),
                api_key,
            ))
        }
        ProviderKind::Heuristic => Box::new(HeuristicProvider),
    }
}

/// Default provider: no network. It refuses to pretend to be a model.
#[derive(Debug, Default)]
pub struct HeuristicProvider;

impl Provider for HeuristicProvider {
    fn name(&self) -> &'static str {
        "heuristic"
    }

    fn complete(&mut self, _request: &PromptRequest) -> Result<String> {
        Err(Error::unsupported(
            "no LLM provider configured; set RECRAFT_AI_PROVIDER/RECRAFT_AI_ENDPOINT and the API key env var",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_are_offline() {
        let settings = ProviderSettings::default();
        assert_eq!(settings.kind, ProviderKind::Heuristic);
        assert!(!settings.enabled);
        assert!(settings.describe().contains("state=disabled"));
    }

    #[test]
    fn http_provider_requires_endpoint_and_key() {
        let settings = ProviderSettings::from_lookup(|name| match name {
            "RECRAFT_AI_PROVIDER" => Some("openai".to_owned()),
            "RECRAFT_AI_ENDPOINT" => Some("http://localhost:11434".to_owned()),
            _ => None,
        });
        assert!(!settings.enabled, "key env missing -> disabled");

        let settings = ProviderSettings::from_lookup(|name| match name {
            "RECRAFT_AI_PROVIDER" => Some("ollama".to_owned()),
            "RECRAFT_AI_ENDPOINT" => Some("http://localhost:11434".to_owned()),
            "RECRAFT_AI_MODEL" => Some("qwen2.5-coder".to_owned()),
            "RECRAFT_AI_API_KEY" => Some("secret".to_owned()),
            _ => None,
        });
        assert!(settings.enabled);
        assert!(settings.describe().contains("ollama"));
        assert!(!settings.describe().contains("secret"));
    }

    #[test]
    fn redacts_urls_ips_and_emails() {
        let text = "GET https://evil.example/a?b=1 from 10.0.0.5 mail admin@example.com";
        let redacted = redact(text);
        assert!(redacted.contains("<url>"));
        assert!(redacted.contains("<ip>"));
        assert!(redacted.contains("<email>"));
        assert!(!redacted.contains("evil.example"));
        assert!(!redacted.contains("10.0.0.5"));
    }

    #[test]
    fn budget_truncates_long_prompts() {
        let budget = TokenBudget::new(32);
        let long = "a".repeat(10_000);
        let fitted = budget.fit(&long);
        assert!(TokenBudget::estimate(&fitted) <= budget.max_tokens + 8);
        assert!(fitted.ends_with("…[truncated]"));
    }

    #[test]
    fn cache_and_audit_bookkeeping() {
        let budget = TokenBudget::new(128);
        let request = PromptRequest::new(budget, "system", "user");
        let mut cache = PromptCache::new();
        assert!(cache.get(&request).is_none());
        cache.insert(&request, "answer");
        assert_eq!(cache.get(&request), Some("answer"));
        assert_eq!(cache.stats(), (1, 1));

        let mut audit = AuditLog::default();
        audit.record(AuditEntry {
            provider: "heuristic",
            function: Some(Address::new(0x1000)),
            prompt_tokens: request.estimated_tokens,
            response_tokens: 2,
            cached: true,
        });
        assert_eq!(audit.entries().len(), 1);
        assert!(audit.total_prompt_tokens() > 0);
    }

    #[test]
    fn heuristic_provider_never_pretends_to_be_a_model() {
        let request = PromptRequest::new(TokenBudget::new(64), "system", "user");
        let error = HeuristicProvider.complete(&request).unwrap_err();
        assert!(error.to_string().contains("no LLM provider configured"));
    }

    fn test_settings(kind: ProviderKind, endpoint: &str) -> ProviderSettings {
        ProviderSettings {
            kind,
            endpoint: endpoint.to_owned(),
            model: "test-model".to_owned(),
            api_key_env: "RECRAFT_AI_API_KEY".to_owned(),
            max_tokens: 256,
            enabled: true,
        }
    }

    #[test]
    fn ollama_provider_posts_chat_request_and_parses_reply() {
        let mut transport = crate::transport::MockTransport::new();
        let requests = transport.requests_handle();
        transport.push(200, r#"{"message":{"content":"explained"}}"#);
        let settings = test_settings(ProviderKind::Ollama, "http://127.0.0.1:11434");
        let mut provider = OllamaProvider::new(settings, Box::new(transport));
        let request = PromptRequest::new(TokenBudget::new(256), "sys", "user");
        assert_eq!(provider.complete(&request).unwrap(), "explained");

        let recorded = requests.lock().unwrap();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].url, "http://127.0.0.1:11434/api/chat");
        let body: serde_json::Value = serde_json::from_str(&recorded[0].body).unwrap();
        assert_eq!(body["model"], "test-model");
        assert_eq!(body["stream"], false);
        assert_eq!(body["messages"][0]["content"], "sys");
    }

    #[test]
    fn openai_provider_sends_key_and_parses_reply() {
        let mut transport = crate::transport::MockTransport::new();
        let requests = transport.requests_handle();
        transport.push(200, r#"{"choices":[{"message":{"content":"hello"}}]}"#);
        let settings = test_settings(
            ProviderKind::OpenAiCompatible,
            "http://127.0.0.1:8080/proxy",
        );
        let mut provider = OpenAiCompatibleProvider::new(
            settings,
            Box::new(transport),
            Some("sk-test".to_owned()),
        );
        let request = PromptRequest::new(TokenBudget::new(128), "system", "user");
        assert_eq!(provider.complete(&request).unwrap(), "hello");

        let recorded = requests.lock().unwrap();
        assert_eq!(
            recorded[0].url,
            "http://127.0.0.1:8080/proxy/v1/chat/completions"
        );
        assert!(
            recorded[0]
                .headers
                .iter()
                .any(|(name, value)| name == "Authorization" && value == "Bearer sk-test")
        );
    }

    #[test]
    fn http_error_status_is_reported() {
        let mut transport = crate::transport::MockTransport::new();
        transport.push(500, "boom");
        let settings = test_settings(ProviderKind::Ollama, "http://127.0.0.1:11434");
        let mut provider = OllamaProvider::new(settings, Box::new(transport));
        let request = PromptRequest::new(TokenBudget::new(64), "sys", "user");
        let error = provider.complete(&request).unwrap_err();
        assert!(error.to_string().contains("HTTP 500"));
    }

    #[test]
    fn disabled_settings_select_the_heuristic_provider() {
        let provider = provider_from_settings(&ProviderSettings::default());
        assert_eq!(provider.name(), "heuristic");
    }
}
