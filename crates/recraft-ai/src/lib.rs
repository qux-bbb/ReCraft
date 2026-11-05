// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! AI assist layer (DESIGN §9.2/§9.3).
//!
//! Evidence gathering is deterministic and offline; only the provider layer touches the
//! network:
//!
//! - `explain`  – function explanation built from API tags, referenced strings and call
//!   edges (no network, no model).
//! - `suggest_names` – rename **Proposals** for `sub_*` functions, each with evidence
//!   addresses, a confidence and a rationale.
//! - `triage` – sample summary: API categories, suspicious functions, URLs/strings.
//! - `apply` – the only way a proposal reaches the project; it goes through the
//!   `Command` bus (journaled, undoable). Nothing is ever changed silently.
//!
//! The same evidence feeds the provider prompts (`build_explain_prompt`,
//! `build_rename_prompt`, `build_polish_prompt`); model replies are parsed back into the
//! same `Proposal` shape (`apply_rename_reply`), so confirm → apply stays identical.

// DPAPI is the one OS FFI exception; it is isolated in `secret`.
#![deny(unsafe_code)]

mod cache;
mod eval;
mod provider;
mod query;
#[allow(unsafe_code)]
mod secret;
mod transport;

pub use cache::{
    AUDIT_FILE, AiCache, AuditRecord, CACHE_FILE, CacheEntry, append_audit, append_default_audit,
    default_dir,
};
pub use eval::{CORPUS_CASES, EVAL_VERSION, EvalCase, EvalReport, ensure_eval, evaluate_explain};
pub use provider::{
    AuditEntry, AuditLog, DEEPSEEK_ENDPOINT, DEEPSEEK_MODELS, EXPLAIN_SYSTEM, OPENCODE_GO_ENDPOINT,
    OPENCODE_GO_MODELS, OllamaProvider, OpenAiCompatibleProvider, POLISH_SYSTEM,
    PROMPT_TEMPLATE_VERSION, PromptCache, PromptRequest, Provider, ProviderKind, ProviderPreset,
    ProviderSettings, RENAME_SYSTEM, RenameSuggestion, TokenBudget, UnavailableProvider,
    apply_rename_reply, build_explain_prompt, build_polish_prompt, build_rename_prompt,
    parse_rename_reply, provider_from_settings, redact,
};
pub use query::{QueryAnswer, QueryItem, answer_query};
pub use transport::{
    HttpRequest, HttpResponse, MockTransport, TcpHttpTransport, Transport, TransportKind,
    UreqTransport, parse_ollama_response, parse_openai_response, transport_for, transport_kind,
};

use std::collections::{BTreeMap, BTreeSet};

use recraft_common::{Address, Error, Result};
use recraft_core::{Command, CommentKind, Session};
use serde_json::{Value, json};

const XREF_CALL: i64 = 1;
const XREF_DATA: i64 = 3;
const MAX_SUGGESTIONS: usize = 200;
const MAX_SUSPICIOUS: usize = 10;
/// Suspiciousness weights for the API categories produced by `recraft-analysis`.
const WEIGHTS: [(&str, f32); 12] = [
    ("injection", 4.0),
    ("anti-analysis", 3.0),
    ("service", 2.5),
    ("registry", 2.0),
    ("process", 2.0),
    ("thread", 1.5),
    ("network", 1.5),
    ("crypto", 1.2),
    ("memory", 1.0),
    ("file", 1.0),
    ("ui", 0.5),
    ("other", 0.2),
];

// --------------------------------------------------------------- proposals ----

/// What a proposal asks the user to change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalKind {
    Rename,
    Comment,
    Prototype,
    /// Bind a struct layout to a function argument; `value` is `"<slot>:<Type>"`.
    ApplyType,
}

impl ProposalKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            ProposalKind::Rename => "rename",
            ProposalKind::Comment => "comment",
            ProposalKind::Prototype => "prototype",
            ProposalKind::ApplyType => "apply_type",
        }
    }

    fn from_str(text: &str) -> Option<Self> {
        match text {
            "rename" => Some(ProposalKind::Rename),
            "comment" => Some(ProposalKind::Comment),
            "prototype" => Some(ProposalKind::Prototype),
            "apply_type" => Some(ProposalKind::ApplyType),
            _ => None,
        }
    }
}

/// One piece of evidence backing a proposal; every entry points at a real address.
#[derive(Debug, Clone)]
pub struct Evidence {
    pub addr: Address,
    /// `api_tag`, `string`, `call` or `evidence`.
    pub kind: &'static str,
    pub text: String,
}

