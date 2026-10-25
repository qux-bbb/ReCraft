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
use std::path::{Path, PathBuf};

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

/// Built-in OpenAI-compatible providers offered by the desktop settings UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderPreset {
    DeepSeek,
    OpenCodeGo,
}

/// DeepSeek official API endpoint (the provider appends `/v1/chat/completions`).
pub const DEEPSEEK_ENDPOINT: &str = "https://api.deepseek.com";
/// OpenCode Go OpenAI-compatible endpoint.
pub const OPENCODE_GO_ENDPOINT: &str = "https://opencode.ai/inference/go/openai";
/// Common DeepSeek model ids.
pub const DEEPSEEK_MODELS: &[&str] = &["deepseek-chat", "deepseek-reasoner"];
/// A subset of OpenCode Go model ids that are useful for code reasoning.
pub const OPENCODE_GO_MODELS: &[&str] = &[
    "deepseek-v4.1-flash",
    "deepseek-v4-pro",
    "kimi-k3",
    "glm-5.3-flash",
    "qwen3.8-flash",
    "minimax-m3.1-flash",
];

impl ProviderPreset {
    pub const fn label(self) -> &'static str {
        match self {
            ProviderPreset::DeepSeek => "DeepSeek",
            ProviderPreset::OpenCodeGo => "OpenCode Go",
        }
    }

    pub const fn endpoint(self) -> &'static str {
        match self {
            ProviderPreset::DeepSeek => DEEPSEEK_ENDPOINT,
            ProviderPreset::OpenCodeGo => OPENCODE_GO_ENDPOINT,
        }
    }

    pub const fn models(self) -> &'static [&'static str] {
        match self {
            ProviderPreset::DeepSeek => DEEPSEEK_MODELS,
            ProviderPreset::OpenCodeGo => OPENCODE_GO_MODELS,
        }
    }

    pub fn default_model(self) -> &'static str {
        self.models().first().copied().unwrap_or("deepseek-chat")
    }

    /// Apply the preset to settings, keeping the current model when it is valid.
    pub fn apply(self, settings: &mut ProviderSettings) {
        settings.kind = ProviderKind::OpenAiCompatible;
        settings.endpoint = self.endpoint().to_owned();
        if !self.models().contains(&settings.model.as_str()) {
            settings.model = self.default_model().to_owned();
        }
        // Reasoning models spend a large part of the budget on `reasoning_content`
        // before emitting the answer; 1k tokens is not enough for a function rewrite.
        settings.max_tokens = settings.max_tokens.max(32768);
        settings.enabled = false;
    }
}

/// Provider configuration. Defaults are offline: heuristic, disabled, empty endpoint.
#[derive(Clone)]
pub struct ProviderSettings {
    pub kind: ProviderKind,
    pub endpoint: String,
    pub model: String,
    /// Name of the environment variable that holds the API key (never the key itself).
    pub api_key_env: String,
    /// Session-only key entered in the UI; never serialized by [`Self::save`].
    pub api_key: Option<String>,
    pub max_tokens: usize,
    /// True only when a non-heuristic provider has an endpoint and a key is available.
    pub enabled: bool,
}

impl std::fmt::Debug for ProviderSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderSettings")
            .field("kind", &self.kind)
            .field("endpoint", &self.endpoint)
            .field("model", &self.model)
            .field("api_key_env", &self.api_key_env)
            .field("api_key", &self.api_key.as_ref().map(|_| "<redacted>"))
            .field("max_tokens", &self.max_tokens)
            .field("enabled", &self.enabled)
            .finish()
    }
}

