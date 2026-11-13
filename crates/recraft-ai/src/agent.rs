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

use recraft_common::{Error, Result};
use recraft_core::Session;
use recraft_query::Budget;
use serde_json::json;

use crate::provider::{
    ChatMessage, ChatRole, Provider, ProviderSettings, ToolCall, ToolReply, ToolSpec,
    provider_from_settings,
};

/// System prompt for the agent. Kept short: the tool descriptions carry the detail.
pub const AGENT_SYSTEM: &str = "You are ReCraft's reverse-engineering agent. Answer questions \
about the open project by calling the read-only tools; never invent functions, addresses or \
strings you have not read. Call tools silently (no \"I will look for…\" narration): start \
broad (list_functions, find_api_callers), then read the one or two functions that matter \
(function_summary, get_pseudocode). Two to four tool calls are usually enough: answer as soon \
as the evidence supports it instead of reading everything. Cite addresses as 0x... in the \
answer. Answer in the language of the question. You cannot modify the project: propose renames \
as plain text and leave applying them to the user.";

/// Chinese system prompt for the agent (the built-in table pairs it with [`AGENT_SYSTEM`]).
pub const AGENT_SYSTEM_ZH: &str = "你是 ReCraft 的反向工程智能体。通过调用只读工具回答关于\
当前项目的问题；绝不臆造没有读过的函数、地址或字符串。安静地调用工具（不要“我先去找……”这类\
叙述）：先广后深（list_functions、find_api_callers），然后读真正重要的一两个函数\
（function_summary、get_pseudocode）。两到四次工具调用通常就够了：证据一足够就回答，不要\
把所有东西都读一遍。回答里用 0x... 引用地址。用提问的语言回答。你不能修改项目：把改名建议\
写成纯文本，交给用户应用。";

/// Default number of model turns `run_agent` allows (each may contain several tool calls).
/// One extra turn is always reserved for the answer after the tool budget is spent.
pub const AGENT_MAX_ROUNDS: usize = 10;

/// Conversation cap before the oldest turns are dropped. Tool results are large, so a long
/// session would otherwise crowd out the question being asked.
pub const AGENT_HISTORY_MAX_MESSAGES: usize = 60;

/// Progress the caller can show while the loop runs.
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// A tool is about to run.
    Tool { name: String, arguments: String },
    /// A model turn started; content deltas that follow belong to it.
    TurnStart,
    /// A content delta of the answer being streamed.
    Delta(String),
    /// The text streamed since [AgentEvent::TurnStart] was narration before tool calls,
    /// not part of the answer: drop it (the panel moves it into the tool trace).
    Retract,
    /// The final answer (identical to `run_agent`'s return value).
    Answer(String),
}

/// The tools the agent may call. Every one is read-only and bounded.
pub fn agent_tools() -> Vec<ToolSpec> {
    // The agent shares `recraft-query` with the MCP server: same tools, same result shapes,
    // only a smaller budget (a model turn cannot absorb 5 000 rows).
    recraft_query::specs(&Budget::agent())
        .into_iter()
        .map(|spec| ToolSpec {
            name: spec["name"].as_str().unwrap_or_default().to_owned(),
            description: spec["description"].as_str().unwrap_or_default().to_owned(),
            parameters: spec["inputSchema"].clone(),
        })
        .collect()
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
    let system = crate::provider::system_prompt(settings, crate::provider::PromptKind::Agent);
    let mut provider = provider_from_settings(settings);
    let mut history = Vec::new();
    run_agent_with_history_system(
        session,
        provider.as_mut(),
        &mut history,
        question,
        &system,
        max_rounds,
        on_event,
    )
}

/// [`run_agent`] against an already constructed provider. Only the tests use it: production
/// builds the provider from the settings, which is also where the system prompt comes from.
#[cfg(test)]
pub fn run_agent_with_provider(
    session: &mut Session,
    provider: &mut dyn Provider,
    question: &str,
    max_rounds: usize,
    on_event: &mut dyn FnMut(AgentEvent),
) -> Result<String> {
    let mut history = Vec::new();
    run_agent_with_history(
        session,
        provider,
        &mut history,
        question,
        max_rounds,
        on_event,
    )
}

/// Multi-turn variant: `history` carries the conversation across questions, so a follow-up
/// ("那它的调用者呢？") keeps the functions already read in context instead of starting over.
/// The caller owns the vector; it is extended with the question, the assistant/tool turns
/// and the answer.
pub fn run_agent_with_history(
    session: &mut Session,
    provider: &mut dyn Provider,
    history: &mut Vec<ChatMessage>,
    question: &str,
    max_rounds: usize,
    on_event: &mut dyn FnMut(AgentEvent),
) -> Result<String> {
    run_agent_with_history_system(
        session,
        provider,
        history,
        question,
        AGENT_SYSTEM,
        max_rounds,
        on_event,
    )
}

