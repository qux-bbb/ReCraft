// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! LLM 供应商抽象与提示词配置（DESIGN §9.3）。
//!
//! 网络 I/O 只发生在 `OpenAiCompatibleProvider` / `OllamaProvider` 里，且都经过
//! `transport`，因此测试可以用 mock 传输离线跑完整条链路。
//!
//! - [`ProviderSettings`] 在配置了 endpoint、model 与 API 密钥之前保持禁用（`enabled = false`）；
//!   设置既可从 `ai-settings.json` 读取，也可从 `RECRAFT_AI_*` 环境变量推导。
//! - 内置提示词只剩智能体这一个 [`PromptKind`]；[`system_prompt`] 返回用户的覆盖文本或内置文本，
//!   覆盖值按 [`prompt_key`] 存在设置文件里。
//! - [`PromptRequest`] / [`ChatMessage`] 与 SSE 解码器把模型回复拆成可直接渲染的增量。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use recraft_common::{Error, Result};
use serde_json::Value;

/// Which provider backend a session is configured to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    /// OpenAI-compatible HTTP endpoint.
    OpenAiCompatible,
    /// Local Ollama HTTP endpoint.
    Ollama,
}

impl ProviderKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            ProviderKind::OpenAiCompatible => "openai",
            ProviderKind::Ollama => "ollama",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text.to_ascii_lowercase().as_str() {
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
/// Current DeepSeek model ids. `deepseek-chat` / `deepseek-reasoner` were retired on
/// 2026-07-24 and no longer resolve, so the picker only offers the live V4 lineup.
pub const DEEPSEEK_MODELS: &[&str] = &["deepseek-flash", "deepseek-v4-pro"];
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
        self.models().first().copied().unwrap_or("deepseek-flash")
    }

    /// Apply the preset to settings, keeping the current model when it is valid.
    pub fn apply(self, settings: &mut ProviderSettings) {
        settings.kind = ProviderKind::OpenAiCompatible;
        settings.endpoint = self.endpoint().to_owned();
        if !self.models().contains(&settings.model.as_str()) {
            settings.model = self.default_model().to_owned();
        }
        // Reasoning models spend a large part of the budget on `reasoning_content`
        // before emitting the answer; deepseek-v4-pro needs ~64k for this task.
        settings.max_tokens = settings.max_tokens.max(65536);
        settings.enabled = false;
    }
}

/// Provider configuration. A provider must be configured before AI features can run.
#[derive(Clone)]
pub struct ProviderSettings {
    pub kind: ProviderKind,
    pub endpoint: String,
    pub model: String,
    /// Name of the environment variable that holds the API key (never the key itself).
    pub api_key_env: String,
    /// API key entered in the UI. Persisted sealed as `api_key_sealed` (DPAPI on Windows).
    pub api_key: Option<String>,
    /// Remembered keys per provider identity so switching back does not discard a key.
    /// Values are sealed independently in `ai-settings.json`.
    pub saved_api_keys: BTreeMap<String, String>,
    pub max_tokens: usize,
    /// True only when a provider has an endpoint and a key is available.
    pub enabled: bool,
    /// Language of the built-in system prompts (independent of the UI language).
    pub prompt_language: PromptLanguage,
    /// User edits keyed by [`prompt_key`]; an absent key means "use the built-in text".
    pub prompt_overrides: BTreeMap<String, String>,
}

impl std::fmt::Debug for ProviderSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderSettings")
            .field("kind", &self.kind)
            .field("endpoint", &self.endpoint)
            .field("model", &self.model)
            .field("api_key_env", &self.api_key_env)
            .field("api_key", &self.api_key.as_ref().map(|_| "<redacted>"))
            .field("saved_api_keys", &self.saved_api_keys.len())
            .field("max_tokens", &self.max_tokens)
            .field("enabled", &self.enabled)
            .field("prompt_language", &self.prompt_language)
            .field("prompt_overrides", &self.prompt_overrides.len())
            .finish()
    }
}

impl Default for ProviderSettings {
    fn default() -> Self {
        ProviderSettings {
            kind: ProviderKind::OpenAiCompatible,
            endpoint: String::new(),
            model: "gpt-4o-mini".to_owned(),
            api_key_env: "RECRAFT_AI_API_KEY".to_owned(),
            api_key: None,
            saved_api_keys: BTreeMap::new(),
            max_tokens: 65536,
            enabled: false,
            prompt_language: PromptLanguage::En,
            prompt_overrides: BTreeMap::new(),
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
        settings.enabled = !settings.endpoint.is_empty()
            && lookup(&settings.api_key_env).is_some_and(|key| !key.is_empty());
        settings
    }

    /// Read the real process environment (disabled until a provider is configured).
    pub fn from_env() -> Self {
        ProviderSettings::from_lookup(|name| std::env::var(name).ok())
    }

    /// Stable identity for remembering a key across provider switches.
    pub fn provider_key_id(&self) -> String {
        let endpoint = self.endpoint.trim_end_matches('/');
        if endpoint.contains("api.deepseek.com") {
            "deepseek".to_owned()
        } else if endpoint.contains("opencode.ai") {
            "opencode-go".to_owned()
        } else if endpoint.is_empty() {
            "unconfigured".to_owned()
        } else {
            format!("custom:{endpoint}")
        }
    }

    /// Remember the currently entered key under the current provider identity.
    pub fn remember_current_key(&mut self) {
        if let Some(key) = self.api_key.as_deref().filter(|key| !key.is_empty()) {
            self.saved_api_keys
                .insert(self.provider_key_id(), key.to_owned());
        }
    }

    /// Persisted provider settings location (`%LOCALAPPDATA%\ReCraft\ai-settings.json`).
    pub fn settings_path() -> Option<PathBuf> {
        std::env::var_os("LOCALAPPDATA")
            .map(|dir| PathBuf::from(dir).join("ReCraft").join("ai-settings.json"))
    }

    /// Load persisted settings, falling back to environment-only configuration. `enabled`
    /// is always recomputed from the current environment/key state.
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
        settings.migrate_retired_model();
        if let Some(api_key_env) = value.get("api_key_env").and_then(|value| value.as_str()) {
            settings.api_key_env = api_key_env.to_owned();
        }
        if let Some(max_tokens) = value.get("max_tokens").and_then(|value| value.as_u64()) {
            settings.max_tokens = max_tokens as usize;
        }
        if let Some(language) = value
            .get("prompt_language")
            .and_then(|value| value.as_str())
            .and_then(PromptLanguage::parse)
        {
            settings.prompt_language = language;
        }
        if let Some(prompts) = value.get("prompts").and_then(|value| value.as_object()) {
            for (key, text) in prompts {
                if let Some(text) = text.as_str() {
                    settings
                        .prompt_overrides
                        .insert(key.clone(), text.to_owned());
                }
            }
        }
        if let Some(sealed) = value.get("api_key_sealed").and_then(|value| value.as_str()) {
            settings.api_key = crate::secret::open(sealed);
        }
        if let Some(keys) = value.get("api_keys").and_then(|value| value.as_object()) {
            for (id, sealed) in keys {
                if let Some(key) = sealed.as_str().and_then(crate::secret::open) {
                    settings.saved_api_keys.insert(id.clone(), key);
                }
            }
        }
        if let Some(key) = settings.api_key.clone() {
            let id = settings.provider_key_id();
            settings.saved_api_keys.entry(id).or_insert(key);
        }
        Some(settings)
    }

