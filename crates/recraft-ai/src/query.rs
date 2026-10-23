// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Natural-language queries (DESIGN §9.2: "find every function that references a URL"
//! → MCP-style tool calls).
//!
//! Fully offline: the intent is recognised with keyword heuristics and answered from the
//! session's derived tables (strings, xrefs, API tags, functions). An LLM can later be
//! used to *propose* the same structured query, but the execution path stays local and
//! read-only.

use std::collections::BTreeSet;

use recraft_common::{Address, Result};
use recraft_core::{FunctionInfo, Session};

/// One row of a query answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryItem {
    /// `url`, `api`, `function` or `text`.
    pub kind: String,
    pub addr: String,
    pub text: String,
    /// Functions related to the item (callers or referencing functions).
    pub functions: Vec<String>,
}

/// Structured result of a natural-language query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryAnswer {
    /// `urls`, `api_callers`, `function_callers` or `text`.
    pub intent: String,
    /// The recognised search target.
    pub query: String,
    pub items: Vec<QueryItem>,
    /// True when the item list was cut off.
    pub truncated: bool,
}

const MAX_ITEMS: usize = 50;
const XREF_CALL: i64 = 1;

/// Answer a natural-language question about the analysed project.
pub fn answer_query(session: &mut Session, text: &str) -> Result<QueryAnswer> {
    let lower = text.to_lowercase();
    let quoted = quoted_text(text);
    let mentions_url = ["url", "http", "link", "网址", "链接", "域名"]
        .iter()
        .any(|keyword| lower.contains(keyword));
    if mentions_url {
        return urls_query(session, quoted.as_deref());
    }
    // "find strings mentioning X" is a text query even when the sentence also contains
    // a caller keyword ("引用字符串 Microsoft").
    let mentions_string = ["字符串", "string", "text", "文本"]
        .iter()
        .any(|keyword| lower.contains(keyword));
    if mentions_string {
        let needle = quoted
            .clone()
            .or_else(|| identifier_candidate(text))
            .unwrap_or_else(|| text.trim().to_owned());
        return text_search(session, &needle);
    }
    let asks_for_callers = [
        "调用", "引用", "caller", "call", "calls", "uses", "使用", "谁",
    ]
    .iter()
    .any(|keyword| lower.contains(keyword));
    if asks_for_callers {
        // Prefer a known API rule mentioned in the sentence, then a known function name.
        let tags = session.all_api_tags()?;
        let mut best_api: Option<String> = None;
        for (_, rule, _) in &tags {
            let rule_lower = rule.to_lowercase();
            if lower.contains(&rule_lower)
                && best_api
                    .as_ref()
                    .is_none_or(|best| rule_lower.len() > best.len())
            {
                best_api = Some(rule.clone());
            }
        }
        if let Some(api) = best_api {
            return api_callers_query(session, &api);
        }
        let functions = session.functions()?;
        let mut best_function: Option<String> = None;
        for function in &functions {
            let Some(name) = session.name_at(function.start)? else {
                continue;
            };
            if name.starts_with("sub_") {
                continue;
            }
            let name_lower = name.to_lowercase();
            if lower.contains(&name_lower)
                && best_function
                    .as_ref()
                    .is_none_or(|best| name_lower.len() > best.len())
            {
                best_function = Some(name);
            }
        }
        if let Some(name) = best_function.or_else(|| quoted.clone()) {
            return function_callers_query(session, &name);
        }
        // Unknown target: still answer the caller question for the most plausible
        // identifier instead of degrading to a full-sentence text search.
        if let Some(candidate) = quoted.clone().or_else(|| identifier_candidate(text)) {
            return function_callers_query(session, &candidate);
        }
    }
    let needle = quoted
        .or_else(|| identifier_candidate(text))
        .unwrap_or_else(|| text.trim().to_owned());
    text_search(session, &needle)
}

fn urls_query(session: &mut Session, needle: Option<&str>) -> Result<QueryAnswer> {
    let strings = session.strings()?;
    let functions = session.functions()?;
    let mut items = Vec::new();
    let mut truncated = false;
    for entry in &strings {
        let text = string_text(session, entry)?;
        let lower = text.to_lowercase();
        let looks_like_url = ["http://", "https://", "ftp://", "www."]
            .iter()
            .any(|prefix| lower.starts_with(prefix));
        if !looks_like_url {
            continue;
        }
        if let Some(needle) = needle
            && !lower.contains(&needle.to_lowercase())
        {
            continue;
        }
        if items.len() >= MAX_ITEMS {
            truncated = true;
            break;
        }
        items.push(QueryItem {
            kind: "url".to_owned(),
            addr: format!("{:#x}", entry.addr.get()),
            text,
            functions: callers_of(session, &functions, entry.addr)?,
        });
    }
    Ok(QueryAnswer {
        intent: "urls".to_owned(),
        query: needle.unwrap_or("<url>").to_owned(),
        items,
        truncated,
    })
}

