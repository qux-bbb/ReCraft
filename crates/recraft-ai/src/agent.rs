// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Project-level agent: the model answers a question by calling **read-only** tools on the
//! open project instead of receiving one pre-selected function in the prompt.
//!
//! The panel used to be a set of one-shot buttons: "explain this function", "name these
//! functions". The questions users actually ask — *where is persistence handled? which
//! function parses the command line? what calls CreateFileW?* — need the model to choose
//! what to look at. This module gives it the same evidence the MCP server exposes
//! ([`agent_tools`]) and runs the tool loop; nothing here writes to the project, so a
//! wrong answer can mislead the reader but never corrupt the database. Renames still go
//! through proposals and an explicit apply.
//!
//! The caller owns the [`Session`]: the CLI passes its open session, the desktop opens a
//! second connection inside the worker thread so the UI never blocks on HTTP or decoding.

use recraft_common::{Address, Error, Result};
use recraft_core::{FunctionInfo, Session};
use serde_json::{Value, json};

use crate::provider::{
    ChatMessage, Provider, ProviderSettings, ToolCall, ToolReply, ToolSpec, provider_from_settings,
};

/// System prompt for the agent. Kept short: the tool descriptions carry the detail.
pub const AGENT_SYSTEM: &str = "You are ReCraft's reverse-engineering agent. Answer questions \
about the open project by calling the read-only tools; never invent functions, addresses or \
strings you have not read. Start broad (list_functions, find_api_callers), then read the one \
or two functions that matter (function_info, pseudocode). Two to four tool calls are usually \
enough: answer as soon as the evidence supports it instead of reading everything. Cite \
addresses as 0x... in the answer. Answer in the language of the question. You cannot modify \
the project: propose renames as plain text and leave applying them to the user.";

/// Default number of model turns `run_agent` allows (each may contain several tool calls).
/// One extra turn is always reserved for the answer after the tool budget is spent.
pub const AGENT_MAX_ROUNDS: usize = 10;

/// Progress the caller can show while the loop runs.
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// A tool is about to run.
    Tool { name: String, arguments: String },
    /// The final answer (identical to `run_agent`'s return value).
    Answer(String),
}

/// The tools the agent may call. Every one is read-only and bounded.
pub fn agent_tools() -> Vec<ToolSpec> {
    vec![
        ToolSpec {
            name: "list_functions",
            description: "List functions with their address and size. `filter` matches the \
                          symbol name (case-insensitive).",
            parameters: json!({
                "type": "object",
                "properties": {
                    "filter": {"type": "string", "description": "substring of the function name"},
                    "limit": {"type": "integer", "minimum": 1, "maximum": 200}
                }
            }),
        },
        ToolSpec {
            name: "function_info",
            description: "Summarise one function: name, range, instruction count, imported \
                          APIs, referenced strings, direct calls and callers.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "addr": {"type": "string", "description": "0xaddress, symbol name or entry"}
                },
                "required": ["addr"]
            }),
        },
        ToolSpec {
            name: "pseudocode",
            description: "Read a function's reconstructed C (L2). Use max_lines for huge \
                          functions.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "addr": {"type": "string"},
                    "max_lines": {"type": "integer", "minimum": 10, "maximum": 400}
                },
                "required": ["addr"]
            }),
        },
        ToolSpec {
            name: "xrefs_to",
            description: "List references to an address (who calls or reads it).",
            parameters: json!({
                "type": "object",
                "properties": {
                    "addr": {"type": "string"},
                    "limit": {"type": "integer", "minimum": 1, "maximum": 100}
                },
                "required": ["addr"]
            }),
        },
        ToolSpec {
            name: "xrefs_from",
            description: "List references made from an address.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "addr": {"type": "string"},
                    "limit": {"type": "integer", "minimum": 1, "maximum": 100}
                },
                "required": ["addr"]
            }),
        },
        ToolSpec {
            name: "find_api_callers",
            description: "Find the functions that call an imported API (import name substring, \
                          e.g. CreateFileW).",
            parameters: json!({
                "type": "object",
                "properties": {
                    "api": {"type": "string"},
                    "limit": {"type": "integer", "minimum": 1, "maximum": 100}
                },
                "required": ["api"]
            }),
        },
        ToolSpec {
            name: "search_strings",
            description: "Search the string table by content (case-insensitive).",
            parameters: json!({
                "type": "object",
                "properties": {
                    "text": {"type": "string"},
                    "limit": {"type": "integer", "minimum": 1, "maximum": 100}
                },
                "required": ["text"]
            }),
        },
        ToolSpec {
            name: "disassemble",
            description: "Read raw disassembly from an address (up to 60 instructions).",
            parameters: json!({
                "type": "object",
                "properties": {
                    "addr": {"type": "string"},
                    "count": {"type": "integer", "minimum": 1, "maximum": 60}
                },
                "required": ["addr"]
            }),
        },
    ]
}