    /// Replace a model id the DeepSeek API has retired with the current preset default, so
    /// settings persisted by older builds do not keep requesting a model that no longer
    /// resolves.
    fn migrate_retired_model(&mut self) {
        if self.endpoint.contains("api.deepseek.com")
            && matches!(self.model.as_str(), "deepseek-chat" | "deepseek-reasoner")
        {
            self.model = ProviderPreset::DeepSeek.default_model().to_owned();
        }
    }

    /// Persist to the default per-user location.
    pub fn save(&self) -> Result<()> {
        let Some(path) = Self::settings_path() else {
            return Err(Error::internal("LOCALAPPDATA is not set"));
        };
        self.save_to(&path)
    }

    /// Persist to an explicit path (creates parent directories). An entered API key is
    /// sealed at rest (DPAPI on Windows, `plain:` fallback elsewhere); environment-variable
    /// names are stored separately so env-based setups keep working.
    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                Error::internal(format!("cannot create {}: {error}", parent.display()))
            })?;
        }
        let mut value = serde_json::json!({
            "kind": self.kind.as_str(),
            "endpoint": self.endpoint,
            "model": self.model,
            "api_key_env": self.api_key_env,
            "max_tokens": self.max_tokens,
            "prompt_language": self.prompt_language.as_str(),
            "prompts": self.prompt_overrides,
        });
        if let Some(key) = self.api_key.as_deref().filter(|key| !key.is_empty()) {
            value["api_key_sealed"] = serde_json::Value::String(crate::secret::seal(key)?);
        }
        let mut saved = self.saved_api_keys.clone();
        if let Some(key) = self.api_key.as_deref().filter(|key| !key.is_empty()) {
            saved.insert(self.provider_key_id(), key.to_owned());
        }
        let mut sealed_keys = serde_json::Map::new();
        for (id, key) in saved {
            sealed_keys.insert(id, serde_json::Value::String(crate::secret::seal(&key)?));
        }
        value["api_keys"] = serde_json::Value::Object(sealed_keys);
        let text = serde_json::to_string_pretty(&value)
            .map_err(|error| Error::internal(format!("cannot encode settings: {error}")))?;
        std::fs::write(path, text).map_err(|error| {
            Error::internal(format!("cannot write {}: {error}", path.display()))
        })?;
        Ok(())
    }

    /// Recompute `enabled` from the current environment after the user edits settings.
    pub fn refresh_enabled(&mut self) {
        self.enabled = !self.endpoint.is_empty() && self.api_key_present();
    }

    /// Store (`Some`) or clear (`None`) the user's text for the current prompt language.
    /// Empty text counts as clearing, so the built-in prompt comes back.
    pub fn set_prompt_override(&mut self, kind: PromptKind, text: Option<String>) {
        let key = prompt_key(kind, self.prompt_language);
        match text.filter(|text| !text.trim().is_empty()) {
            Some(text) => {
                self.prompt_overrides.insert(key, text);
            }
            None => {
                self.prompt_overrides.remove(&key);
            }
        }
    }

    /// The shipped prompt for `kind` in the configured language ("reset" restores this).
    pub fn builtin_prompt(&self, kind: PromptKind) -> &'static str {
        kind.builtin(self.prompt_language)
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

    /// Human-readable backend name. `kind` only says OpenAI-compatible; presets such as
    /// DeepSeek and OpenCode Go share that transport but must not both display as `openai`.
    pub fn provider_name(&self) -> &'static str {
        let endpoint = self.endpoint.as_str();
        if endpoint.contains("api.deepseek.com") {
            "deepseek"
        } else if endpoint.contains("opencode.ai") {
            "opencode-go"
        } else {
            match self.kind {
                ProviderKind::Ollama => "ollama",
                ProviderKind::OpenAiCompatible => "openai-compatible",
            }
        }
    }

    /// One-line description safe to show in UI/CLI (never prints a key).
    pub fn describe(&self) -> String {
        let state = if self.enabled { "enabled" } else { "disabled" };
        format!(
            "provider={} state={state} model={} endpoint={} key_env={} max_tokens={}",
            self.provider_name(),
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
    /// Whether the model may spend tokens on chain-of-thought before answering.
    /// DeepSeek V4 enables thinking by default, which delays the first content token by
    /// about a minute; ReCraft asks for a direct answer unless a request opts in.
    pub thinking: bool,
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
            thinking: false,
        }
    }

    /// Opt into the provider's reasoning pass (slower, occasionally better answers).
    pub fn with_thinking(mut self) -> Self {
        self.thinking = true;
        self
    }
}

/// 内置提示词模板的版本号：措辞一改就递增，便于把用户保存的覆盖文本与之对应起来。
pub const PROMPT_TEMPLATE_VERSION: &str = "rc-ai-2026-11-v1";

/// 需要系统提示词的场景；产品只剩智能体一个，所以枚举只有一个变体。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PromptKind {
    /// 项目级智能体的工具调用循环。
    Agent,
}

impl PromptKind {
    /// Identifier used by the settings file and the override keys.
    pub const fn key(self) -> &'static str {
        match self {
            PromptKind::Agent => "agent",
        }
    }

    /// The shipped prompt in `language`.
    pub fn builtin(self, language: PromptLanguage) -> &'static str {
        match (self, language) {
            (PromptKind::Agent, PromptLanguage::En) => crate::agent::AGENT_SYSTEM,
            (PromptKind::Agent, PromptLanguage::Zh) => crate::agent::AGENT_SYSTEM_ZH,
        }
    }
}

/// Language of the built-in prompts. Independent of the UI language: models often behave
/// better with an English system prompt even in a Chinese session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PromptLanguage {
    #[default]
    En,
    Zh,
}

impl PromptLanguage {
    pub fn as_str(self) -> &'static str {
        match self {
            PromptLanguage::En => "en",
            PromptLanguage::Zh => "zh",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "en" | "en-US" | "english" => Some(PromptLanguage::En),
            "zh" | "zh-CN" | "chinese" => Some(PromptLanguage::Zh),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            PromptLanguage::En => "English",
            PromptLanguage::Zh => "中文",
        }
    }
}

/// The key a user override for `kind`/`language` is stored under (`"zh:agent"`).
pub fn prompt_key(kind: PromptKind, language: PromptLanguage) -> String {
    format!("{}:{}", language.as_str(), kind.key())
}

/// The system prompt to send for `kind`: the user's override, else the built-in text.
pub fn system_prompt(settings: &ProviderSettings, kind: PromptKind) -> String {
    let language = settings.prompt_language;
    settings
        .prompt_overrides
        .get(&prompt_key(kind, language))
        .cloned()
        .unwrap_or_else(|| kind.builtin(language).to_owned())
}