/// A proposed user-data change. LLM/heuristics only ever produce these; applying is an
/// explicit, journaled step.
#[derive(Debug, Clone)]
pub struct Proposal {
    pub kind: ProposalKind,
    pub target: Address,
    pub value: String,
    pub confidence: f32,
    pub rationale: String,
    pub evidence: Vec<Evidence>,
}

impl Proposal {
    pub fn to_json(&self) -> Value {
        json!({
            "kind": self.kind.as_str(),
            "target": format!("{:#x}", self.target.get()),
            "value": self.value,
            "confidence": self.confidence,
            "rationale": self.rationale,
            "evidence": self.evidence.iter().map(|evidence| json!({
                "addr": format!("{:#x}", evidence.addr.get()),
                "kind": evidence.kind,
                "text": evidence.text,
            })).collect::<Vec<_>>(),
        })
    }

    pub fn from_json(value: &Value) -> Result<Self> {
        let kind = value
            .get("kind")
            .and_then(Value::as_str)
            .and_then(ProposalKind::from_str)
            .ok_or_else(|| Error::internal("proposal: missing/invalid kind"))?;
        let target = value
            .get("target")
            .and_then(Value::as_str)
            .and_then(parse_address)
            .ok_or_else(|| Error::internal("proposal: missing/invalid target"))?;
        let value_text = value
            .get("value")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::internal("proposal: missing value"))?
            .to_owned();
        let confidence = value
            .get("confidence")
            .and_then(Value::as_f64)
            .unwrap_or(0.0) as f32;
        let rationale = value
            .get("rationale")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        let mut evidence = Vec::new();
        if let Some(items) = value.get("evidence").and_then(Value::as_array) {
            for item in items {
                let Some(addr) = item
                    .get("addr")
                    .and_then(Value::as_str)
                    .and_then(parse_address)
                else {
                    continue;
                };
                evidence.push(Evidence {
                    addr,
                    kind: "evidence",
                    text: item
                        .get("text")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_owned(),
                });
            }
        }
        Ok(Proposal {
            kind,
            target,
            value: value_text,
            confidence,
            rationale,
            evidence,
        })
    }

    /// Apply through the command bus (journaled/undoable). Never called implicitly.
    pub fn apply(&self, session: &mut Session) -> Result<()> {
        match self.kind {
            ProposalKind::Rename => {
                let name = if self.value.trim().is_empty() {
                    None
                } else {
                    Some(self.value.clone())
                };
                session.execute(Command::Rename {
                    addr: self.target,
                    name,
                })?;
                Ok(())
            }
            ProposalKind::Comment => {
                session.execute(Command::SetComment {
                    addr: self.target,
                    kind: CommentKind::Regular,
                    text: Some(self.value.clone()),
                })?;
                Ok(())
            }
            ProposalKind::Prototype => {
                let text = if self.value.trim().is_empty() {
                    None
                } else {
                    Some(self.value.clone())
                };
                session.execute(Command::SetPrototype {
                    addr: self.target,
                    text,
                })?;
                Ok(())
            }
            ProposalKind::ApplyType => {
                let (slot, type_name) = self.value.split_once(':').ok_or_else(|| {
                    Error::internal("apply_type proposal value must be \"<slot>:<Type>\"")
                })?;
                let slot = slot.trim().parse::<u8>().map_err(|_| {
                    Error::internal(format!("apply_type proposal has invalid slot {slot:?}"))
                })?;
                let type_name = type_name.trim();
                session.execute(Command::ApplyType {
                    func: self.target,
                    slot,
                    struct_name: (!type_name.is_empty()).then(|| type_name.to_owned()),
                })?;
                Ok(())
            }
        }
    }
}

/// Apply a reviewed list of proposals; returns how many commands were executed.
pub fn apply_proposals(session: &mut Session, proposals: &[Proposal]) -> Result<usize> {
    let mut applied = 0;
    for proposal in proposals {
        proposal.apply(session)?;
        applied += 1;
    }
    Ok(applied)
}

fn parse_address(text: &str) -> Option<Address> {
    let hex = text.trim();
    let digits = hex
        .strip_prefix("0x")
        .or_else(|| hex.strip_prefix("0X"))
        .unwrap_or(hex);
    u64::from_str_radix(digits, 16).ok().map(Address::new)
}

// ------------------------------------------------------------- explanation ----