impl Default for ProviderSettings {
    fn default() -> Self {
        ProviderSettings {
            kind: ProviderKind::Heuristic,
            endpoint: String::new(),
            model: "gpt-4o-mini".to_owned(),
            api_key_env: "RECRAFT_AI_API_KEY".to_owned(),
            api_key: None,
            max_tokens: 32768,
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

    /// Persisted provider settings location (`%LOCALAPPDATA%\ReCraft\ai-settings.json`).
    pub fn settings_path() -> Option<PathBuf> {
        std::env::var_os("LOCALAPPDATA")
            .map(|dir| PathBuf::from(dir).join("ReCraft").join("ai-settings.json"))
    }

    /// Load persisted settings, falling back to environment-only configuration. `enabled`
    /// is always recomputed from the current environment (keys are never stored).
    pub fn load() -> Self {
        match Self::settings_path().as_deref().and_then(Self::load_from) {
            Some(mut settings) => {
                settings.refresh_enabled();
                settings
            }
            None => Self::from_env(),
        }
    }

    /// Read settings from an explicit JSON file (UI/tests).
    pub fn load_from(path: &Path) -> Option<Self> {
        let text = std::fs::read_to_string(path).ok()?;
        let value: serde_json::Value = serde_json::from_str(&text).ok()?;
        let mut settings = Self::default();
        if let Some(kind) = value
            .get("kind")
            .and_then(|value| value.as_str())
            .and_then(ProviderKind::parse)
        {
            settings.kind = kind;
        }
        if let Some(endpoint) = value.get("endpoint").and_then(|value| value.as_str()) {
            settings.endpoint = endpoint.to_owned();
        }
        if let Some(model) = value.get("model").and_then(|value| value.as_str()) {
            settings.model = model.to_owned();
        }
        if let Some(api_key_env) = value.get("api_key_env").and_then(|value| value.as_str()) {
            settings.api_key_env = api_key_env.to_owned();
        }
        if let Some(max_tokens) = value.get("max_tokens").and_then(|value| value.as_u64()) {
            settings.max_tokens = max_tokens as usize;
        }
        Some(settings)
    }

    /// Persist to the default per-user location.
    pub fn save(&self) -> Result<()> {
        let Some(path) = Self::settings_path() else {
            return Err(Error::internal("LOCALAPPDATA is not set"));
        };
        self.save_to(&path)
    }

    /// Persist to an explicit path (creates parent directories). The API key itself is
    /// never written — only the name of the environment variable holding it.
    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                Error::internal(format!("cannot create {}: {error}", parent.display()))
            })?;
        }
        let value = serde_json::json!({
            "kind": self.kind.as_str(),
            "endpoint": self.endpoint,
            "model": self.model,
            "api_key_env": self.api_key_env,
            "max_tokens": self.max_tokens,
        });
        let text = serde_json::to_string_pretty(&value)
            .map_err(|error| Error::internal(format!("cannot encode settings: {error}")))?;
        std::fs::write(path, text).map_err(|error| {
            Error::internal(format!("cannot write {}: {error}", path.display()))
        })?;
        Ok(())
    }

    /// Recompute `enabled` from the current environment after the user edits settings.
    pub fn refresh_enabled(&mut self) {
        self.enabled = self.kind != ProviderKind::Heuristic
            && !self.endpoint.is_empty()
            && self.api_key_present();
    }

    /// Whether a session key or the configured API-key environment variable is present.
    pub fn api_key_present(&self) -> bool {
        self.api_key.as_deref().is_some_and(|key| !key.is_empty())
            || std::env::var(&self.api_key_env).is_ok_and(|value| !value.is_empty())
    }

    /// The effective key (session key first, then the configured environment variable).
    pub fn resolved_api_key(&self) -> Option<String> {
        self.api_key
            .clone()
            .filter(|key| !key.is_empty())
            .or_else(|| std::env::var(&self.api_key_env).ok())
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

/// Version of the built-in prompt templates (bump whenever wording changes; the eval
/// baseline and audit entries reference it).
pub const PROMPT_TEMPLATE_VERSION: &str = "rc-ai-2026-10-v1";

/// System prompt for function explanation (template `explain-v1`).
pub const EXPLAIN_SYSTEM: &str = "You are ReCraft's reverse-engineering assistant. \
Explain what the given function does in plain language. Use only the supplied evidence \
(API calls, strings, pseudo-code); never invent behaviour. Mention uncertainty explicitly.";

/// System prompt for rename proposals (template `rename-v1`).
pub const RENAME_SYSTEM: &str = "You are ReCraft's symbol-naming assistant. Propose concise \
snake_case names for the listed functions. Base every suggestion on the supplied API \
calls, strings and pseudo-code, and keep the evidence addresses in your rationale.";

/// System prompt for pseudo-code polishing (DESIGN §9.2: post-processing, never stored).
pub const POLISH_SYSTEM: &str = "You are a reverse-engineering decompiler. Reconstruct the \
most likely original C source from the supplied pseudo-C. You MUST preserve every memory \
access, call, constant, comparison and control-flow edge; do not invent or delete behavior, \
and do not substitute a different library function for one that appears in the input. \
You MAY recover meaningful local variable names and types (e.g. a stack buffer passed to \
fgets becomes `char input[0x100]`), fold argument setup into calls, replace goto labels with \
while/for/if/else when the edges are equivalent, and remove prologue/epilogue register saves. \
Output only the reconstructed C function(s), without markdown fences or commentary.";

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

/// Build a pseudo-code polishing prompt from the function's L1 output. The result is a
/// suggestion only: it is never written back to the project (DESIGN §9.2 / §9.3).
pub fn build_polish_prompt(
    session: &mut Session,
    addr: Address,
    settings: &ProviderSettings,
) -> Result<PromptRequest> {
    let budget = TokenBudget::new(settings.max_tokens);
    let functions = session.functions()?;
    let function = functions
        .iter()
        .find(|function| addr >= function.start && addr < function.end)
        .copied();
    let (start, end) = function
        .map(|function| (function.start, function.end))
        .unwrap_or((addr, addr));
    let pseudo = recraft_decompile::pseudocode_l2(session, start)
        .or_else(|_| recraft_decompile::pseudocode_l1(session, start))?;
    let prototype = session.prototype_at(start)?;
    let mut user = String::new();
    user.push_str(&format!("function: {:#x}..{:#x}\n", start.get(), end.get()));
    if let Some(prototype) = prototype {
        user.push_str(&format!("known prototype: {prototype}\n"));
    }
    user.push_str(
        "task: recover the original source. Name stack buffers from their use, reuse \
         argument names, turn register moves into expressions, and structure branches/loops. \
         Keep every external call and constant; do not add a flag or a different algorithm.\n",
    );
    user.push_str("pseudo-code (L2):\n");
    for line in pseudo.lines.iter().take(240) {
        user.push_str(&line.text);
        user.push('\n');
    }
    let user = redact(&user);
    Ok(PromptRequest::new(budget, POLISH_SYSTEM, &user))
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
        headers.push((
            "User-Agent".to_owned(),
            format!("recraft/{}", env!("CARGO_PKG_VERSION")),
        ));
        if self.settings.endpoint.contains("opencode.ai") {
            headers.push(("x-opencode-session".to_owned(), opencode_session_id()));
        }
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
                response.body.chars().take(800).collect::<String>()
            )));
        }
        let text = crate::transport::parse_openai_response(&response.body)?;
        if text.trim().is_empty() {
            return Err(Error::internal(format!(
                "openai returned an empty completion (max_tokens={}); reasoning models \
                 can consume the whole budget on reasoning_content — raise \
                 RECRAFT_AI_MAX_TOKENS (try 8192–16384) or pick a non-reasoning model",
                self.settings.max_tokens
            )));
        }
        Ok(text)
    }
}