/// One function the model may call (an OpenAI `tools` entry).
#[derive(Debug, Clone)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    /// JSON Schema object describing the arguments.
    pub parameters: Value,
}

impl ToolSpec {
    fn to_json(&self) -> Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": self.name,
                "description": self.description,
                "parameters": self.parameters,
            },
        })
    }
}

/// A tool invocation requested by the model.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCall {
    /// Provider-assigned id; the tool result must echo it back.
    pub id: String,
    pub name: String,
    /// Parsed JSON arguments (an unparsable payload becomes `{"_raw": "..."}`).
    pub arguments: Value,
}

/// What one assistant turn contains: final text, or tools to run.
#[derive(Debug, Clone)]
pub enum ToolReply {
    Text(String),
    Calls(Vec<ToolCall>),
}

/// Who wrote one message in a tool-calling conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatRole {
    System,
    User,
    Assistant,
    Tool,
}

impl ChatRole {
    const fn as_str(self) -> &'static str {
        match self {
            ChatRole::System => "system",
            ChatRole::User => "user",
            ChatRole::Assistant => "assistant",
            ChatRole::Tool => "tool",
        }
    }
}

/// One message in a tool-calling conversation.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    /// Only for [`ChatRole::Assistant`] turns that call tools.
    pub tool_calls: Vec<ToolCall>,
    /// Only for [`ChatRole::Tool`] turns: the call this result answers.
    pub tool_call_id: Option<String>,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::System,
            content: content.into(),
            tool_calls: Vec::new(),
            tool_call_id: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::User,
            content: content.into(),
            tool_calls: Vec::new(),
            tool_call_id: None,
        }
    }

    pub fn assistant(content: impl Into<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: ChatRole::Assistant,
            content: content.into(),
            tool_calls,
            tool_call_id: None,
        }
    }

    pub fn tool(call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::Tool,
            content: content.into(),
            tool_calls: Vec::new(),
            tool_call_id: Some(call_id.into()),
        }
    }

    fn to_json(&self) -> Value {
        let mut value = serde_json::json!({
            "role": self.role.as_str(),
            "content": self.content,
        });
        if let Some(call_id) = &self.tool_call_id {
            value["tool_call_id"] = serde_json::json!(call_id);
        }
        if !self.tool_calls.is_empty() {
            value["tool_calls"] = Value::Array(
                self.tool_calls
                    .iter()
                    .map(|call| {
                        serde_json::json!({
                            "id": call.id,
                            "type": "function",
                            "function": {
                                "name": call.name,
                                "arguments": call.arguments.to_string(),
                            },
                        })
                    })
                    .collect(),
            );
        }
        value
    }
}

/// The offline-safe provider trait a future HTTP transport implements.
pub trait Provider {
    fn name(&self) -> &'static str;
    fn complete(&mut self, request: &PromptRequest) -> Result<String>;

    /// Run one tool-calling turn. Providers without tool support must refuse: silently
    /// dropping the tools would leave the agent asking for them forever.
    fn complete_chat(
        &mut self,
        _messages: &[ChatMessage],
        _tools: &[ToolSpec],
        _thinking: bool,
    ) -> Result<ToolReply> {
        Err(Error::unsupported(
            "this provider does not support tool calls; configure an OpenAI-compatible endpoint",
        ))
    }

    /// Streaming variant of [Provider::complete_chat]: content deltas go to on_chunk
    /// while tool-call fragments are accumulated. The default delivers the finished answer
    /// as one chunk, so providers without SSE keep working.
    fn complete_chat_stream(
        &mut self,
        messages: &[ChatMessage],
        tools: &[ToolSpec],
        thinking: bool,
        on_chunk: &mut dyn FnMut(&str),
    ) -> Result<ToolReply> {
        let reply = self.complete_chat(messages, tools, thinking)?;
        if let ToolReply::Text(text) = &reply {
            on_chunk(text);
        }
        Ok(reply)
    }

    /// Streaming variant used by the desktop UI. The default preserves providers that
    /// do not implement SSE by delivering the completed answer as one chunk.
    fn complete_stream(
        &mut self,
        request: &PromptRequest,
        on_chunk: &mut dyn FnMut(&str),
    ) -> Result<String> {
        let text = self.complete(request)?;
        on_chunk(&text);
        Ok(text)
    }

    /// Streaming variant that also forwards the model's chain-of-thought deltas
    /// (`reasoning_content`) when the provider emits them. Providers without a reasoning
    /// channel simply never call `on_reasoning`.
    fn complete_stream_with_reasoning(
        &mut self,
        request: &PromptRequest,
        on_chunk: &mut dyn FnMut(&str),
        _on_reasoning: &mut dyn FnMut(&str),
    ) -> Result<String> {
        self.complete_stream(request, on_chunk)
    }
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

    /// Whether the request must explicitly turn the provider's thinking mode off.
    fn thinking_toggle(&self, request: &PromptRequest) -> Option<bool> {
        let endpoint = &self.settings.endpoint;
        // DeepSeek V4 turns thinking on by default (high effort), and the reasoning pass
        // delays the first content token by about a minute. Both built-in presets always
        // send an explicit toggle: direct answer by default, reasoning when the UI opts in.
        (endpoint.contains("api.deepseek.com") || endpoint.contains("opencode.ai"))
            .then_some(request.thinking)
    }

