// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Persistent LLM response cache + append-only audit log (DESIGN §9.3).
//!
//! Both live under `%LOCALAPPDATA%\ReCraft` (or the temp dir when that is unset):
//!
//! - `ai-cache.json` maps a stable prompt key (template + provider + model + prompt
//!   text) to the model response, so repeated explanations do not hit the network.
//! - `ai-audit.jsonl` appends one JSON line per provider interaction (provider, model,
//!   template, function, token counts, cache hit) for traceability.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use recraft_common::{Address, Error, Result};
use serde_json::{Value, json};

use crate::PromptRequest;

/// Cache file name under the ReCraft data directory.
pub const CACHE_FILE: &str = "ai-cache.json";
/// Audit file name under the ReCraft data directory.
pub const AUDIT_FILE: &str = "ai-audit.jsonl";

/// One cached model response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheEntry {
    pub response: String,
    pub provider: String,
    pub model: String,
    pub template: String,
    pub created_unix: u64,
}

/// File-backed prompt response cache.
#[derive(Debug)]
pub struct AiCache {
    path: PathBuf,
    entries: BTreeMap<String, CacheEntry>,
}

impl AiCache {
    /// Open the cache file (missing file means an empty cache).
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let entries = match std::fs::read_to_string(&path) {
            Ok(text) => parse_entries(&text)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => BTreeMap::new(),
            Err(error) => return Err(Error::from(error)),
        };
        Ok(AiCache { path, entries })
    }

    /// Open the default per-user cache.
    pub fn open_default() -> Result<Self> {
        Self::open(default_dir().join(CACHE_FILE))
    }

    /// Stable key: template + provider + model + both prompt halves.
    pub fn key(request: &PromptRequest, provider: &str, model: &str, template: &str) -> String {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        template.hash(&mut hasher);
        provider.hash(&mut hasher);
        model.hash(&mut hasher);
        request.system.hash(&mut hasher);
        request.user.hash(&mut hasher);
        request.thinking.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    pub fn get(&self, key: &str) -> Option<&CacheEntry> {
        self.entries.get(key)
    }

    pub fn insert(&mut self, key: String, entry: CacheEntry) {
        self.entries.insert(key, entry);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Persist the cache atomically-ish (write then rename is unnecessary for a cache;
    /// a truncated file just means a cold cache next run).
    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut object = serde_json::Map::new();
        for (key, entry) in &self.entries {
            object.insert(
                key.clone(),
                json!({
                    "response": entry.response,
                    "provider": entry.provider,
                    "model": entry.model,
                    "template": entry.template,
                    "created_unix": entry.created_unix,
                }),
            );
        }
        let text = serde_json::to_string_pretty(&Value::Object(object))
            .map_err(|error| Error::internal(format!("serialize ai cache: {error}")))?;
        std::fs::write(&self.path, text)?;
        Ok(())
    }
}

fn parse_entries(text: &str) -> Result<BTreeMap<String, CacheEntry>> {
    let value: Value = serde_json::from_str(text)
        .map_err(|error| Error::internal(format!("parse ai cache: {error}")))?;
    let Some(object) = value.as_object() else {
        return Ok(BTreeMap::new());
    };
    let mut entries = BTreeMap::new();
    for (key, item) in object {
        let Some(response) = item.get("response").and_then(Value::as_str) else {
            continue;
        };
        entries.insert(
            key.clone(),
            CacheEntry {
                response: response.to_owned(),
                provider: item
                    .get("provider")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned(),
                model: item
                    .get("model")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned(),
                template: item
                    .get("template")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned(),
                created_unix: item
                    .get("created_unix")
                    .and_then(Value::as_u64)
                    .unwrap_or(0),
            },
        );
    }
    Ok(entries)
}

/// One audit line (written as JSONL).
#[derive(Debug, Clone)]
pub struct AuditRecord {
    pub provider: String,
    pub model: String,
    pub template: String,
    pub function: Option<Address>,
    pub prompt_tokens: usize,
    pub response_tokens: usize,
    pub cached: bool,
}

impl AuditRecord {
    pub fn to_json(&self) -> Value {
        json!({
            "ts": unix_now(),
            "provider": self.provider,
            "model": self.model,
            "template": self.template,
            "function": self.function.map(|addr| format!("{:#x}", addr.get())),
            "prompt_tokens": self.prompt_tokens,
            "response_tokens": self.response_tokens,
            "cached": self.cached,
        })
    }
}

/// Append one audit record as a JSON line.
pub fn append_audit(path: &Path, record: &AuditRecord) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let line = serde_json::to_string(&record.to_json())
        .map_err(|error| Error::internal(format!("serialize audit: {error}")))?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{line}")?;
    Ok(())
}

/// Append to the default audit log.
pub fn append_default_audit(record: &AuditRecord) -> Result<()> {
    append_audit(&default_dir().join(AUDIT_FILE), record)
}

/// Per-user data directory for AI artefacts (`%LOCALAPPDATA%\ReCraft`).
pub fn default_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join("ReCraft")
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TokenBudget;

    fn entry(provider: &str, model: &str) -> CacheEntry {
        CacheEntry {
            response: "explained".to_owned(),
            provider: provider.to_owned(),
            model: model.to_owned(),
            template: crate::PROMPT_TEMPLATE_VERSION.to_owned(),
            created_unix: 1,
        }
    }

    #[test]
    fn cache_roundtrips_through_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(CACHE_FILE);
        let mut cache = AiCache::open(&path).unwrap();
        assert!(cache.is_empty());
        let request = PromptRequest::new(TokenBudget::new(128), "system", "user");
        let key = AiCache::key(
            &request,
            "ollama",
            "model-a",
            crate::PROMPT_TEMPLATE_VERSION,
        );
        cache.insert(key.clone(), entry("ollama", "model-a"));
        cache.save().unwrap();

        let reopened = AiCache::open(&path).unwrap();
        let hit = reopened.get(&key).expect("cache hit after reopen");
        assert_eq!(hit.response, "explained");
        assert_eq!(hit.provider, "ollama");
    }

    #[test]
    fn cache_key_depends_on_provider_model_template_and_prompt() {
        let request = PromptRequest::new(TokenBudget::new(128), "system", "user");
        let base = AiCache::key(&request, "ollama", "m", "t");
        assert_ne!(base, AiCache::key(&request, "openai", "m", "t"));
        assert_ne!(base, AiCache::key(&request, "ollama", "m2", "t"));
        assert_ne!(base, AiCache::key(&request, "ollama", "m", "t2"));
        let other = PromptRequest::new(TokenBudget::new(128), "system", "other");
        assert_ne!(base, AiCache::key(&other, "ollama", "m", "t"));
    }

    #[test]
    fn audit_log_appends_json_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(AUDIT_FILE);
        let record = AuditRecord {
            provider: "ollama".to_owned(),
            model: "qwen".to_owned(),
            template: crate::PROMPT_TEMPLATE_VERSION.to_owned(),
            function: Some(Address::new(0x140001000)),
            prompt_tokens: 120,
            response_tokens: 40,
            cached: false,
        };
        append_audit(&path, &record).unwrap();
        append_audit(&path, &record).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        let value: Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(value["provider"], "ollama");
        assert_eq!(value["model"], "qwen");
        assert_eq!(value["function"], "0x140001000");
        assert_eq!(value["cached"], false);
        assert!(value["ts"].as_u64().unwrap() > 0);
    }

    #[test]
    fn default_dir_is_absolute() {
        let dir = default_dir();
        assert!(dir.is_absolute(), "{dir:?}");
        assert!(dir.ends_with("ReCraft"));
    }
}