fn api_callers_query(session: &mut Session, api: &str) -> Result<QueryAnswer> {
    let tags = session.all_api_tags()?;
    let functions = session.functions()?;
    let mut names = BTreeSet::new();
    for (site, rule, _) in &tags {
        if !rule.eq_ignore_ascii_case(api) {
            continue;
        }
        if let Some(index) = containing(&functions, *site)
            && let Some(name) = session.name_at(functions[index].start)?
        {
            names.insert(name);
        }
    }
    let mut items: Vec<QueryItem> = names
        .into_iter()
        .map(|name| QueryItem {
            kind: "api".to_owned(),
            addr: functions
                .iter()
                .find(|function| {
                    session
                        .name_at(function.start)
                        .ok()
                        .flatten()
                        .is_some_and(|candidate| candidate == name)
                })
                .map(|function| format!("{:#x}", function.start.get()))
                .unwrap_or_default(),
            text: api.to_owned(),
            functions: vec![name],
        })
        .collect();
    let truncated = items.len() > MAX_ITEMS;
    items.truncate(MAX_ITEMS);
    Ok(QueryAnswer {
        intent: "api_callers".to_owned(),
        query: api.to_owned(),
        items,
        truncated,
    })
}

fn function_callers_query(session: &mut Session, name: &str) -> Result<QueryAnswer> {
    let functions = session.functions()?;
    let Some(target) = resolve_function(session, &functions, name)? else {
        return Ok(QueryAnswer {
            intent: "function_callers".to_owned(),
            query: name.to_owned(),
            items: Vec::new(),
            truncated: false,
        });
    };
    let callers = callers_of(session, &functions, target)?;
    let items: Vec<QueryItem> = callers
        .into_iter()
        .map(|caller| QueryItem {
            kind: "function".to_owned(),
            addr: format!("{:#x}", target.get()),
            text: name.to_owned(),
            functions: vec![caller],
        })
        .collect();
    Ok(QueryAnswer {
        intent: "function_callers".to_owned(),
        query: name.to_owned(),
        items,
        truncated: false,
    })
}

fn text_search(session: &mut Session, needle: &str) -> Result<QueryAnswer> {
    let needle_lower = needle.to_lowercase();
    let mut items = Vec::new();
    let functions = session.functions()?;
    for function in &functions {
        let Some(name) = session.name_at(function.start)? else {
            continue;
        };
        if name.to_lowercase().contains(&needle_lower) {
            items.push(QueryItem {
                kind: "function".to_owned(),
                addr: format!("{:#x}", function.start.get()),
                text: name,
                functions: Vec::new(),
            });
            if items.len() >= MAX_ITEMS {
                break;
            }
        }
    }
    if items.len() < MAX_ITEMS {
        let strings = session.strings()?;
        for entry in &strings {
            let text = string_text(session, entry)?;
            if !text.to_lowercase().contains(&needle_lower) {
                continue;
            }
            items.push(QueryItem {
                kind: "text".to_owned(),
                addr: format!("{:#x}", entry.addr.get()),
                text,
                functions: callers_of(session, &functions, entry.addr)?,
            });
            if items.len() >= MAX_ITEMS {
                break;
            }
        }
    }
    let truncated = items.len() >= MAX_ITEMS;
    Ok(QueryAnswer {
        intent: "text".to_owned(),
        query: needle.to_owned(),
        items,
        truncated,
    })
}

fn resolve_function(
    session: &Session,
    functions: &[FunctionInfo],
    name: &str,
) -> Result<Option<Address>> {
    if let Some(addr) = session.addr_of_name(name)? {
        return Ok(Some(addr));
    }
    let needle = name.to_lowercase();
    for function in functions {
        if session
            .name_at(function.start)?
            .is_some_and(|candidate| candidate.to_lowercase().contains(&needle))
        {
            return Ok(Some(function.start));
        }
    }
    Ok(None)
}