/// Run the tool loop until the model answers.
///
/// `max_rounds` bounds the back-and-forth; a model that keeps calling tools without ever
/// answering fails loudly instead of burning tokens forever.
pub fn run_agent(
    session: &mut Session,
    settings: &ProviderSettings,
    question: &str,
    max_rounds: usize,
    on_event: &mut dyn FnMut(AgentEvent),
) -> Result<String> {
    let mut provider = provider_from_settings(settings);
    run_agent_with_provider(session, provider.as_mut(), question, max_rounds, on_event)
}

/// [`run_agent`] against an already constructed provider (tests drive the loop with a
/// scripted provider; production builds one from the settings).
pub fn run_agent_with_provider(
    session: &mut Session,
    provider: &mut dyn Provider,
    question: &str,
    max_rounds: usize,
    on_event: &mut dyn FnMut(AgentEvent),
) -> Result<String> {
    if question.trim().is_empty() {
        return Err(Error::invalid_argument("the agent needs a question"));
    }
    let tools = agent_tools();
    let mut messages = vec![
        ChatMessage::system(AGENT_SYSTEM),
        ChatMessage::user(question),
    ];
    let rounds = max_rounds.clamp(1, 16);
    for round in 0..=rounds {
        // The last turn is reserved for the answer: tools are withdrawn and the model is
        // told to synthesise. Without it a broad question burned every round on evidence
        // and the user got no answer at all.
        let last = round == rounds;
        if last {
            messages.push(ChatMessage::user(
                "Tool budget exhausted. Answer now using only the evidence already \
                 collected; state explicitly what you could not verify.",
            ));
            return match provider.complete_chat(&messages, &[], false)? {
                ToolReply::Text(text) => {
                    on_event(AgentEvent::Answer(text.clone()));
                    Ok(text)
                }
                ToolReply::Calls(_) => Err(Error::internal(
                    "the model kept calling tools after the tool budget was exhausted",
                )),
            };
        }
        match provider.complete_chat(&messages, &tools, false)? {
            ToolReply::Text(text) => {
                on_event(AgentEvent::Answer(text.clone()));
                return Ok(text);
            }
            ToolReply::Calls(calls) => {
                messages.push(ChatMessage::assistant("", calls.clone()));
                for call in calls {
                    on_event(AgentEvent::Tool {
                        name: call.name.clone(),
                        arguments: call.arguments.to_string(),
                    });
                    let result = execute_tool(session, &call);
                    messages.push(ChatMessage::tool(call.id.clone(), result));
                }
            }
        }
    }
    unreachable!("the loop always returns on its last round")
}

/// Execute one tool call and return its JSON result (errors become `{"error": ...}` so the
/// model can correct itself instead of the whole run failing).
fn execute_tool(session: &mut Session, call: &ToolCall) -> String {
    let result = match call.name.as_str() {
        "list_functions" => tool_list_functions(session, &call.arguments),
        "function_info" => tool_function_info(session, &call.arguments),
        "pseudocode" => tool_pseudocode(session, &call.arguments),
        "xrefs_to" => tool_xrefs(session, &call.arguments, true),
        "xrefs_from" => tool_xrefs(session, &call.arguments, false),
        "find_api_callers" => tool_find_api_callers(session, &call.arguments),
        "search_strings" => tool_search_strings(session, &call.arguments),
        "disassemble" => tool_disassemble(session, &call.arguments),
        other => Err(Error::internal(format!("unknown tool {other:?}"))),
    };
    match result {
        Ok(value) => value.to_string(),
        Err(error) => json!({"error": error.to_string()}).to_string(),
    }
}

fn limit_arg(args: &Value, key: &str, default: usize, max: usize) -> usize {
    args.get(key)
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or(default)
        .clamp(1, max)
}

fn string_arg<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
}