    /// Request body for a tool-calling turn (`stream` selects SSE).
    fn chat_body(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolSpec],
        thinking: bool,
        stream: bool,
    ) -> Value {
        let mut body = serde_json::json!({
            "model": self.settings.model,
            "max_tokens": self.settings.max_tokens,
            "stream": stream,
            "tool_choice": "auto",
            "messages": messages.iter().map(ChatMessage::to_json).collect::<Vec<_>>(),
            "tools": tools.iter().map(ToolSpec::to_json).collect::<Vec<_>>(),
        });
        if self.settings.endpoint.contains("api.deepseek.com")
            || self.settings.endpoint.contains("opencode.ai")
        {
            body["thinking"] = serde_json::json!({
                "type": if thinking { "enabled" } else { "disabled" },
            });
        }
        body
    }

    /// Headers every JSON request carries (auth, user agent, OpenCode session affinity).
    fn json_headers(&self) -> Vec<(String, String)> {
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
        headers
    }

    fn stream_request(&self, request: &PromptRequest) -> crate::transport::HttpRequest {
        let url = format!(
            "{}/v1/chat/completions",
            self.settings.endpoint.trim_end_matches('/')
        );
        let mut body = serde_json::json!({
            "model": self.settings.model,
            "max_tokens": self.settings.max_tokens,
            "stream": true,
            "messages": [
                {"role": "system", "content": request.system},
                {"role": "user", "content": request.user},
            ],
        });
        // DeepSeek V4 enables thinking by default, and the reasoning pass delays the first
        // content token by up to a minute. The built-in presets always send an explicit
        // toggle so the UI's "show thinking" switch fully controls the mode.
        if let Some(enabled) = self.thinking_toggle(request) {
            body["thinking"] = serde_json::json!({
                "type": if enabled { "enabled" } else { "disabled" },
            });
        }
        let body = body.to_string();
        let mut headers = vec![("Content-Type".to_owned(), "application/json".to_owned())];
        headers.push((
            "User-Agent".to_owned(),
            format!("recraft/{}", env!("CARGO_PKG_VERSION")),
        ));
        // Some OpenAI-compatible gateways buffer until the request advertises SSE.
        headers.push(("Accept".to_owned(), "text/event-stream".to_owned()));
        headers.push(("Cache-Control".to_owned(), "no-cache".to_owned()));
        headers.push(("Accept-Encoding".to_owned(), "identity".to_owned()));
        if self.settings.endpoint.contains("opencode.ai") {
            headers.push(("x-opencode-session".to_owned(), opencode_session_id()));
        }
        if let Some(key) = &self.api_key {
            headers.push(("Authorization".to_owned(), format!("Bearer {key}")));
        }
        crate::transport::HttpRequest {
            method: "POST",
            url,
            headers,
            body,
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
        let mut body = serde_json::json!({
            "model": self.settings.model,
            "max_tokens": self.settings.max_tokens,
            "messages": [
                {"role": "system", "content": request.system},
                {"role": "user", "content": request.user},
            ],
        });
        if let Some(enabled) = self.thinking_toggle(request) {
            body["thinking"] = serde_json::json!({
                "type": if enabled { "enabled" } else { "disabled" },
            });
        }
        let body = body.to_string();
        let response = self.transport.send(&crate::transport::HttpRequest {
            method: "POST",
            url,
            headers: self.json_headers(),
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
                "openai returned an empty completion (model={}, max_tokens={}); reasoning \
                 models can consume the whole budget on reasoning_content — raise \
                 RECRAFT_AI_MAX_TOKENS (up to 65536) or pick a non-reasoning model",
                self.settings.model, self.settings.max_tokens
            )));
        }
        Ok(strip_code_fences(&text))
    }

    fn complete_chat(
        &mut self,
        messages: &[ChatMessage],
        tools: &[ToolSpec],
        thinking: bool,
    ) -> Result<ToolReply> {
        let url = format!(
            "{}/v1/chat/completions",
            self.settings.endpoint.trim_end_matches('/')
        );
        let response = self.transport.send(&crate::transport::HttpRequest {
            method: "POST",
            url,
            headers: self.json_headers(),
            body: self.chat_body(messages, tools, thinking, false).to_string(),
        })?;
        if response.status != 200 {
            return Err(Error::internal(format!(
                "openai HTTP {}: {}",
                response.status,
                response.body.chars().take(800).collect::<String>()
            )));
        }
        parse_tool_reply(&response.body)
    }

    fn complete_chat_stream(
        &mut self,
        messages: &[ChatMessage],
        tools: &[ToolSpec],
        thinking: bool,
        on_chunk: &mut dyn FnMut(&str),
    ) -> Result<ToolReply> {
        let url = format!(
            "{}/v1/chat/completions",
            self.settings.endpoint.trim_end_matches('/')
        );
        let request = crate::transport::HttpRequest {
            method: "POST",
            url,
            headers: self.json_headers(),
            body: self.chat_body(messages, tools, thinking, true).to_string(),
        };
        let mut decoder = ToolStreamDecoder::default();
        let response = self
            .transport
            .send_stream(&request, &mut |raw| decoder.push(raw, on_chunk))?;
        if response.status != 200 {
            return Err(Error::internal(format!(
                "openai HTTP {}: {}",
                response.status,
                response.body.chars().take(800).collect::<String>()
            )));
        }
        decoder.finish()
    }

    fn complete_stream(
        &mut self,
        request: &PromptRequest,
        on_chunk: &mut dyn FnMut(&str),
    ) -> Result<String> {
        self.complete_stream_with_reasoning(request, on_chunk, &mut |_| {})
    }

    fn complete_stream_with_reasoning(
        &mut self,
        request: &PromptRequest,
        on_chunk: &mut dyn FnMut(&str),
        on_reasoning: &mut dyn FnMut(&str),
    ) -> Result<String> {
        let http = self.stream_request(request);
        let mut decoder = OpenAiSseDecoder::default();
        let response = self
            .transport
            .send_stream(&http, &mut |raw| decoder.push(raw, on_chunk, on_reasoning))?;
        if response.status != 200 {
            return Err(Error::internal(format!(
                "openai HTTP {}: {}",
                response.status,
                response.body.chars().take(800).collect::<String>()
            )));
        }
        let text = decoder.finish(on_chunk, on_reasoning)?;
        if text.trim().is_empty() {
            return Err(Error::internal(
                "openai stream returned no assistant content (reasoning-only or empty stream)",
            ));
        }
        Ok(strip_code_fences(&text))
    }
}

/// Parse an OpenAI-compatible chat completion that may contain `tool_calls`.
///
/// Models differ in how they encode `arguments` (JSON string, already-decoded object, or
/// invalid text); the value is normalised here so the agent never sees a string it has to
/// re-parse. Missing ids get a stable synthetic id so the tool result can be paired back.
pub fn parse_tool_reply(body: &str) -> Result<ToolReply> {
    let value: Value = serde_json::from_str(body)
        .map_err(|error| Error::internal(format!("openai JSON: {error}")))?;
    let message = value
        .pointer("/choices/0/message")
        .ok_or_else(|| Error::internal("openai response has no choices[0].message"))?;
    let mut calls = Vec::new();
    if let Some(items) = message.get("tool_calls").and_then(Value::as_array) {
        for (index, item) in items.iter().enumerate() {
            let Some(name) = item
                .pointer("/function/name")
                .and_then(Value::as_str)
                .filter(|name| !name.is_empty())
            else {
                continue;
            };
            let id = item
                .get("id")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .unwrap_or_else(|| format!("call_{index}"));
            let arguments = match item.pointer("/function/arguments") {
                Some(Value::String(text)) => serde_json::from_str(text)
                    .unwrap_or_else(|_| serde_json::json!({"_raw": text.clone()})),
                Some(value) => value.clone(),
                None => Value::Null,
            };
            calls.push(ToolCall {
                id,
                name: name.to_owned(),
                arguments,
            });
        }
    }
    if !calls.is_empty() {
        return Ok(ToolReply::Calls(calls));
    }
    let text = message
        .get("content")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_owned();
    if text.is_empty() {
        return Err(Error::internal(
            "openai returned no assistant content and no tool calls",
        ));
    }
    Ok(ToolReply::Text(text))
}

/// Incremental parser for the OpenAI `text/event-stream` wire format.
#[derive(Debug, Default)]
struct OpenAiSseDecoder {
    pending: String,
    raw: String,
    content: String,
    reasoning_chars: usize,
    error: Option<String>,
    saw_data: bool,
}

impl OpenAiSseDecoder {
    fn push(
        &mut self,
        raw: &str,
        on_chunk: &mut dyn FnMut(&str),
        on_reasoning: &mut dyn FnMut(&str),
    ) {
        self.raw.push_str(raw);
        self.pending.push_str(raw);
        while let Some(position) = self.pending.find('\n') {
            let line: String = self.pending.drain(..=position).collect();
            self.line(line.trim_end_matches(['\r', '\n']), on_chunk, on_reasoning);
        }
    }

