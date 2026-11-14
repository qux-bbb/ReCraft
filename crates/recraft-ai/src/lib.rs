// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! AI 层：只保留只读的“分析智能体”（DESIGN §9.2/§9.3）。
//!
//! 用户可见的 AI 功能只剩一个：`recraft-cli ai ask` 与 GUI 侧边栏共用的分析智能体。
//! 模型通过 `agent_tools` 暴露的只读工具读取当前项目，再据此回答问题；本 crate 不写项目，
//! 因此错误的回答只能误导读者，不会破坏数据库。
//!
//! - `agent` – 工具调用循环（`run_agent`、`run_agent_with_history_system`），有轮次上限。
//! - `provider` – 供应商配置、请求/回复结构与 SSE/工具流解析，是唯一的网络出口。
//! - `secret` – API 密钥的静态封存（Windows 上走 DPAPI）。
//! - `transport` – HTTP 传输与可替换的 mock。
//!
//! `truncate_text` 是 CLI 与 GUI 共用的显示辅助函数。

// DPAPI is the one OS FFI exception; it is isolated in `secret`.
#![deny(unsafe_code)]

mod agent;
mod provider;
#[allow(unsafe_code)]
mod secret;
mod transport;

pub use agent::{
    AGENT_MAX_ROUNDS, AGENT_SYSTEM, AGENT_SYSTEM_ZH, AgentEvent, agent_tools, run_agent,
    run_agent_with_history, run_agent_with_history_system,
};
pub use provider::{
    ChatMessage, ChatRole, DEEPSEEK_ENDPOINT, DEEPSEEK_MODELS, OPENCODE_GO_ENDPOINT,
    OPENCODE_GO_MODELS, OllamaProvider, OpenAiCompatibleProvider, PROMPT_TEMPLATE_VERSION,
    PromptKind, PromptLanguage, PromptRequest, Provider, ProviderKind, ProviderPreset,
    ProviderSettings, TokenBudget, ToolCall, ToolReply, ToolSpec, UnavailableProvider,
    parse_tool_reply, prompt_key, provider_from_settings, system_prompt,
};
pub use transport::{
    HttpRequest, HttpResponse, MockTransport, TcpHttpTransport, Transport, TransportKind,
    UreqTransport, parse_ollama_response, parse_openai_response, transport_for, transport_kind,
};

/// Truncate to `max_chars` characters on a char boundary, appending `…`.
pub fn truncate_text(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_owned();
    }
    let mut out: String = text.chars().take(max_chars).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::truncate_text;

    #[test]
    fn truncate_text_keeps_text_within_the_limit() {
        assert_eq!(truncate_text("abc", 8), "abc");
        assert_eq!(truncate_text("abc", 3), "abc");
        assert_eq!(truncate_text("", 0), "");
    }

    /// 截断必须落在字符边界上，多字节文本不能被切成半个字符。
    #[test]
    fn truncate_text_cuts_multi_byte_text_on_char_boundaries() {
        assert_eq!(truncate_text("分析智能体", 2), "分析…");
        assert_eq!(truncate_text(&"a".repeat(10), 4), "aaaa…");
    }
}