/// Resolve `0x...`, decimal, a symbol name or `entry`.
fn address_arg(session: &Session, args: &Value, key: &str) -> Result<Address> {
    let text = string_arg(args, key).ok_or_else(|| Error::internal(format!("missing {key}")))?;
    if text.eq_ignore_ascii_case("entry") {
        return Ok(session.entry_point());
    }
    let digits = text
        .strip_prefix("0x")
        .or_else(|| text.strip_prefix("0X"))
        .unwrap_or(text);
    if let Ok(value) = u64::from_str_radix(digits, 16)
        && (text.starts_with("0x") || text.starts_with("0X") || value > 0xFFFF)
    {
        return Ok(Address::new(value));
    }
    if let Some(addr) = session.addr_of_name(text)? {
        return Ok(addr);
    }
    if let Ok(value) = text.parse::<u64>() {
        return Ok(Address::new(value));
    }
    Err(Error::internal(format!("unknown address {text:?}")))
}

fn function_of(session: &Session, addr: Address) -> Result<FunctionInfo> {
    session
        .functions()?
        .iter()
        .find(|function| addr >= function.start && addr < function.end)
        .copied()
        .ok_or_else(|| Error::internal(format!("no function contains {:#x}", addr.get())))
}

fn name_of(session: &Session, start: Address) -> String {
    session
        .name_at(start)
        .ok()
        .flatten()
        .unwrap_or_else(|| format!("sub_{:x}", start.get()))
}

fn tool_list_functions(session: &mut Session, args: &Value) -> Result<Value> {
    let filter = string_arg(args, "filter")
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    let limit = limit_arg(args, "limit", 50, 200);
    let mut matched = Vec::new();
    for function in session.functions()? {
        let name = name_of(session, function.start);
        if !filter.is_empty() && !name.to_ascii_lowercase().contains(&filter) {
            continue;
        }
        matched.push(json!({
            "name": name,
            "addr": format!("{:#x}", function.start.get()),
            "size": function.end.get().saturating_sub(function.start.get()),
        }));
    }
    let total = matched.len();
    matched.truncate(limit);
    Ok(json!({
        "functions": matched,
        "count": matched.len(),
        "total": total,
        "truncated": total > limit,
    }))
}

fn tool_function_info(session: &mut Session, args: &Value) -> Result<Value> {
    let addr = address_arg(session, args, "addr")?;
    let explanation = crate::explain(session, addr)?;
    let prototype = session.prototype_at(explanation.start)?;
    Ok(json!({
        "name": explanation.name,
        "start": format!("{:#x}", explanation.start.get()),
        "end": format!("{:#x}", explanation.end.get()),
        "instructions": explanation.instructions,
        "prototype": prototype,
        "apis": explanation
            .apis
            .iter()
            .take(16)
            .map(|(site, rule, category)| json!({
                "addr": format!("{:#x}", site.get()),
                "api": rule,
                "category": category,
            }))
            .collect::<Vec<_>>(),
        "strings": explanation
            .strings
            .iter()
            .take(8)
            .map(|(site, text)| json!({
                "addr": format!("{:#x}", site.get()),
                "text": crate::truncate_text(text, 200),
            }))
            .collect::<Vec<_>>(),
        "calls": explanation
            .calls
            .iter()
            .take(16)
            .map(|(site, name)| json!({"addr": format!("{:#x}", site.get()), "name": name}))
            .collect::<Vec<_>>(),
        "callers": explanation
            .callers
            .iter()
            .take(8)
            .map(|(site, name)| json!({"addr": format!("{:#x}", site.get()), "name": name}))
            .collect::<Vec<_>>(),
    }))
}

fn tool_pseudocode(session: &mut Session, args: &Value) -> Result<Value> {
    let addr = address_arg(session, args, "addr")?;
    let function = function_of(session, addr)?;
    let max_lines = limit_arg(args, "max_lines", 150, 400);
    let text = recraft_decompile::pseudocode_l2(session, function.start)
        .or_else(|_| recraft_decompile::pseudocode_l1(session, function.start))?;
    let total = text.lines.len();
    let lines: Vec<String> = text
        .lines
        .iter()
        .take(max_lines)
        .map(|line| crate::truncate_text(&line.text, 400))
        .collect();
    Ok(json!({
        "function": name_of(session, function.start),
        "start": format!("{:#x}", function.start.get()),
        "lines": lines,
        "total_lines": total,
        "truncated": total > max_lines,
    }))
}