    fn finish(
        mut self,
        on_chunk: &mut dyn FnMut(&str),
        on_reasoning: &mut dyn FnMut(&str),
    ) -> Result<String> {
        if !self.pending.is_empty() {
            let line = std::mem::take(&mut self.pending);
            self.line(line.trim_end_matches(['\r', '\n']), on_chunk, on_reasoning);
        }
        if let Some(error) = self.error.take() {
            return Err(Error::internal(format!("openai stream error: {error}")));
        }
        if self.content.is_empty() && self.reasoning_chars > 0 {
            return Err(Error::internal(format!(
                "openai stream returned only reasoning content ({} chars), no assistant text; \
                 the model used the completion budget before answering. Try a non-reasoning \
                 model (e.g. GLM-5.3-Flash) or a larger max_tokens",
                self.reasoning_chars
            )));
        }
        if self.saw_data {
            return Ok(self.content);
        }
        // Some compatible gateways ignore `stream: true` and return one JSON body.
        let text = crate::transport::parse_openai_response(&self.raw)?;
        if !text.is_empty() {
            on_chunk(&text);
        }
        Ok(text)
    }

    fn line(
        &mut self,
        line: &str,
        on_chunk: &mut dyn FnMut(&str),
        on_reasoning: &mut dyn FnMut(&str),
    ) {
        let Some(data) = line.strip_prefix("data:") else {
            return;
        };
        self.saw_data = true;
        let data = data.trim();
        if data.is_empty() || data == "[DONE]" {
            return;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(data) else {
            return;
        };
        if let Some(error) = value.get("error") {
            self.error = Some(error.to_string());
            return;
        }
        if let Some(content) = value["choices"][0]["delta"]["content"].as_str()
            && !content.is_empty()
        {
            self.content.push_str(content);
            on_chunk(content);
        }
        if let Some(reasoning) = value["choices"][0]["delta"]["reasoning_content"].as_str()
            && !reasoning.is_empty()
        {
            self.reasoning_chars += reasoning.chars().count();
            on_reasoning(reasoning);
        }
    }
}

/// Accumulates tool-call fragments from an OpenAI SSE stream while forwarding content.
///
/// In a streamed turn the assistant content arrives as `delta.content` fragments and the
/// calls as `delta.tool_calls[{index, id, function{name, arguments}}]`, where `arguments`
/// is a JSON *string fragment*: joining them in order is what makes the call usable.
#[derive(Debug, Default)]
struct ToolStreamDecoder {
    pending: String,
    raw: String,
    content: String,
    calls: BTreeMap<usize, PartialCall>,
    error: Option<String>,
    saw_data: bool,
}

#[derive(Debug, Default)]
struct PartialCall {
    id: String,
    name: String,
    arguments: String,
}

impl ToolStreamDecoder {
    fn push(&mut self, raw: &str, on_chunk: &mut dyn FnMut(&str)) {
        self.raw.push_str(raw);
        self.pending.push_str(raw);
        while let Some(position) = self.pending.find('\n') {
            let line: String = self.pending.drain(..=position).collect();
            self.line(line.trim_end_matches(['\r', '\n']), on_chunk);
        }
    }

    fn finish(mut self) -> Result<ToolReply> {
        if !self.pending.is_empty() {
            let line = std::mem::take(&mut self.pending);
            self.line(line.trim_end_matches(['\r', '\n']), &mut |_| {});
        }
        if let Some(error) = self.error.take() {
            return Err(Error::internal(format!("openai stream error: {error}")));
        }
        if !self.saw_data {
            // A gateway that ignores `stream: true` returns one JSON body.
            return parse_tool_reply(&self.raw);
        }
        if !self.calls.is_empty() {
            let calls = std::mem::take(&mut self.calls)
                .into_iter()
                .filter(|(_, call)| !call.name.is_empty())
                .map(|(index, call)| {
                    let arguments = serde_json::from_str(&call.arguments)
                        .unwrap_or_else(|_| serde_json::json!({"_raw": call.arguments}));
                    ToolCall {
                        id: if call.id.is_empty() {
                            format!("call_{index}")
                        } else {
                            call.id
                        },
                        name: call.name,
                        arguments,
                    }
                })
                .collect();
            return Ok(ToolReply::Calls(calls));
        }
        Ok(ToolReply::Text(self.content))
    }