/// Stable per-process session id for OpenCode Go's routing/cache hint header.
fn opencode_session_id() -> String {
    use std::sync::OnceLock;
    static SESSION: OnceLock<String> = OnceLock::new();
    SESSION
        .get_or_init(|| {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or_default();
            format!("recraft-{:x}-{nanos:x}", std::process::id())
        })
        .clone()
}

/// Build the provider selected by `settings`; disabled settings stay offline.
pub fn provider_from_settings(settings: &ProviderSettings) -> Box<dyn Provider> {
    if !settings.enabled || settings.kind == ProviderKind::Heuristic {
        return Box::new(HeuristicProvider);
    }
    match settings.kind {
        ProviderKind::Ollama => Box::new(OllamaProvider::new(
            settings.clone(),
            crate::transport::transport_for(&settings.endpoint),
        )),
        ProviderKind::OpenAiCompatible => {
            let api_key = settings.resolved_api_key();
            Box::new(OpenAiCompatibleProvider::new(
                settings.clone(),
                crate::transport::transport_for(&settings.endpoint),
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
    fn settings_roundtrip_through_json_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ai-settings.json");
        let settings = ProviderSettings {
            kind: ProviderKind::OpenAiCompatible,
            endpoint: "https://api.example.test/v1".to_owned(),
            model: "test-model".to_owned(),
            api_key_env: "MY_TEST_KEY".to_owned(),
            max_tokens: 512,
            ..ProviderSettings::default()
        };
        settings.save_to(&path).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(!text.contains("sk-")); // no key material is written
        let loaded = ProviderSettings::load_from(&path).expect("settings reload");
        assert_eq!(loaded.kind, ProviderKind::OpenAiCompatible);
        assert_eq!(loaded.endpoint, settings.endpoint);
        assert_eq!(loaded.model, settings.model);
        assert_eq!(loaded.api_key_env, settings.api_key_env);
        assert_eq!(loaded.max_tokens, settings.max_tokens);
        assert!(
            !loaded.enabled,
            "enabled is recomputed from the environment"
        );
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
            api_key: None,
            max_tokens: 256,
            enabled: true,
        }
    }

    #[test]
    fn presets_fill_endpoint_and_known_model() {
        let mut settings = ProviderSettings {
            model: "custom-model".to_owned(),
            ..ProviderSettings::default()
        };
        ProviderPreset::DeepSeek.apply(&mut settings);
        assert_eq!(settings.kind, ProviderKind::OpenAiCompatible);
        assert_eq!(settings.endpoint, DEEPSEEK_ENDPOINT);
        assert_eq!(settings.model, "deepseek-chat");

        ProviderPreset::OpenCodeGo.apply(&mut settings);
        assert_eq!(settings.endpoint, OPENCODE_GO_ENDPOINT);
        assert_eq!(settings.model, "deepseek-v4.1-flash");
    }

    #[test]
    fn session_key_enables_provider_without_being_persisted() {
        let mut settings = ProviderSettings {
            kind: ProviderKind::OpenAiCompatible,
            endpoint: DEEPSEEK_ENDPOINT.to_owned(),
            model: "deepseek-chat".to_owned(),
            api_key: Some("sk-session-secret".to_owned()),
            ..ProviderSettings::default()
        };
        settings.refresh_enabled();
        assert!(settings.enabled);
        assert!(settings.api_key_present());
        assert_eq!(
            settings.resolved_api_key().as_deref(),
            Some("sk-session-secret")
        );

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ai-settings.json");
        settings.save_to(&path).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(
            !text.contains("sk-session-secret"),
            "the key must never be written to disk: {text}"
        );
        assert!(text.contains(DEEPSEEK_ENDPOINT));
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
        assert!(
            recorded[0]
                .headers
                .iter()
                .any(|(name, value)| name == "User-Agent" && value.starts_with("recraft/"))
        );
    }

    #[test]
    fn opencode_go_requests_include_session_header() {
        let mut transport = crate::transport::MockTransport::new();
        let requests = transport.requests_handle();
        transport.push(200, r#"{"choices":[{"message":{"content":"ok"}}]}"#);
        let settings = test_settings(
            ProviderKind::OpenAiCompatible,
            "https://opencode.ai/inference/go/openai",
        );
        let mut provider = OpenAiCompatibleProvider::new(
            settings,
            Box::new(transport),
            Some("sk-opencode".to_owned()),
        );
        let request = PromptRequest::new(TokenBudget::new(128), "system", "user");
        assert_eq!(provider.complete(&request).unwrap(), "ok");

        let recorded = requests.lock().unwrap();
        assert_eq!(
            recorded[0].url,
            "https://opencode.ai/inference/go/openai/v1/chat/completions"
        );
        assert!(
            recorded[0]
                .headers
                .iter()
                .any(|(name, _)| name == "x-opencode-session")
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
