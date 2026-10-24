# AI assist（M3）

DESIGN §9 的 AI 层分两半：**确定性的离线能力** 与 **可插拔 LLM**。本里程碑先交付离线
能力 + 写操作协议；LLM provider（OpenAI 兼容 / 本地 Ollama）之后接入同一套 Proposals。

核心原则（DESIGN §9.1）：**LLM/启发式只能产生 `Proposal`，禁止静默改库**。Proposal 必须带
证据地址与置信度，用户 review 后经 `Command` 总线（journaled、可 undo）应用。

## 命令

```powershell
# 函数解释：API 标签、引用字符串、直接调用、指令数（离线，无网络）
recraft ai explain notepad.rcraft --addr entry [--json]

# 命名建议：只针对 sub_* 函数，输出 Proposal（证据 + 置信度 + 理由），不修改项目
recraft ai suggest notepad.rcraft [--limit 20] [--json] [--out proposals.json]

# 样本摘要：API 分类统计、可疑函数 top-N、URL
recraft ai triage notepad.rcraft [--json]

# 应用已 review 的 proposals（无 --yes 直接拒绝）
recraft ai apply notepad.rcraft proposals.json --yes

# 自然语言查询（DESIGN §9.2）：离线意图解析 → 本地表查询
recraft ai query notepad.rcraft "find every function that references a URL" [--json]
recraft ai query notepad.rcraft "找所有引用字符串 Microsoft 的函数"
recraft ai query notepad.rcraft "which functions call CreateFileW"

# 伪代码润色（DESIGN §9.2）：需要已配置 LLM；只输出建议，不写回项目
recraft ai polish notepad.rcraft --addr entry [--json] [--no-cache]
```

## 自然语言查询（离线）

`ai query` 先在本地识别意图，再查已分析项目的派生表（strings/xref/API tags/functions），
**不联网**：

| 意图 | 触发关键词 | 回答内容 |
|---|---|---|
| `urls` | url/http/网址/链接 | 以 `http(s)://`/`www.` 开头的字符串 + 引用它们的函数 |
| `api_callers` | 调用/引用/call/caller + 句中出现已知 API 名 | 调用该 API 的函数列表 |
| `function_callers` | 同上 + 函数名/引号目标 | 该函数的调用者 |
| `text` | 字符串/string/text 或兜底 | 名字/字符串内容匹配，并附引用函数 |

未识别的目标会抽取句中"最长标识符"（如 `CreateFileW`、`Microsoft`）继续查询，而不是
退化为整句匹配。`--json` 输出 `{intent, query, items:[{kind,addr,text,functions}], count}`。

## 伪代码润色（需 LLM）

`ai polish` 取该函数的 **L1** 输出，构造带脱敏的 prompt（`POLISH_SYSTEM`：只允许改善命名/
格式，禁止改动语义），经 provider 发送；结果仅打印（带 `// AI polish suggestion` 头），
**不写回项目**，并复用结果缓存（prompt 哈希）与审计日志。未配置 provider 时直接报错并
给出配置提示（默认离线）。

## Proposal 协议

```json
{
  "kind": "rename",
  "target": "0x1400012dc",
  "value": "network_1400012dc",
  "confidence": 0.65,
  "rationale": "calls InternetOpenUrlW; 1 string ref(s)",
  "evidence": [
    {"addr": "0x140001368", "kind": "api_tag", "text": "WININET.dll!InternetOpenUrlW"},
    {"addr": "0x140002e7f0", "kind": "string", "text": "http://example/"}
  ]
}
```

- `apply` 把 `rename` / `comment` / `prototype` / `apply_type` 映射为对应 `Command`
  （`Rename`/`SetComment`/`SetPrototype`/`ApplyType`），写入 journal，可 `recraft undo`
  撤销；`apply_type` 的 proposal value 形如 `"0:Probe"`（槽位:结构体名，空类型名=清除）。
  `ai suggest` 本身永不写库。
- 无 `--yes` 时返回 "refusing to apply proposals without --yes"，防止误操作。

## 命名启发式（离线）