    fn line(&mut self, line: &str, on_chunk: &mut dyn FnMut(&str)) {
        let Some(data) = line.strip_prefix("data:") else {
            return;
        };
        self.saw_data = true;
        let data = data.trim();
        if data.is_empty() || data == "[DONE]" {
            return;
        }
        let Ok(value) = serde_json::from_str::<Value>(data) else {
            return;
        };
        if let Some(error) = value.get("error") {
            self.error = Some(error.to_string());
            return;
        }
        let delta = &value["choices"][0]["delta"];
        if let Some(content) = delta["content"].as_str()
            && !content.is_empty()
        {
            self.content.push_str(content);
            on_chunk(content);
        }
        if let Some(items) = delta["tool_calls"].as_array() {
            for (position, item) in items.iter().enumerate() {
                let index = item["index"]
                    .as_u64()
                    .map_or(position, |value| value as usize);
                let call = self.calls.entry(index).or_default();
                if let Some(id) = item["id"].as_str()
                    && !id.is_empty()
                {
                    call.id = id.to_owned();
                }
                if let Some(name) = item["function"]["name"].as_str()
                    && !name.is_empty()
                    && !call.name.ends_with(name)
                {
                    call.name.push_str(name);
                }
                if let Some(arguments) = item["function"]["arguments"].as_str() {
                    call.arguments.push_str(arguments);
                }
            }
        }
    }
}

/// Remove a leading/trailing Markdown code fence the model may add despite instructions.
fn strip_code_fences(text: &str) -> String {
    let mut text = text.trim();
    if let Some(rest) = text.strip_prefix("```") {
        // Drop the fence line (with optional language tag).
        text = rest.split_once('\n').map_or("", |(_, body)| body);
    }
    if let Some(rest) = text.trim_end().strip_suffix("```") {
        text = rest;
    }
    text.trim().to_owned()
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

/// Build the provider selected by `settings`; unconfigured AI returns a clear error.
pub fn provider_from_settings(settings: &ProviderSettings) -> Box<dyn Provider> {
    if !settings.enabled {
        return Box::new(UnavailableProvider);
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
    }
}

/// Placeholder provider when AI is not configured; it never pretends to be a model.
#[derive(Debug, Default)]
pub struct UnavailableProvider;

impl Provider for UnavailableProvider {
    fn name(&self) -> &'static str {
        "unconfigured"
    }

    fn complete(&mut self, _request: &PromptRequest) -> Result<String> {
        Err(Error::unsupported(
            "no LLM provider configured; open Provider settings and set an endpoint/model/API key",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opencode_stream_disables_thinking() {
        let settings = ProviderSettings {
            kind: ProviderKind::OpenAiCompatible,
            endpoint: OPENCODE_GO_ENDPOINT.to_owned(),
            model: "glm-5.3-flash".to_owned(),
            max_tokens: 8192,
            ..ProviderSettings::default()
        };
        let provider = OpenAiCompatibleProvider::new(
            settings,
            Box::new(crate::transport::MockTransport::new()),
            None,
        );
        let request = PromptRequest {
            system: "system".to_owned(),
            user: "user".to_owned(),
            estimated_tokens: 2,
            thinking: false,
        };
        let http = provider.stream_request(&request);
        assert!(
            http.body.contains("\"thinking\":{\"type\":\"disabled\"}"),
            "{}",
            http.body
        );
        assert!(http.body.contains("\"stream\":true"), "{}", http.body);
    }

    #[test]
    fn deepseek_fast_path_disables_thinking() {
        let settings = ProviderSettings {
            endpoint: DEEPSEEK_ENDPOINT.to_owned(),
            model: "deepseek-flash".to_owned(),
            max_tokens: 8192,
            ..ProviderSettings::default()
        };
        let provider = OpenAiCompatibleProvider::new(
            settings.clone(),
            Box::new(crate::transport::MockTransport::new()),
            None,
        );
        let fast = PromptRequest::new(TokenBudget::new(256), "sys", "user");
        let http = provider.stream_request(&fast);
        assert!(
            http.body.contains("\"thinking\":{\"type\":\"disabled\"}"),
            "{}",
            http.body
        );

        // The UI's thinking toggle sends an explicit `enabled` for the reasoning pass.
        let deep = PromptRequest::new(TokenBudget::new(256), "sys", "user").with_thinking();
        let http = provider.stream_request(&deep);
        assert!(
            http.body.contains("\"thinking\":{\"type\":\"enabled\"}"),
            "{}",
            http.body
        );

        // 非流式路径同样遵守这个开关。
        let mut transport = crate::transport::MockTransport::new();
        let requests = transport.requests_handle();
        transport.push(
            200,
            r#"{"choices":[{"message":{"content":"int main(void) {}"}}]}"#,
        );
        let mut provider = OpenAiCompatibleProvider::new(settings, Box::new(transport), None);
        assert_eq!(provider.complete(&fast).unwrap(), "int main(void) {}");
        let requests = requests.lock().unwrap();
        assert!(
            requests[0]
                .body
                .contains("\"thinking\":{\"type\":\"disabled\"}"),
            "{}",
            requests[0].body
        );
    }

    #[test]
    fn openai_sse_decoder_handles_split_chunks() {
        let mut decoder = OpenAiSseDecoder::default();
        let mut streamed = String::new();
        decoder.push(
            "data: {\"choices\":[{\"delta\":{\"content\":\"int \"}}]}\n\ndata: {\"cho",
            &mut |chunk| streamed.push_str(chunk),
            &mut |_| {},
        );
        decoder.push(
            "ices\":[{\"delta\":{\"content\":\"main(void)\"}}]}\n\ndata: [DONE]\n",
            &mut |chunk| streamed.push_str(chunk),
            &mut |_| {},
        );
        let text = decoder
            .finish(&mut |chunk| streamed.push_str(chunk), &mut |_| {})
            .expect("SSE");
        assert_eq!(text, "int main(void)");
        assert_eq!(streamed, text);
    }

    #[test]
    fn openai_sse_decoder_forwards_reasoning_chunks() {
        let mut decoder = OpenAiSseDecoder::default();
        let mut streamed = String::new();
        let mut reasoning = String::new();
        decoder.push(
            "data: {\"choices\":[{\"delta\":{\"reasoning_content\":\"plan \"}}]}\n\n\
             data: {\"choices\":[{\"delta\":{\"reasoning_content\":\"carefully\"}}]}\n\n\
             data: {\"choices\":[{\"delta\":{\"content\":\"int main\"}}]}\n\ndata: [DONE]\n",
            &mut |chunk| streamed.push_str(chunk),
            &mut |chunk| reasoning.push_str(chunk),
        );
        let text = decoder
            .finish(&mut |chunk| streamed.push_str(chunk), &mut |chunk| {
                reasoning.push_str(chunk)
            })
            .expect("SSE");
        assert_eq!(reasoning, "plan carefully");
        assert_eq!(text, "int main");
        assert_eq!(streamed, text);
    }

    #[test]
    fn openai_sse_decoder_reports_reasoning_only_streams() {
        let mut decoder = OpenAiSseDecoder::default();
        decoder.push(
            "data: {\"choices\":[{\"delta\":{\"reasoning_content\":\"thinking\"}}]}\n\ndata: [DONE]\n",
            &mut |_| {},
            &mut |_| {},
        );
        let error = decoder
            .finish(&mut |_| {}, &mut |_| {})
            .expect_err("reasoning-only");
        assert!(
            error.to_string().contains("only reasoning content"),
            "{error}"
        );
    }

    #[test]
    fn openai_sse_decoder_accepts_non_stream_json_fallback() {
        let mut decoder = OpenAiSseDecoder::default();
        let mut streamed = String::new();
        decoder.push(
            "{\"choices\":[{\"message\":{\"content\":\"fallback\"}}]}",
            &mut |chunk| streamed.push_str(chunk),
            &mut |_| {},
        );
        let text = decoder
            .finish(&mut |chunk| streamed.push_str(chunk), &mut |_| {})
            .expect("fallback");
        assert_eq!(text, "fallback");
        assert_eq!(streamed, text);
    }

    #[test]
    fn openai_provider_streams_sse_chunks() {
        struct SseTransport;
        impl crate::transport::Transport for SseTransport {
            fn send(
                &mut self,
                _request: &crate::transport::HttpRequest,
            ) -> Result<crate::transport::HttpResponse> {
                panic!("streaming path must use send_stream")
            }
            fn send_stream(
                &mut self,
                request: &crate::transport::HttpRequest,
                on_chunk: &mut dyn FnMut(&str),
            ) -> Result<crate::transport::HttpResponse> {
                assert!(request.body.contains("\"stream\":true"), "{}", request.body);
                on_chunk("data: {\"choices\":[{\"delta\":{\"content\":\"int \"}}]}\n\n");
                on_chunk("data: {\"choices\":[{\"delta\":{\"content\":\"main\"}}]}\n\n");
                on_chunk("data: [DONE]\n\n");
                Ok(crate::transport::HttpResponse {
                    status: 200,
                    body: String::new(),
                })
            }
        }

        let settings = ProviderSettings {
            kind: ProviderKind::OpenAiCompatible,
            endpoint: "https://example.invalid".to_owned(),
            model: "test".to_owned(),
            max_tokens: 128,
            ..ProviderSettings::default()
        };
        let mut provider = OpenAiCompatibleProvider::new(settings, Box::new(SseTransport), None);
        let request = PromptRequest {
            system: "system".to_owned(),
            user: "user".to_owned(),
            estimated_tokens: 2,
            thinking: false,
        };
        let mut streamed = String::new();
        let text = provider
            .complete_stream(&request, &mut |chunk| streamed.push_str(chunk))
            .expect("stream");
        assert_eq!(text, "int main");
        assert_eq!(streamed, text);
    }

    #[test]
    fn openai_provider_streams_reasoning_to_the_ui() {
        struct ReasoningTransport;
        impl crate::transport::Transport for ReasoningTransport {
            fn send(
                &mut self,
                _request: &crate::transport::HttpRequest,
            ) -> Result<crate::transport::HttpResponse> {
                panic!("streaming path must use send_stream")
            }
            fn send_stream(
                &mut self,
                _request: &crate::transport::HttpRequest,
                on_chunk: &mut dyn FnMut(&str),
            ) -> Result<crate::transport::HttpResponse> {
                on_chunk("data: {\"choices\":[{\"delta\":{\"reasoning_content\":\"plan \"}}]}\n\n");
                on_chunk("data: {\"choices\":[{\"delta\":{\"reasoning_content\":\"first\"}}]}\n\n");
                on_chunk("data: {\"choices\":[{\"delta\":{\"content\":\"int main\"}}]}\n\n");
                on_chunk("data: [DONE]\n\n");
                Ok(crate::transport::HttpResponse {
                    status: 200,
                    body: String::new(),
                })
            }
        }

        let settings = ProviderSettings {
            kind: ProviderKind::OpenAiCompatible,
            endpoint: "https://example.invalid".to_owned(),
            model: "test".to_owned(),
            max_tokens: 128,
            ..ProviderSettings::default()
        };
        let mut provider =
            OpenAiCompatibleProvider::new(settings, Box::new(ReasoningTransport), None);
        let request = PromptRequest::new(TokenBudget::new(128), "system", "user").with_thinking();
        let mut streamed = String::new();
        let mut reasoning = String::new();
        let text = provider
            .complete_stream_with_reasoning(
                &request,
                &mut |chunk| streamed.push_str(chunk),
                &mut |chunk| reasoning.push_str(chunk),
            )
            .expect("stream");
        assert_eq!(reasoning, "plan first");
        assert_eq!(text, "int main");
        assert_eq!(streamed, text);
    }

    #[test]
    fn describe_names_the_selected_preset() {
        let mut settings = ProviderSettings {
            endpoint: DEEPSEEK_ENDPOINT.to_owned(),
            ..ProviderSettings::default()
        };
        assert!(settings.describe().contains("provider=deepseek"));
        settings.endpoint = OPENCODE_GO_ENDPOINT.to_owned();
        assert!(settings.describe().contains("provider=opencode-go"));
        settings.endpoint = "https://example.test".to_owned();
        assert!(settings.describe().contains("provider=openai-compatible"));
    }

    #[test]
    fn default_settings_are_unconfigured() {
        let settings = ProviderSettings::default();
        assert_eq!(settings.kind, ProviderKind::OpenAiCompatible);
        assert!(!settings.enabled);
        assert!(settings.describe().contains("state=disabled"));
    }

    #[test]
    fn prompts_switch_language_and_take_overrides() {
        let mut settings = ProviderSettings::default();
        assert_eq!(
            system_prompt(&settings, PromptKind::Agent),
            crate::AGENT_SYSTEM
        );
        settings.prompt_language = PromptLanguage::Zh;
        assert_eq!(
            system_prompt(&settings, PromptKind::Agent),
            crate::AGENT_SYSTEM_ZH
        );

        // 覆盖文本只替换它写入时对应的语言。
        settings.set_prompt_override(PromptKind::Agent, Some("自定义".to_owned()));
        assert_eq!(system_prompt(&settings, PromptKind::Agent), "自定义");
        settings.prompt_language = PromptLanguage::En;
        assert_eq!(
            system_prompt(&settings, PromptKind::Agent),
            crate::AGENT_SYSTEM
        );
        // 去掉空白后为空的文本等于清除覆盖，内置提示词重新生效。
        settings.prompt_language = PromptLanguage::Zh;
        settings.set_prompt_override(PromptKind::Agent, Some("   ".to_owned()));
        assert_eq!(
            system_prompt(&settings, PromptKind::Agent),
            crate::AGENT_SYSTEM_ZH
        );
    }

    #[test]
    fn prompt_edits_roundtrip_through_settings_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ai-settings.json");
        let mut settings = ProviderSettings {
            prompt_language: PromptLanguage::Zh,
            ..ProviderSettings::default()
        };
        settings.set_prompt_override(PromptKind::Agent, Some("自定义智能体提示".to_owned()));
        settings.save_to(&path).unwrap();

        let loaded = ProviderSettings::load_from(&path).expect("settings reload");
        assert_eq!(loaded.prompt_language, PromptLanguage::Zh);
        assert_eq!(
            system_prompt(&loaded, PromptKind::Agent),
            "自定义智能体提示"
        );
        assert!(
            loaded.prompt_overrides.contains_key("zh:agent"),
            "{:?}",
            loaded.prompt_overrides
        );
    }

    /// 旧版设置文件里的 `"en:agent"` 覆盖必须继续生效（键名格式没有变）。
    #[test]
    fn legacy_agent_prompt_override_still_loads() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ai-settings.json");
        std::fs::write(
            &path,
            r#"{"kind":"openai","endpoint":"https://api.deepseek.com","prompts":{"en:agent":"legacy agent prompt","en:polish":"retired kind"}}"#,
        )
        .unwrap();
        let loaded = ProviderSettings::load_from(&path).expect("settings reload");
        assert_eq!(
            system_prompt(&loaded, PromptKind::Agent),
            "legacy agent prompt"
        );
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
    fn retired_deepseek_models_migrate_on_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ai-settings.json");
        std::fs::write(
            &path,
            r#"{"kind":"openai","endpoint":"https://api.deepseek.com","model":"deepseek-chat","api_key_env":"RECRAFT_AI_API_KEY","max_tokens":65536}"#,
        )
        .unwrap();
        let loaded = ProviderSettings::load_from(&path).expect("settings reload");
        assert_eq!(loaded.endpoint, DEEPSEEK_ENDPOINT);
        assert_eq!(loaded.model, "deepseek-flash");

        // A custom endpoint may legitimately keep a model id we do not know about.
        std::fs::write(
            &path,
            r#"{"kind":"openai","endpoint":"https://gateway.example/v1","model":"deepseek-chat"}"#,
        )
        .unwrap();
        let loaded = ProviderSettings::load_from(&path).expect("settings reload");
        assert_eq!(loaded.model, "deepseek-chat");
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
    fn budget_truncates_long_prompts() {
        let budget = TokenBudget::new(32);
        let long = "a".repeat(10_000);
        let fitted = budget.fit(&long);
        assert!(TokenBudget::estimate(&fitted) <= budget.max_tokens + 8);
        assert!(fitted.ends_with("…[truncated]"));
    }

    #[test]
    fn unavailable_provider_reports_missing_configuration() {
        let request = PromptRequest::new(TokenBudget::new(64), "system", "user");
        let error = UnavailableProvider.complete(&request).unwrap_err();
        assert!(error.to_string().contains("no LLM provider configured"));
    }

    fn test_settings(kind: ProviderKind, endpoint: &str) -> ProviderSettings {
        ProviderSettings {
            kind,
            endpoint: endpoint.to_owned(),
            model: "test-model".to_owned(),
            api_key_env: "RECRAFT_AI_API_KEY".to_owned(),
            api_key: None,
            saved_api_keys: BTreeMap::new(),
            max_tokens: 256,
            enabled: true,
            ..ProviderSettings::default()
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
        assert_eq!(settings.model, "deepseek-flash");
        assert_eq!(
            ProviderPreset::DeepSeek.models(),
            ["deepseek-flash", "deepseek-v4-pro"].as_slice(),
            "only the live DeepSeek lineup is offered"
        );
        assert!(
            !ProviderPreset::DeepSeek
                .models()
                .iter()
                .any(|model| model.contains("chat") || model.contains("reasoner")),
            "retired DeepSeek ids must not be offered"
        );

        ProviderPreset::OpenCodeGo.apply(&mut settings);
        assert_eq!(settings.endpoint, OPENCODE_GO_ENDPOINT);
        assert_eq!(settings.model, "deepseek-v4.1-flash");
        assert!(
            settings.max_tokens >= 65536,
            "reasoning models need a large token budget"
        );
    }

    #[test]
    fn saved_api_keys_roundtrip_per_provider() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ai-settings.json");
        let mut settings = ProviderSettings {
            endpoint: DEEPSEEK_ENDPOINT.to_owned(),
            model: "deepseek-flash".to_owned(),
            api_key: Some("deepseek-secret".to_owned()),
            ..ProviderSettings::default()
        };
        settings.remember_current_key();
        settings.endpoint = OPENCODE_GO_ENDPOINT.to_owned();
        settings.model = "glm-5.3-flash".to_owned();
        settings.api_key = Some("opencode-secret".to_owned());
        settings.remember_current_key();
        settings.save_to(&path).unwrap();

        let loaded = ProviderSettings::load_from(&path).expect("settings reload");
        assert_eq!(
            loaded.saved_api_keys.get("deepseek").map(String::as_str),
            Some("deepseek-secret")
        );
        assert_eq!(
            loaded.saved_api_keys.get("opencode-go").map(String::as_str),
            Some("opencode-secret")
        );
    }

    #[test]
    fn api_key_is_sealed_and_roundtrips() {
        let mut settings = ProviderSettings {
            kind: ProviderKind::OpenAiCompatible,
            endpoint: DEEPSEEK_ENDPOINT.to_owned(),
            model: "deepseek-flash".to_owned(),
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
            "the key must not be stored in plaintext: {text}"
        );
        assert!(text.contains("api_key_sealed"), "{text}");
        let loaded = ProviderSettings::load_from(&path).expect("settings reload");
        assert_eq!(loaded.api_key.as_deref(), Some("sk-session-secret"));
        assert!(loaded.api_key_present());
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
    fn disabled_settings_select_the_unavailable_provider() {
        let provider = provider_from_settings(&ProviderSettings::default());
        assert_eq!(provider.name(), "unconfigured");
    }

    #[test]
    fn parse_tool_reply_reads_calls_and_text() {
        let calls = parse_tool_reply(
            r#"{"choices":[{"message":{"content":null,"tool_calls":[
                {"id":"call_a","type":"function","function":{"name":"list_functions","arguments":"{\"limit\":5}"}}
            ]}}]}"#,
        )
        .unwrap();
        match calls {
            ToolReply::Calls(calls) => {
                assert_eq!(calls.len(), 1);
                assert_eq!(calls[0].id, "call_a");
                assert_eq!(calls[0].name, "list_functions");
                assert_eq!(calls[0].arguments["limit"], 5);
            }
            ToolReply::Text(text) => panic!("expected calls, got {text:?}"),
        }