fn tool_xrefs(session: &mut Session, args: &Value, incoming: bool) -> Result<Value> {
    let addr = address_arg(session, args, "addr")?;
    let limit = limit_arg(args, "limit", 40, 100);
    let rows = if incoming {
        session.xrefs_to(addr)?
    } else {
        session.xrefs_from(addr)?
    };
    let total = rows.len();
    let mut out = Vec::new();
    for row in rows.iter().take(limit) {
        let (site, target) = if incoming {
            (row.from, addr)
        } else {
            (addr, row.to)
        };
        let owner = session
            .functions()?
            .iter()
            .find(|function| site >= function.start && site < function.end)
            .map(|function| name_of(session, function.start));
        out.push(json!({
            "addr": format!("{:#x}", site.get()),
            "target": format!("{:#x}", target.get()),
            "kind": row.kind,
            "function": owner,
        }));
    }
    Ok(json!({
        "xrefs": out,
        "count": out.len(),
        "total": total,
        "truncated": total > limit,
    }))
}

fn tool_find_api_callers(session: &mut Session, args: &Value) -> Result<Value> {
    let needle = string_arg(args, "api")
        .ok_or_else(|| Error::internal("missing api"))?
        .to_ascii_lowercase();
    let limit = limit_arg(args, "limit", 30, 100);
    let functions = session.functions()?;
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for (site, rule, category) in session.all_api_tags()? {
        if !rule.to_ascii_lowercase().contains(&needle) {
            continue;
        }
        let Some(function) = functions
            .iter()
            .find(|function| site >= function.start && site < function.end)
        else {
            continue;
        };
        if !seen.insert(function.start.get()) {
            continue;
        }
        out.push(json!({
            "name": name_of(session, function.start),
            "addr": format!("{:#x}", function.start.get()),
            "api": rule,
            "category": category,
            "site": format!("{:#x}", site.get()),
        }));
    }
    let total = out.len();
    out.truncate(limit);
    Ok(json!({
        "functions": out,
        "count": out.len(),
        "total": total,
        "truncated": total > limit,
    }))
}

fn tool_search_strings(session: &mut Session, args: &Value) -> Result<Value> {
    let needle = string_arg(args, "text")
        .ok_or_else(|| Error::internal("missing text"))?
        .to_ascii_lowercase();
    let limit = limit_arg(args, "limit", 30, 100);
    let entries = session.strings()?;
    let mut out = Vec::new();
    for entry in entries {
        let text = crate::read_string(session, entry.addr, entry.len, entry.enc);
        if !text.to_ascii_lowercase().contains(&needle) {
            continue;
        }
        out.push(json!({
            "addr": format!("{:#x}", entry.addr.get()),
            "text": crate::truncate_text(&text, 200),
        }));
        if out.len() >= limit {
            break;
        }
    }
    Ok(json!({
        "strings": out,
        "count": out.len(),
        "truncated": out.len() >= limit,
    }))
}

