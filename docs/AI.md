# AI assistant（2026-11-14 精简）

ReCraft 只有**一个 AI 功能：分析助手**。它在 GUI 侧栏和 CLI（`recraft-cli ai ask`）里是同一套
实现：模型自己决定调用哪些**只读**工具（列函数、读伪代码、跟 xref、搜字符串、查 API 调用者、
看导入导出/元数据、调用图、字节搜索），取证后作答；轮次与每次返回都有预算，模型读不到也改不了
数据库。

之前的功能（函数解释、批量命名建议、离线自然语言查询、伪代码润色、AI 变量命名、样本初筛、
结果缓存与审计、prompt 评测与命名基准）已整体下线：它们要么是「问助手」的子集，要么产出没人
review 得动的批量提议。这些实现与它们的提示词、缓存、审计、评测代码都已删除（记录见
[CHANGELOG](../CHANGELOG.md)），当前 crate 只保留助手路径。

## 命令

```powershell
# 分析助手：模型调只读工具后作答（--json 输出 answer + 工具调用轨迹）
recraft-cli ai ask notepad.rcraft "哪些函数在操作注册表？" [--json] [--rounds 10]

# 解析后的 provider 配置（不打印密钥）
recraft-cli ai provider
```

GUI 的 **AI 侧栏**是同一个助手：输入问题 → `分析`；`新对话` 清空上下文；在 Decompile 里选中
伪代码、在 Hex 里选中字节、或选中一条反汇编指令后，可以勾「附带选中内容」把这段作为围栏块拼在
问题前面。答案流式显示，工具调用收在「工具调用 · N」折叠区里作为证据，答案中引用的 `0x…` 地址
（只保留落在已知函数内、去重后最多 10 个）可点。

## 工具与预算

工具定义与执行器来自 `recraft-query`，与 MCP 服务器**同一份 spec、同一个实现**，只是预算更小
（行数 / 伪代码行数，避免一次塞满上下文）：

| 工具 | 作用 |
|---|---|
| `list_functions` | 按名字子串列函数（地址/大小），先找候选；支持 `offset` 翻页 |
| `function_summary` | 名字、范围、指令数、原型、调用者（含函数名）、直接调用、引用字符串、API 标签 |
| `get_pseudocode` | 读该函数的伪 C（默认 L2，400 行/64 KiB 上限，`startLine` 翻页） |
| `xrefs_to` / `xrefs_from` | 谁引用它 / 它引用谁（附所属函数名） |
| `find_api_callers` | 哪些函数调用某个导入 API（如 `CreateFileW`） |
| `list_strings` | 按内容搜字符串表（`filter`） |
| `get_disassembly` | 原始反汇编 |
| `list_imports` / `list_exports` / `get_metadata` | 导入表、导出表、镜像元数据（段、base、entry、计数） |
| `callgraph` / `search_bytes` | 调用图、字节搜索 |

- **只读**：工具只查库与反编译，不写项目；模型在正文里给的重命名建议只是文字，要改名得自己按
  `N`（或走 MCP 的 `--write` 写工具），每一步都进 journal。
- **有界**：轮数上限 `AGENT_MAX_ROUNDS=10`，每次结果都截断，未知工具或坏参数以 `{"error": …}`
  返回给模型让它自我纠正；预算用尽时收回工具并要求“用已有证据作答”。
- **多轮追问**：面板保留整段对话（系统提示只出现一次，答案也写回历史），下一个问题可以直接接着
  问“那它的调用者呢”；`新对话` 清空上下文。对话超过 `AGENT_HISTORY_MAX_MESSAGES=60` 条时按
  user 边界裁掉最早的一轮，保证 assistant 的 `tool_calls` 与对应 tool 结果不会被拆开。CLI 的
  `ai ask` 是一次性问答。
- **流式与旁白**：答案按 SSE delta 流式显示。模型在调用工具前常先写一句“我先查一下……”
  （narration），`AgentEvent::TurnStart/Retract` 让面板在工具轮结束时把这一轮的文字从答案里
  撤回、移进工具轨迹（CLI 打到 stderr），所以最终答案不会混入旁白。
- 桌面端在**工作线程**里打开第二个 SQLite 连接跑整个循环，UI 只接收工具调用轨迹与最终答案。

## Provider 配置

HTTP 传输：OpenAI-compatible 与 Ollama（`recraft-ai/transport.rs`），GUI 与 CLI 共用同一层。

- **默认未配置**：`ProviderSettings` 默认 `disabled`（无 endpoint / key）；启用 HTTP provider
  必须同时设置 `RECRAFT_AI_PROVIDER=openai|ollama`、`RECRAFT_AI_ENDPOINT`，且
  `RECRAFT_AI_API_KEY`（变量名可配）非空，否则 `enabled=false`。UI 设置与文件配置同样遵守，
  未配置时 GUI 与 CLI 都提示先配置（`ai ask` 直接报错，不会假装回答）。