fn containing(functions: &[FunctionInfo], addr: Address) -> Option<usize> {
    let mut low = 0usize;
    let mut high = functions.len();
    while low < high {
        let mid = (low + high) / 2;
        if functions[mid].start <= addr {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    let index = low.checked_sub(1)?;
    (functions[index].start <= addr && addr < functions[index].end).then_some(index)
}

fn callers_of(
    session: &Session,
    functions: &[FunctionInfo],
    target: Address,
) -> Result<Vec<String>> {
    let mut names = BTreeSet::new();
    for xref in session.xrefs_to(target)? {
        if xref.kind != XREF_CALL {
            continue;
        }
        let Some(index) = containing(functions, xref.from) else {
            continue;
        };
        if let Some(name) = session.name_at(functions[index].start)? {
            names.insert(name);
        }
    }
    Ok(names.into_iter().collect())
}

fn string_text(session: &mut Session, entry: &recraft_core::StringEntry) -> Result<String> {
    let bytes = session.read_bytes(entry.addr, entry.len as usize)?;
    if entry.enc == 2 {
        // `recraft_analysis::STRING_ENC_UTF16LE` (kept as a literal so this crate does
        // not need analysis as a runtime dependency; the DB encoding is stable schema).
        let units: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();
        Ok(String::from_utf16_lossy(&units))
    } else {
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }
}

fn quoted_text(text: &str) -> Option<String> {
    for quote in ['"', '\'', '`'] {
        if let Some(start) = text.find(quote)
            && let Some(end) = text[start + 1..].find(quote)
        {
            return Some(text[start + 1..start + 1 + end].to_owned());
        }
    }
    None
}

/// Pick the most plausible identifier/symbol in a free-form question (`CreateFileW`,
/// `Microsoft`, `str_probe`). Uses the longest token that is not a generic question word.
fn identifier_candidate(text: &str) -> Option<String> {
    const STOPWORDS: &[&str] = &[
        "find",
        "every",
        "function",
        "functions",
        "call",
        "calls",
        "caller",
        "callers",
        "which",
        "that",
        "references",
        "reference",
        "referencing",
        "strings",
        "string",
        "url",
        "urls",
        "api",
        "apis",
        "the",
        "all",
        "does",
        "this",
        "project",
        "sample",
        "with",
        "from",
        "into",
        "uses",
        "used",
        "who",
        "what",
        "where",
        "many",
        "list",
        "show",
        "调用",
        "引用",
        "函数",
        "字符串",
        "所有",
        "哪些",
        "什么",
        "使用",
    ];
    text.split(|character: char| {
        !(character.is_ascii_alphanumeric()
            || character == '_'
            || character == '!'
            || character == '.')
    })
    .map(|token| token.trim_matches('!'))
    .filter(|token| token.len() >= 4)
    .filter(|token| {
        let lower = token.to_lowercase();
        !STOPWORDS.contains(&lower.as_str()) && lower != "http" && lower != "https"
    })
    .max_by_key(|token| token.len())
    .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use recraft_common::Result;

    fn notepad() -> Option<(Session, tempfile::TempDir)> {
        let bytes = std::fs::read("C:/Windows/System32/notepad.exe").ok()?;
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("query.rcraft");
        let mut session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let result = recraft_analysis::analyze(&mut session).unwrap();
        recraft_analysis::persist(&mut session, &result).unwrap();
        let strings = recraft_analysis::extract_strings(&mut session, 4, 200_000).unwrap();
        recraft_analysis::persist_strings(&mut session, &strings).unwrap();
        Some((session, dir))
    }

    #[test]
    fn url_query_finds_strings_and_callers() -> Result<()> {
        let Some((mut session, _dir)) = notepad() else {
            return Ok(());
        };
        let answer = answer_query(&mut session, "find every function that references a URL")?;
        assert_eq!(answer.intent, "urls");
        assert!(answer.items.iter().all(|item| item.kind == "url"));
        Ok(())
    }

    #[test]
    fn api_caller_query_mentions_functions() -> Result<()> {
        let Some((mut session, _dir)) = notepad() else {
            return Ok(());
        };
        let tags = session.all_api_tags()?;
        let Some((_, rule, _)) = tags.first() else {
            return Ok(());
        };
        let answer = answer_query(&mut session, &format!("which functions call {rule}"))?;
        assert_eq!(
            answer.intent, "api_callers",
            "rule={rule} answer={answer:?}"
        );
        assert_eq!(answer.query, *rule);
        assert!(answer.items.iter().any(|item| !item.text.is_empty()));
        Ok(())
    }

    #[test]
    fn unknown_caller_target_still_answers_the_question() -> Result<()> {
        let Some((mut session, _dir)) = notepad() else {
            return Ok(());
        };
        let answer = answer_query(&mut session, "which functions call CreateFileW")?;
        assert!(
            matches!(answer.intent.as_str(), "function_callers" | "api_callers"),
            "unexpected intent: {}",
            answer.intent
        );
        assert_eq!(answer.query.to_lowercase(), "createfilew");
        Ok(())
    }

    #[test]
    fn plain_text_query_still_returns_something() -> Result<()> {
        let Some((mut session, _dir)) = notepad() else {
            return Ok(());
        };
        let answer = answer_query(&mut session, "Microsoft")?;
        assert_eq!(answer.intent, "text");
        assert!(!answer.items.is_empty(), "expected at least one match");
        Ok(())
    }
}