fn tool_disassemble(session: &mut Session, args: &Value) -> Result<Value> {
    let addr = address_arg(session, args, "addr")?;
    let count = limit_arg(args, "count", 30, 60);
    let rows = session.disasm(addr, count)?;
    let lines: Vec<Value> = rows
        .iter()
        .map(|line| {
            json!({
                "addr": format!("{:#x}", line.addr.get()),
                "mnemonic": line.mnemonic,
                "operands": line.operands,
            })
        })
        .collect();
    Ok(json!({"lines": lines, "count": lines.len()}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    use crate::provider::{ChatRole, PromptRequest, ToolReply};

    /// Replays scripted assistant turns and records what the loop sent back.
    struct Scripted {
        replies: VecDeque<ToolReply>,
        seen: Vec<Vec<ChatMessage>>,
    }

    impl Provider for Scripted {
        fn name(&self) -> &'static str {
            "scripted"
        }

        fn complete(&mut self, _request: &PromptRequest) -> Result<String> {
            unreachable!("the agent only uses complete_chat")
        }

        fn complete_chat(
            &mut self,
            messages: &[ChatMessage],
            _tools: &[ToolSpec],
            _thinking: bool,
        ) -> Result<ToolReply> {
            self.seen.push(messages.to_vec());
            self.replies
                .pop_front()
                .ok_or_else(|| Error::internal("script exhausted"))
        }
    }

    fn notepad() -> Option<(Session, tempfile::TempDir)> {
        let bytes = std::fs::read("C:/Windows/System32/notepad.exe").ok()?;
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("agent.rcraft");
        let mut session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let result = recraft_analysis::analyze(&mut session).unwrap();
        recraft_analysis::persist(&mut session, &result).unwrap();
        Some((session, dir))
    }

    /// One tool round, then an answer: the loop must execute the tool, feed the result back
    /// with the right `tool_call_id`, and report the trace.
    #[test]
    fn agent_runs_a_tool_then_answers() {
        let Some((mut session, _dir)) = notepad() else {
            return;
        };
        let mut provider = Scripted {
            replies: VecDeque::from(vec![
                ToolReply::Calls(vec![ToolCall {
                    id: "call_1".to_owned(),
                    name: "list_functions".to_owned(),
                    arguments: json!({"filter": "sub_", "limit": 3}),
                }]),
                ToolReply::Text("three functions".to_owned()),
            ]),
            seen: Vec::new(),
        };
        let mut trace = Vec::new();
        let answer = run_agent_with_provider(
            &mut session,
            &mut provider,
            "list three functions",
            4,
            &mut |event| {
                if let AgentEvent::Tool { name, .. } = event {
                    trace.push(name);
                }
            },
        )
        .unwrap();
        assert_eq!(answer, "three functions");
        assert_eq!(trace, vec!["list_functions"]);

        let second = provider.seen.get(1).expect("second turn");
        let tool_message = second.last().expect("tool result");
        assert_eq!(tool_message.role, ChatRole::Tool);
        assert_eq!(tool_message.tool_call_id.as_deref(), Some("call_1"));
        assert!(
            tool_message.content.contains("functions"),
            "tool result missing: {}",
            tool_message.content
        );
    }

    /// A bad tool name is reported back to the model instead of failing the whole run.
    #[test]
    fn unknown_tools_report_an_error_to_the_model() {
        let Some((mut session, _dir)) = notepad() else {
            return;
        };
        let mut provider = Scripted {
            replies: VecDeque::from(vec![
                ToolReply::Calls(vec![ToolCall {
                    id: "c".to_owned(),
                    name: "does_not_exist".to_owned(),
                    arguments: Value::Null,
                }]),
                ToolReply::Text("recovered".to_owned()),
            ]),
            seen: Vec::new(),
        };
        let answer =
            run_agent_with_provider(&mut session, &mut provider, "q", 4, &mut |_| {}).unwrap();
        assert_eq!(answer, "recovered");
        let second = provider.seen.get(1).expect("second turn");
        assert!(
            second.last().unwrap().content.contains("unknown tool"),
            "{}",
            second.last().unwrap().content
        );
    }

    /// When the tool budget runs out the model still answers: the loop withdraws the
    /// tools and asks for a synthesis from the evidence already collected.
    #[test]
    fn the_tool_budget_ends_with_an_answer() {
        let Some((mut session, _dir)) = notepad() else {
            return;
        };
        let call = || {
            ToolReply::Calls(vec![ToolCall {
                id: "c".to_owned(),
                name: "list_functions".to_owned(),
                arguments: json!({"limit": 1}),
            }])
        };
        let mut provider = Scripted {
            replies: VecDeque::from(vec![call(), call(), ToolReply::Text("partial".to_owned())]),
            seen: Vec::new(),
        };
        let answer =
            run_agent_with_provider(&mut session, &mut provider, "q", 2, &mut |_| {}).unwrap();
        assert_eq!(answer, "partial");
        // The synthesis turn was sent without tools and with the exhaustion note.
        let last = provider.seen.last().expect("synthesis turn");
        assert!(
            last.last()
                .unwrap()
                .content
                .contains("Tool budget exhausted"),
            "{}",
            last.last().unwrap().content
        );
    }

    /// A model that ignores the withdrawal and keeps asking for tools fails loudly.
    #[test]
    fn ignoring_the_synthesis_turn_is_an_error() {
        let Some((mut session, _dir)) = notepad() else {
            return;
        };
        let call = || {
            ToolReply::Calls(vec![ToolCall {
                id: "c".to_owned(),
                name: "list_functions".to_owned(),
                arguments: json!({"limit": 1}),
            }])
        };
        let mut provider = Scripted {
            replies: VecDeque::from(vec![call(), call(), call()]),
            seen: Vec::new(),
        };
        let error = run_agent_with_provider(&mut session, &mut provider, "q", 2, &mut |_| {})
            .expect_err("must fail");
        assert!(
            error.to_string().contains("tool budget was exhausted"),
            "{error}"
        );
    }
}