对每个 `sub_*` 函数，依据函数内 call 站点的 API 类别与引用字符串选择前缀
（`network` / `memory` / `file` / `process` / `thread` / `registry` / `service` /
`crypto` / `anti_debug` / `inject` / `ui`），并追加函数地址保证唯一；置信度随证据数
增长（0.35 → 0.9 上限）。没有证据的函数不产生建议。

## 样本摘要（triage）

- `apis_by_category`：全样本 API 标签按类别计数。
- `suspicious`：按函数聚合 API 类别加权（injection 4、anti-analysis 3、service 2.5、
  registry/process 2、network/thread 1.5、crypto 1.2、memory/file 1、ui 0.5、other 0.2），
  取 top-10。
- `urls`：字符串表中 `http(s)://` 开头的去重 URL（最多 20）。

## 实现与测试

- 代码：`crates/recraft-ai`（`explain` / `suggest_names` / `triage` / `Proposal` /
  `apply_proposals`）。
- 测试：notepad 真实样本——解释入口函数、建议带证据、`apply` 后 `name_at` 生效且可 undo、
  triage 类别非空、Proposal JSON roundtrip。
- CLI 冒烟纳入 `scripts/e2e.ps1`（triage JSON / suggest --out / apply --yes）。

## UI proposal 面板（M3）

左侧新增 **AI** 页签：

- **Suggest names**：调用 `suggest_names`，把 Proposal 列表展示为 `置信度 地址 -> 名称`，
  下方显示理由与证据数量；证据链接可跳转到对应地址。
- 单选 proposal → **Apply selected**：经 `Command` 总线执行（journaled，`Ctrl+Z` 可撤销），
  应用后从列表移除并刷新反汇编/函数列表。
- **Explain**：离线解释当前光标所在函数（summary/API/字符串/调用），结果显示在面板内，
  不依赖网络与配置。
- **Polish**：对当前函数构建润色 prompt 并调用已启用的 provider；未启用时提示先配置。
  请求在后台线程执行，UI 不阻塞；结果只做展示（不写回伪代码）。
- **Provider settings**：UI 内直接编辑 provider 类型、endpoint、model、API key 环境变量名与
  max tokens，保存到 `%LOCALAPPDATA%\ReCraft\ai-settings.json`（只存环境变量名，**不存密钥**）。
  保存后从环境变量重算 `enabled`；CLI 的 `ai provider` 仍可查看解析结果。
- **Clear** 清空当前建议、解释与状态。

无项目/未分析时会提示先 Analyze。UI 不会在后台自动应用任何提案。

## Provider 抽象与 prompt 安全（M3 地基）

HTTP 传输已实现（OpenAI-compatible + Ollama；见 `recraft-ai/transport.rs`），CLI 与 UI 共用同一
provider 层：

```powershell
recraft ai provider                      # 解析后的配置（不打印密钥）
recraft ai explain notepad.rcraft --addr entry --llm
recraft ai polish notepad.rcraft --addr entry
recraft ai prompt notepad.rcraft --addr entry [--max-tokens 1024] [--json]
```

- **默认离线**：`ProviderSettings` 默认 `heuristic`/`disabled`；启用 HTTP provider 必须同时
  设置 `RECRAFT_AI_PROVIDER=openai|ollama`、`RECRAFT_AI_ENDPOINT`，且
  `RECRAFT_AI_API_KEY`（变量名可配）非空，否则 `enabled=false`。UI 设置与文件配置同样遵守。
- **持久化**：`ProviderSettings::save()` 写入 `%LOCALAPPDATA%\ReCraft\ai-settings.json`；
  `load()` 优先读该文件，缺省回退环境变量，并按当前环境重算 `enabled`。
- **配置项**：`RECRAFT_AI_PROVIDER`、`RECRAFT_AI_ENDPOINT`、`RECRAFT_AI_MODEL`、
  `RECRAFT_AI_API_KEY_ENV`、`RECRAFT_AI_MAX_TOKENS`。
- **prompt 组装**：`build_explain_prompt`（函数摘要 + API 标签 + 引用字符串 + L2 伪代码）与
  `build_rename_prompt`（候选函数列表），都经过：
  - **脱敏**：URL → `<url>`、IPv4 → `<ip>`、邮箱 → `<email>`（`redact`，保留行结构）；
  - **token 预算**：约 4 字符/token，超限截断并标注 `…[truncated]`；
  - **缓存**：`PromptCache`（prompt hash → response）；**审计**：`AuditLog`
    （provider/函数/prompt tokens/response tokens/cached）。