/// Deterministic explanation of one function.
#[derive(Debug, Clone)]
pub struct Explanation {
    pub name: Option<String>,
    pub start: Address,
    pub end: Address,
    pub instructions: usize,
    /// `(call site, API name, category)`.
    pub apis: Vec<(Address, String, String)>,
    /// `(address, text)` of referenced strings.
    pub strings: Vec<(Address, String)>,
    /// `(call site, callee name/address)`.
    pub calls: Vec<(Address, String)>,
    pub summary: String,
}

pub fn explain(session: &mut Session, addr: Address) -> Result<Explanation> {
    let functions = session.functions()?;
    let Some(function) = functions
        .iter()
        .find(|function| addr >= function.start && addr < function.end)
    else {
        return Err(Error::internal(format!(
            "no function contains {:#x}",
            addr.get()
        )));
    };
    let name = session.name_at(function.start)?;
    let mut instructions = 0usize;
    let mut cursor = function.start;
    while cursor < function.end {
        let Some(insn) = session.insn_at(cursor)? else {
            break;
        };
        instructions += 1;
        let next = insn.next_addr();
        if next <= cursor {
            break;
        }
        cursor = next;
    }

    let apis: Vec<(Address, String, String)> = session
        .all_api_tags()?
        .into_iter()
        .filter(|(site, _, _)| *site >= function.start && *site < function.end)
        .collect();
    let strings = referenced_strings(session, function.start, function.end)?;
    let calls = outgoing_calls(session, function.start, function.end)?;

    let mut parts = Vec::new();
    parts.push(match &name {
        Some(name) => format!(
            "{name} ({:x}..{:x})",
            function.start.get(),
            function.end.get()
        ),
        None => format!("sub_{:x}", function.start.get()),
    });
    parts.push(format!("{instructions} instructions"));
    if !apis.is_empty() {
        let mut seen = BTreeSet::new();
        let api_names: Vec<String> = apis
            .iter()
            .filter_map(|(_, rule, _)| {
                if seen.insert(rule.clone()) {
                    Some(rule.clone())
                } else {
                    None
                }
            })
            .collect();
        parts.push(format!("calls {}", api_names.join(", ")));
    }
    if !strings.is_empty() {
        parts.push(format!("references {} string(s)", strings.len()));
    }
    if !calls.is_empty() {
        parts.push(format!("makes {} direct call(s)", calls.len()));
    }
    let categories: BTreeSet<String> = apis
        .iter()
        .map(|(_, _, category)| category.clone())
        .collect();
    if !categories.is_empty() {
        parts.push(format!(
            "categories: {}",
            categories.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
    let summary = parts.join("; ");

    Ok(Explanation {
        name,
        start: function.start,
        end: function.end,
        instructions,
        apis,
        strings,
        calls,
        summary,
    })
}

// ------------------------------------------------------------ name suggest ----

/// Propose names for `sub_*` functions based on API categories and referenced strings.
pub fn suggest_names(session: &mut Session, limit: usize) -> Result<Vec<Proposal>> {
    let limit = limit.clamp(1, MAX_SUGGESTIONS);
    let functions = session.functions()?;
    let api_tags = session.all_api_tags()?;
    // Preload once: per-function lookups over all strings/xrefs would be quadratic.
    let string_entries = session.strings()?;
    let string_index: BTreeMap<u64, (u32, i64)> = string_entries
        .iter()
        .map(|entry| (entry.addr.get(), (entry.len, entry.enc)))
        .collect();
    let data_refs = session.all_xrefs(Some(XREF_DATA))?;
    let mut proposals = Vec::new();
    for function in &functions {
        if proposals.len() >= limit {
            break;
        }
        let current = session.name_at(function.start)?;
        if current
            .as_deref()
            .is_some_and(|name| !name.starts_with("sub_"))
        {
            continue;
        }
        let apis: Vec<(Address, String, String)> = api_tags
            .iter()
            .filter(|(site, _, _)| *site >= function.start && *site < function.end)
            .cloned()
            .collect();
        let strings = strings_in_range(
            session,
            &string_index,
            &data_refs,
            function.start,
            function.end,
        );
        if apis.is_empty() && strings.is_empty() {
            continue;
        }
        let categories: BTreeSet<String> = apis
            .iter()
            .map(|(_, _, category)| category.clone())
            .collect();
        let prefix = categories
            .iter()
            .find_map(|category| match category.as_str() {
                "injection" => Some("inject"),
                "anti-analysis" => Some("anti_debug"),
                "network" => Some("network"),
                "memory" => Some("memory"),
                "file" => Some("file"),
                "process" => Some("process"),
                "thread" => Some("thread"),
                "registry" => Some("registry"),
                "service" => Some("service"),
                "crypto" => Some("crypto"),
                "callback" => Some("callback"),
                "ui" => Some("ui"),
                _ => None,
            })
            .unwrap_or("interesting");
        let value = format!("{prefix}_{:x}", function.start.get());
        let mut evidence: Vec<Evidence> = apis
            .iter()
            .map(|(site, rule, _)| Evidence {
                addr: *site,
                kind: "api_tag",
                text: rule.clone(),
            })
            .collect();
        evidence.extend(strings.iter().map(|(addr, text)| Evidence {
            addr: *addr,
            kind: "string",
            text: text.clone(),
        }));
        let confidence = (0.35 + 0.15 * evidence.len() as f32).min(0.9);
        let api_names: Vec<String> = apis
            .iter()
            .map(|(_, rule, _)| rule.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let rationale = if api_names.is_empty() {
            format!("references {} string(s)", strings.len())
        } else {
            format!(
                "calls {}; {} string ref(s)",
                api_names.join(", "),
                strings.len()
            )
        };
        proposals.push(Proposal {
            kind: ProposalKind::Rename,
            target: function.start,
            value,
            confidence,
            rationale,
            evidence,
        });
    }
    Ok(proposals)
}

// ------------------------------------------------------------------ triage ----

/// Sample-level triage summary.
#[derive(Debug, Clone)]
pub struct Triage {
    pub functions: usize,
    pub instructions: u64,
    pub apis_by_category: BTreeMap<String, usize>,
    pub suspicious: Vec<SuspiciousFunction>,
    pub urls: Vec<String>,
    pub strings: usize,
}

#[derive(Debug, Clone)]
pub struct SuspiciousFunction {
    pub name: Option<String>,
    pub start: Address,
    pub categories: Vec<String>,
    pub score: f32,
    pub apis: Vec<String>,
}

pub fn triage(session: &mut Session) -> Result<Triage> {
    let functions = session.functions()?;
    let api_tags = session.all_api_tags()?;
    let strings = session.strings()?;

    let mut apis_by_category: BTreeMap<String, usize> = BTreeMap::new();
    for (_, _, category) in &api_tags {
        *apis_by_category.entry(category.clone()).or_insert(0) += 1;
    }

    let mut per_function: BTreeMap<u64, (BTreeSet<String>, BTreeSet<String>, f32)> =
        BTreeMap::new();
    for (site, rule, category) in &api_tags {
        let Some(function) = functions
            .iter()
            .find(|function| *site >= function.start && *site < function.end)
        else {
            continue;
        };
        let weight = WEIGHTS
            .iter()
            .find(|(name, _)| name == category)
            .map(|(_, weight)| *weight)
            .unwrap_or(0.5);
        let entry = per_function
            .entry(function.start.get())
            .or_insert_with(|| (BTreeSet::new(), BTreeSet::new(), 0.0));
        entry.0.insert(category.clone());
        entry.1.insert(rule.clone());
        entry.2 += weight;
    }
    let mut suspicious: Vec<SuspiciousFunction> = per_function
        .into_iter()
        .map(|(start, (categories, apis, score))| SuspiciousFunction {
            name: session.name_at(Address::new(start)).ok().flatten(),
            start: Address::new(start),
            categories: categories.into_iter().collect(),
            score,
            apis: apis.into_iter().collect(),
        })
        .collect();
    suspicious.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.start.cmp(&right.start))
    });
    suspicious.truncate(MAX_SUSPICIOUS);

    let mut instruction_total = 0u64;
    for function in &functions {
        let mut cursor = function.start;
        while cursor < function.end {
            let Ok(Some(insn)) = session.insn_at(cursor) else {
                break;
            };
            instruction_total += 1;
            let next = insn.next_addr();
            if next <= cursor {
                break;
            }
            cursor = next;
        }
    }

    let mut urls = Vec::new();
    let mut seen_urls = BTreeSet::new();
    for entry in &strings {
        let text = read_string(session, entry.addr, entry.len, entry.enc);
        let lower = text.to_ascii_lowercase();
        if (lower.starts_with("http://") || lower.starts_with("https://"))
            && seen_urls.insert(text.clone())
        {
            urls.push(text);
            if urls.len() >= 20 {
                break;
            }
        }
    }

    Ok(Triage {
        functions: functions.len(),
        instructions: instruction_total,
        apis_by_category,
        suspicious,
        urls,
        strings: strings.len(),
    })
}

// ---------------------------------------------------------------- helpers ----

fn referenced_strings(
    session: &mut Session,
    start: Address,
    end: Address,
) -> Result<Vec<(Address, String)>> {
    let strings = session.strings()?;
    let index: BTreeMap<u64, (u32, i64)> = strings
        .iter()
        .map(|entry| (entry.addr.get(), (entry.len, entry.enc)))
        .collect();
    let data_refs = session.all_xrefs(Some(XREF_DATA))?;
    Ok(strings_in_range(session, &index, &data_refs, start, end))
}

/// Strings referenced from code inside `[start, end)` given preloaded tables.
fn strings_in_range(
    session: &mut Session,
    string_index: &BTreeMap<u64, (u32, i64)>,
    data_refs: &[recraft_core::XrefEntry],
    start: Address,
    end: Address,
) -> Vec<(Address, String)> {
    let mut seen = BTreeSet::new();
    let mut found = Vec::new();
    for xref in data_refs {
        if xref.from < start || xref.from >= end || !seen.insert(xref.to.get()) {
            continue;
        }
        if let Some((len, enc)) = string_index.get(&xref.to.get()) {
            let text = read_string(session, xref.to, *len, *enc);
            found.push((xref.to, text));
            if found.len() >= 32 {
                break;
            }
        }
    }
    found
}

fn outgoing_calls(
    session: &mut Session,
    start: Address,
    end: Address,
) -> Result<Vec<(Address, String)>> {
    let calls = session.all_xrefs(Some(XREF_CALL))?;
    let mut seen = BTreeSet::new();
    let mut found = Vec::new();
    for call in calls {
        if call.from < start || call.from >= end || !seen.insert(call.to.get()) {
            continue;
        }
        let name = session
            .name_at(call.to)?
            .unwrap_or_else(|| format!("sub_{:x}", call.to.get()));
        found.push((call.from, name));
        if found.len() >= 32 {
            break;
        }
    }
    Ok(found)
}

fn read_string(session: &mut Session, addr: Address, len: u32, encoding: i64) -> String {
    let bytes = session
        .read_bytes(addr, (len as usize).max(2))
        .unwrap_or_default();
    if encoding == 2 {
        let mut end = bytes.len();
        while end >= 2 && bytes[end - 2] == 0 && bytes[end - 1] == 0 {
            end -= 2;
        }
        let units: Vec<u16> = bytes[..end]
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        String::from_utf16_lossy(&units)
    } else {
        let mut end = bytes.len();
        while end > 0 && bytes[end - 1] == 0 {
            end -= 1;
        }
        String::from_utf8_lossy(&bytes[..end]).into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notepad() -> Option<(Session, tempfile::TempDir)> {
        let bytes = std::fs::read("C:/Windows/System32/notepad.exe").ok()?;
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("ai.rcraft");
        let mut session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let result = recraft_analysis::analyze(&mut session).unwrap();
        recraft_analysis::persist(&mut session, &result).unwrap();
        let strings = recraft_analysis::extract_strings(&mut session, 4, 200_000).unwrap();
        recraft_analysis::persist_strings(&mut session, &strings).unwrap();
        Some((session, dir))
    }

    #[test]
    fn explain_entry_function() {
        let Some((mut session, _dir)) = notepad() else {
            return;
        };
        let entry = session.entry_point();
        let explanation = explain(&mut session, entry).unwrap();
        assert!(explanation.instructions > 0);
        assert!(explanation.summary.contains("instructions"));
        // The entry point may sit inside the CRT startup function.
        assert!(explanation.start <= entry && entry < explanation.end);
    }

    #[test]
    fn polish_prompt_contains_l1_output_and_polish_system() {
        let Some((mut session, _dir)) = notepad() else {
            return;
        };
        let entry = session.entry_point();
        let settings = ProviderSettings::default();
        let prompt = build_polish_prompt(&mut session, entry, &settings).unwrap();
        assert_eq!(prompt.system, POLISH_SYSTEM);
        assert!(
            prompt
                .user
                .contains("pseudo-code (L2, may be mis-structured):"),
            "{}",
            prompt.user
        );
        assert!(prompt.user.contains("function: 0x"), "{}", prompt.user);
        assert!(
            prompt
                .user
                .contains("disassembly (authoritative control flow):"),
            "{}",
            prompt.user
        );
        assert!(
            prompt.user.contains("recover the original source"),
            "{}",
            prompt.user
        );
        assert!(
            !prompt.thinking,
            "polish asks for a direct answer so the first content token is not delayed"
        );
    }

    #[test]
    fn suggestions_carry_evidence_and_apply_through_command_bus() {
        let Some((mut session, _dir)) = notepad() else {
            return;
        };
        let proposals = suggest_names(&mut session, 50).unwrap();
        assert!(!proposals.is_empty(), "no rename proposals for notepad");
        for proposal in &proposals {
            assert_eq!(proposal.kind, ProposalKind::Rename);
            assert!(!proposal.evidence.is_empty());
            assert!(proposal.confidence > 0.0 && proposal.confidence <= 0.9);
        }
        let target = proposals[0].target;
        let name = proposals[0].value.clone();
        let applied = apply_proposals(&mut session, &proposals[..1]).unwrap();
        assert_eq!(applied, 1);
        assert_eq!(
            session.name_at(target).unwrap().as_deref(),
            Some(name.as_str())
        );
        // Undo restores the previous name.
        session.undo().unwrap();
        assert_ne!(
            session.name_at(target).unwrap().as_deref(),
            Some(name.as_str())
        );
    }

    #[test]
    fn triage_reports_categories_and_urls() {
        let Some((mut session, _dir)) = notepad() else {
            return;
        };
        let report = triage(&mut session).unwrap();
        assert!(report.functions > 0);
        assert!(report.instructions > 0);
        assert!(
            !report.apis_by_category.is_empty(),
            "notepad should have API tags"
        );
        assert!(!report.suspicious.is_empty());
    }

    #[test]
    fn prototype_proposal_applies_through_command_bus() {
        let Some((mut session, _dir)) = notepad() else {
            return;
        };
        let entry = session.entry_point();
        let proposal = Proposal {
            kind: ProposalKind::Prototype,
            target: entry,
            value: "int __stdcall wWinMain(void*, void*, wchar_t*, int)".to_owned(),
            confidence: 0.5,
            rationale: "user-confirmed prototype".to_owned(),
            evidence: Vec::new(),
        };
        assert_eq!(apply_proposals(&mut session, &[proposal]).unwrap(), 1);
        assert_eq!(
            session.prototype_at(entry).unwrap().as_deref(),
            Some("int __stdcall wWinMain(void*, void*, wchar_t*, int)")
        );
        session.undo().unwrap();
        assert!(session.prototype_at(entry).unwrap().is_none());
    }

    #[test]
    fn apply_type_proposal_binds_struct_to_argument() {
        let Some((mut session, _dir)) = notepad() else {
            return;
        };
        let entry = session.entry_point();
        session
            .execute(Command::DefineStruct {
                def: recraft_core::StructDef {
                    name: "Probe".to_owned(),
                    size: 8,
                    fields: vec![recraft_core::StructField {
                        offset: 0,
                        name: "tag".to_owned(),
                        kind: "u32".to_owned(),
                    }],
                },
            })
            .unwrap();
        let proposal = Proposal {
            kind: ProposalKind::ApplyType,
            target: entry,
            value: "0:Probe".to_owned(),
            confidence: 0.4,
            rationale: "test".to_owned(),
            evidence: Vec::new(),
        };
        assert_eq!(apply_proposals(&mut session, &[proposal]).unwrap(), 1);
        assert_eq!(
            session.arg_type(entry, 0).unwrap().as_deref(),
            Some("Probe")
        );
        session.undo().unwrap();
        assert!(session.arg_type(entry, 0).unwrap().is_none());
    }

    #[test]
    fn proposal_json_roundtrip() {
        let proposal = Proposal {
            kind: ProposalKind::Rename,
            target: Address::new(0x140001000),
            value: "inject_140001000".to_owned(),
            confidence: 0.5,
            rationale: "test".to_owned(),
            evidence: vec![Evidence {
                addr: Address::new(0x140001010),
                kind: "api_tag",
                text: "VirtualAlloc".to_owned(),
            }],
        };
        let value = proposal.to_json();
        let parsed = Proposal::from_json(&value).unwrap();
        assert_eq!(parsed.kind, ProposalKind::Rename);
        assert_eq!(parsed.target, proposal.target);
        assert_eq!(parsed.value, proposal.value);
        assert_eq!(parsed.evidence.len(), 1);
    }
}