/// [`run_agent_with_history`] with an explicit system prompt: the settings UI can replace
/// [`AGENT_SYSTEM`] with the user's text (or its Chinese built-in).
pub fn run_agent_with_history_system(
    session: &mut Session,
    provider: &mut dyn Provider,
    history: &mut Vec<ChatMessage>,
    question: &str,
    system: &str,
    max_rounds: usize,
    on_event: &mut dyn FnMut(AgentEvent),
) -> Result<String> {
    if question.trim().is_empty() {
        return Err(Error::invalid_argument("the agent needs a question"));
    }
    let tools = agent_tools();
    let messages = history;
    if messages.is_empty() {
        messages.push(ChatMessage::system(system));
    }
    trim_history(messages);
    messages.push(ChatMessage::user(question));
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
            on_event(AgentEvent::TurnStart);
            return match provider.complete_chat_stream(
                messages.as_slice(),
                &[],
                false,
                &mut |chunk| on_event(AgentEvent::Delta(chunk.to_owned())),
            )? {
                ToolReply::Text(text) => {
                    if text.trim().is_empty() {
                        return Err(Error::internal("the model returned an empty answer"));
                    }
                    messages.push(ChatMessage::assistant(text.clone(), Vec::new()));
                    on_event(AgentEvent::Answer(text.clone()));
                    Ok(text)
                }
                ToolReply::Calls(_) => Err(Error::internal(
                    "the model kept calling tools after the tool budget was exhausted",
                )),
            };
        }
        on_event(AgentEvent::TurnStart);
        match provider.complete_chat_stream(messages.as_slice(), &tools, false, &mut |chunk| {
            on_event(AgentEvent::Delta(chunk.to_owned()))
        })? {
            ToolReply::Text(text) => {
                if text.trim().is_empty() {
                    return Err(Error::internal("the model returned an empty answer"));
                }
                // Keep the answer in the conversation: the next question builds on it.
                messages.push(ChatMessage::assistant(text.clone(), Vec::new()));
                on_event(AgentEvent::Answer(text.clone()));
                return Ok(text);
            }
            ToolReply::Calls(calls) => {
                // Any text streamed before the calls was narration ("I'll look for…"), not
                // an answer: let the caller drop it instead of mixing languages into the
                // final answer.
                on_event(AgentEvent::Retract);
                messages.push(ChatMessage::assistant("", calls.clone()));
                for call in calls {
                    on_event(AgentEvent::Tool {
                        name: call.name.clone(),
                        arguments: call.arguments.to_string(),
                    });
                    let result = execute_tool(session, &call);
                    messages.push(ChatMessage::tool(call.id.clone(), result));
                }
                trim_history(messages);
            }
        }
    }
    unreachable!("the loop always returns on its last round")
}

/// Execute one tool call and return its JSON result (errors become `{"error": ...}` so the
/// model can correct itself instead of the whole run failing).
fn execute_tool(session: &mut Session, call: &ToolCall) -> String {
    // Errors become `{"error": ...}` so the model can correct itself instead of the whole
    // run failing.
    match recraft_query::call(session, &call.name, &call.arguments, &Budget::agent()) {
        Ok(value) => value.to_string(),
        Err(error) => json!({"error": error}).to_string(),
    }
}