- **持久化**：`ProviderSettings::save()` 写入 `%LOCALAPPDATA%\ReCraft\ai-settings.json`；
  `load()` 优先读该文件，缺省回退环境变量，并按当前环境重算 `enabled`。
- **配置项**：`RECRAFT_AI_PROVIDER`、`RECRAFT_AI_ENDPOINT`、`RECRAFT_AI_MODEL`、
  `RECRAFT_AI_API_KEY_ENV`、`RECRAFT_AI_MAX_TOKENS`。
- **桌面端内置模式**（AI 面板 → ⚙ Provider settings）：`DeepSeek`、`OpenCode Go`、
  `Custom OpenAI-compatible`。选内模式会自动填 endpoint 和模型列表；`API key` 输入框会**按
  当前用户加密保存**（Windows DPAPI，写入 `ai-settings.json` 的 `api_key_sealed`；其他平台用
  `plain:` 前缀的 base64 仅作混淆），点 **Save** 后下次启动自动加载；旁边的 **Show/Hide**
  可查看明文，**Copy** 可复制。环境变量方式仍可用。

  ![AI provider modes](images/ai-modes.png)
- **已验证的兼容端点**（都是 OpenAI-compatible，`RECRAFT_AI_PROVIDER=openai`；endpoint
  填 base，不要带 `/v1`）：
  - **DeepSeek 官方**：`RECRAFT_AI_ENDPOINT=https://api.deepseek.com`，
    `RECRAFT_AI_MODEL=deepseek-flash`（或 `deepseek-v4-pro`），key 为 DeepSeek API key。
    旧的 `deepseek-chat` / `deepseek-reasoner` 已于 2026-07-24 下线；两个现役模型都支持 1M 上下文
    与 Thinking / Non-Thinking 双模式。V4 默认开启 thinking，而 thinking 会让首个正文 token 等上
    约一分钟，所以请求显式发送 `{"thinking":{"type":"disabled"}}` 直出。
  - **OpenCode Go**：`RECRAFT_AI_ENDPOINT=https://opencode.ai/inference/go/openai`，
    model 例如 `deepseek-v4.1-flash` / `deepseek-v4-pro` / `kimi-k3`；key 是 Console 里复制的
    Go API key。DeepSeek 模型需要在 Privacy 里把 Regions 设为 `Global`。请求会带
    `User-Agent: recraft/<version>` 和稳定的 `x-opencode-session` 头。该网关按 64k completion
    预算计算首字延迟，`direct_answer_token_limit` 会把它限制到 8k；`RECRAFT_AI_MAX_TOKENS`
    可覆盖。
  - 其他 OpenAI 兼容网关同理（如 `https://api.openai.com`）。
- 传输层：`Transport` trait；`TcpHttpTransport` 处理**明文 http://**（本地 Ollama
  `http://127.0.0.1:11434`），`UreqTransport`（ureq + rustls）处理 **https://**
  （OpenAI 官方端点、云厂商兼容 API）；`transport_for(url)` 按 scheme 选择。
  `MockTransport` 用于无网络单测。
- Provider：`OllamaProvider`（`POST /api/chat`, `stream:false`）与
  `OpenAiCompatibleProvider`（`POST {endpoint}/v1/chat/completions`，Bearer key）；
  `provider_from_settings` 在未配置时返回 `UnavailableProvider`（直接报
  "no LLM provider configured"，不会假装是模型）。

## 系统提示词

- 助手的系统提示词有中英两个内置版本（`AGENT_SYSTEM` / `AGENT_SYSTEM_ZH`），由
  `PromptLanguage` 选择（与界面语言无关，默认英文）。
- 设置窗口的「系统提示词」区可以按语言查看、编辑、保存与恢复内置：编辑值存进
  `ai-settings.json` 的 `prompts`（键为 `"<lang>:agent"`），空文本等于恢复内置；
  `system_prompt` 优先返回覆盖值，`run_agent` 走它。
- 想改提示词又不确定效果时，先 `ai ask` 跑一个问题看工具轨迹与答案，再决定改哪一句。

## 限制与后续

- **不缓存**：每次提问都会重新走一遍 provider（含多轮历史）；旧版本的 prompt 哈希缓存与
  `ai-audit.jsonl` 审计日志随直答类功能一起移除。
- **无写路径**：助手给的重命名/注释只是建议；改库要么手动（`N`、`;`），要么经 MCP 的写工具
  （journaled、可 undo）。MCP 与配置见 [MCP.md](MCP.md)。
- **网络可达性**：开发环境仅放行部分域名；TLS 栈通过 `https://crates.io/` 的忽略测试验证，
  `api.openai.com` 在本环境不可达，未做端到端 OpenAI 账号测试。
- **答案质量取决于模型与工具预算**：模糊问题（“这个样本在干嘛”）应当拆成可取证的小问题；
  助手会自己决定看哪些函数，但预算只有 10 轮。