- 传输层：`Transport` trait；`TcpHttpTransport` 处理**明文 http://**（本地 Ollama
  `http://127.0.0.1:11434`），`UreqTransport`（ureq + rustls）处理 **https://**
  （OpenAI 官方端点、云厂商兼容 API）；`transport_for(url)` 按 scheme 选择。
  `MockTransport` 用于无网络单测。
- Provider：`OllamaProvider`（`POST /api/chat`, `stream:false`）与
  `OpenAiCompatibleProvider`（`POST {endpoint}/v1/chat/completions`，Bearer key 从
  配置的环境变量读取）；`provider_from_settings` 在 disabled/heuristic 时返回
  `HeuristicProvider`（拒绝假装是模型）。
- 使用：`recraft ai explain notepad.rcraft --addr entry --llm` 会在启用 provider 后把
  脱敏 prompt 发送出去，并打印模型回答（`--json` 含 provider/tokens/response）；默认配置
  下会明确报 "no LLM provider configured"。

## 结果缓存与审计（M3）

- 路径：`%LOCALAPPDATA%\ReCraft\ai-cache.json`、`%LOCALAPPDATA%\ReCraft\ai-audit.jsonl`
  （无 LOCALAPPDATA 时回退临时目录）。
- **缓存 key** = 模板版本 + provider + model + system/user prompt 的哈希；命中时不再发起网络
  请求（`ai explain --llm` 输出 `cached=yes`）。`--no-cache` 跳过读取但仍写回。
- **审计**：每次 provider 交互追加一行 JSONL：`ts/provider/model/template/function/
  prompt_tokens/response_tokens/cached`；写入失败只告警，不影响回答。
- 回归：`scripts/ai-cache-smoke.ps1` 用本地 mock Ollama 服务验证「首次 miss → 二次 hit →
  审计 2 行」，并已加入 CI；单测覆盖缓存落盘往返、key 组成、审计追加。

## Prompt 模板与评测基座（M3）

- **模板版本**：`PROMPT_TEMPLATE_VERSION = "rc-ai-2026-10-v1"`；`EXPLAIN_SYSTEM` /
  `RENAME_SYSTEM` 是导出的模板常量，改文案需同步 bump 版本与评测期望。
- **离线评测**（无需模型/网络）：`crates/recraft-ai/src/eval.rs`
  - `EVAL_VERSION = "prompt-eval-2026-10-v1"`，内置 corpus 用例（`field_probe`、
    `array_probe`、`str_probe`、`float_math`、`branchy`）；
  - 检查点：prompt 必须包含函数证据（API/字符串/伪代码片段）、**不含裸 URL**（脱敏生效）、
    token 估算不超过预算；
  - 运行方式：
    ```powershell
    cargo test -p recraft-ai --test prompt_eval
    recraft ai eval corpus-project.rcraft [--json]     # 5/5 passed，失败时非 0 退出
    ```
- 这只保证 **prompt 输入质量**；LLM 回答的准确率/命名质量仍需模型评测集（后续版本）。

## 限制与后续

- **单条选择**：UI 当前一次应用一条 proposal；批量勾选/全选与 diff 预览（before/after
  名称对比）在后续版本补全。
- **无评测集**：LLM 输出的命名准确率/解释质量尚无固定 prompt 回归集。
- **网络可达性**：开发环境仅放行部分域名；TLS 栈通过 `https://crates.io/` 的忽略测试
  （`cargo test -p recraft-ai https_tls_handshake_to_crates_io -- --ignored`）验证，
  `api.openai.com` 在本环境不可达，未做端到端 OpenAI 账号测试。
- 命名质量有限（类别前缀 + 地址）；接入 LLM 后应结合字符串/调用关系生成更自然的名字。
- 写工具仅 `rename`/`comment`；`set_prototype` / `define_function` / `apply_type` 尚未实现。
- 自然语言查询（"找所有引用 URL 的函数"）与伪代码润色未实现。