/// Keep the system prompt and the newest turns, cutting only at a user message so an
/// assistant `tool_calls` turn is never separated from the tool results that answer it.
pub(crate) fn trim_history(history: &mut Vec<ChatMessage>) {
    if history.len() <= AGENT_HISTORY_MAX_MESSAGES {
        return;
    }
    let has_system = history
        .first()
        .is_some_and(|message| message.role == ChatRole::System);
    // Smallest index whose removal still fits the cap; move forward to the next question
    // so the oldest surviving turn is a complete one.
    let system = usize::from(has_system);
    let mut cut = history.len() + system - AGENT_HISTORY_MAX_MESSAGES;
    while cut < history.len() && history[cut].role != ChatRole::User {
        cut += 1;
    }
    if cut >= history.len() {
        // No question boundary to cut at: keeping the history is safer than orphaning a
        // tool result, and the round limit already bounds a single answer.
        return;
    }
    history.drain(system..cut);
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

        fn complete_chat_stream(
            &mut self,
            messages: &[ChatMessage],
            tools: &[ToolSpec],
            thinking: bool,
            on_chunk: &mut dyn FnMut(&str),
        ) -> Result<ToolReply> {
            let reply = self.complete_chat(messages, tools, thinking)?;
            match &reply {
                ToolReply::Text(text) => on_chunk(text),
                // Models often narrate before calling a tool; the loop must retract it.
                ToolReply::Calls(_) => on_chunk("narration"),
            }
            Ok(reply)
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
                    arguments: json!(null),
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
    /// A follow-up question sees the previous answer: the conversation is carried, not
    /// rebuilt from scratch.
    #[test]
    fn a_custom_system_prompt_reaches_the_model() {
        let Some((mut session, _dir)) = notepad() else {
            return;
        };
        let mut provider = Scripted {
            replies: VecDeque::from(vec![ToolReply::Text("ok".to_owned())]),
            seen: Vec::new(),
        };
        let mut history = Vec::new();
        run_agent_with_history_system(
            &mut session,
            &mut provider,
            &mut history,
            "hi",
            "CUSTOM SYSTEM",
            1,
            &mut |_| {},
        )
        .unwrap();
        let first = &provider.seen[0][0];
        assert_eq!(first.role, ChatRole::System);
        assert_eq!(first.content, "CUSTOM SYSTEM");
    }

    #[test]
    fn a_follow_up_question_keeps_the_previous_turn() {
        let Some((mut session, _dir)) = notepad() else {
            return;
        };
        let mut provider = Scripted {
            replies: VecDeque::from(vec![
                ToolReply::Text("first answer".to_owned()),
                ToolReply::Text("second answer".to_owned()),
            ]),
            seen: Vec::new(),
        };
        let mut history = Vec::new();
        let first = run_agent_with_history(
            &mut session,
            &mut provider,
            &mut history,
            "first?",
            2,
            &mut |_| {},
        )
        .unwrap();
        assert_eq!(first, "first answer");
        let second = run_agent_with_history(
            &mut session,
            &mut provider,
            &mut history,
            "follow up?",
            2,
            &mut |_| {},
        )
        .unwrap();
        assert_eq!(second, "second answer");
        let second_turn = provider.seen.get(1).expect("second question");
        assert!(
            second_turn
                .iter()
                .any(|message| message.content == "first answer"),
            "previous answer missing from the follow-up turn"
        );
        assert_eq!(
            second_turn
                .iter()
                .filter(|message| message.role == ChatRole::System)
                .count(),
            1,
            "the system prompt must appear once"
        );
    }
    /// The conversation is capped, and the cut never separates an assistant tool call from
    /// the tool messages that answer it.
    #[test]
    fn history_trimming_keeps_system_and_tool_pairs() {
        let mut history = vec![ChatMessage::system("sys")];
        for turn in 0..30 {
            history.push(ChatMessage::user(format!("q{turn}")));
            history.push(ChatMessage::assistant(
                "",
                vec![ToolCall {
                    id: format!("call_{turn}"),
                    name: "list_functions".to_owned(),
                    arguments: json!(null),
                }],
            ));
            history.push(ChatMessage::tool(format!("call_{turn}"), "result"));
            history.push(ChatMessage::assistant(format!("a{turn}"), Vec::new()));
        }
        assert!(history.len() > AGENT_HISTORY_MAX_MESSAGES);
        trim_history(&mut history);
        assert!(
            history.len() <= AGENT_HISTORY_MAX_MESSAGES,
            "trimmed to {}",
            history.len()
        );
        assert_eq!(history[0].role, ChatRole::System);
        assert_eq!(history[1].role, ChatRole::User, "must start on a question");

        // Every remaining tool result still follows the assistant turn that asked for it.
        let mut open_calls = 0usize;
        for message in history.iter().skip(1) {
            match message.role {
                ChatRole::Assistant if !message.tool_calls.is_empty() => {
                    open_calls += message.tool_calls.len();
                }
                ChatRole::Tool => {
                    assert!(open_calls > 0, "orphan tool result");
                    open_calls -= 1;
                }
                ChatRole::User => assert_eq!(open_calls, 0, "turn cut mid tool pair"),
                _ => {}
            }
        }
    }
    /// Text streamed before a tool call is retracted so it cannot leak into the answer.
    #[test]
    fn narration_before_tools_is_retracted() {
        let Some((mut session, _dir)) = notepad() else {
            return;
        };
        let mut provider = Scripted {
            replies: VecDeque::from(vec![
                ToolReply::Calls(vec![ToolCall {
                    id: "c".to_owned(),
                    name: "list_functions".to_owned(),
                    arguments: json!({"limit": 1}),
                }]),
                ToolReply::Text("final".to_owned()),
            ]),
            seen: Vec::new(),
        };
        let mut events = Vec::new();
        let answer = run_agent_with_provider(&mut session, &mut provider, "q", 4, &mut |event| {
            events.push(match event {
                AgentEvent::TurnStart => "start".to_owned(),
                AgentEvent::Delta(chunk) => format!("delta:{chunk}"),
                AgentEvent::Retract => "retract".to_owned(),
                AgentEvent::Tool { .. } => "tool".to_owned(),
                AgentEvent::Answer(_) => "answer".to_owned(),
            });
        })
        .unwrap();
        assert_eq!(answer, "final");
        assert_eq!(
            events,
            vec![
                "start",
                "delta:narration",
                "retract",
                "tool",
                "start",
                "delta:final",
                "answer"
            ]
        );
    }
}