        // A missing id still yields a usable pairing id for the tool result.
        let no_id = parse_tool_reply(
            r#"{"choices":[{"message":{"tool_calls":[
                {"type":"function","function":{"name":"x","arguments":{}}}
            ]}}]}"#,
        )
        .unwrap();
        match no_id {
            ToolReply::Calls(calls) => assert_eq!(calls[0].id, "call_0"),
            ToolReply::Text(text) => panic!("expected calls, got {text:?}"),
        }

        match parse_tool_reply(r#"{"choices":[{"message":{"content":"hello"}}]}"#).unwrap() {
            ToolReply::Text(text) => assert_eq!(text, "hello"),
            ToolReply::Calls(_) => panic!("expected text"),
        }
        assert!(parse_tool_reply("{}").is_err());
    }
    #[test]
    fn tool_stream_decoder_accumulates_content_and_call_fragments() {
        use serde_json::json;
        let mut decoder = ToolStreamDecoder::default();
        let mut text = String::new();
        let sse = [
            json!({"choices":[{"delta":{"content":"Hello "}}]}).to_string(),
            json!({"choices":[{"delta":{"content":"world"}}]}).to_string(),
            "[DONE]".to_owned(),
        ]
        .iter()
        .map(|line| format!("data: {line}\n"))
        .collect::<String>();
        decoder.push(&sse, &mut |chunk| text.push_str(chunk));
        assert_eq!(text, "Hello world");
        match decoder.finish().unwrap() {
            ToolReply::Text(answer) => assert_eq!(answer, "Hello world"),
            ToolReply::Calls(calls) => panic!("expected text, got {calls:?}"),
        }

        // Two calls whose argument JSON arrives split across deltas.
        let mut decoder = ToolStreamDecoder::default();
        let sse = [
            json!({"choices":[{"delta":{"tool_calls":[
                {"index":0,"id":"a","function":{"name":"list_functions","arguments":"{\"lim"}}
            ]}}]})
            .to_string(),
            json!({"choices":[{"delta":{"tool_calls":[
                {"index":0,"function":{"arguments":"it\":5}"}}
            ]}}]})
            .to_string(),
            json!({"choices":[{"delta":{"tool_calls":[
                {"index":1,"id":"b","function":{"name":"pseudocode","arguments":"{}"}}
            ]}}]})
            .to_string(),
            "[DONE]".to_owned(),
        ]
        .iter()
        .map(|line| format!("data: {line}\n"))
        .collect::<String>();
        decoder.push(&sse, &mut |_| {});
        match decoder.finish().unwrap() {
            ToolReply::Calls(calls) => {
                assert_eq!(calls.len(), 2);
                assert_eq!(calls[0].name, "list_functions");
                assert_eq!(calls[0].arguments["limit"], 5);
                assert_eq!(calls[1].id, "b");
            }
            ToolReply::Text(text) => panic!("expected calls, got {text:?}"),
        }

        // A gateway that ignores stream:true returns one plain JSON body.
        let mut decoder = ToolStreamDecoder::default();
        decoder.push(
            r#"{"choices":[{"message":{"content":"one shot"}}]}"#,
            &mut |_| {},
        );
        match decoder.finish().unwrap() {
            ToolReply::Text(text) => assert_eq!(text, "one shot"),
            ToolReply::Calls(_) => panic!("expected text"),
        }
    }
}
