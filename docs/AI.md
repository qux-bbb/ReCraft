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
```

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
- **Clear** 清空当前建议。

无项目/未分析时会提示先 Analyze。UI 不会在后台自动应用任何提案。

## Provider 抽象与 prompt 安全（M3 地基）

网络传输尚未实现，但 contract 已固定并可离线验证：

```powershell
recraft ai provider          # 解析后的配置（不打印密钥）
recraft ai prompt notepad.rcraft --addr entry [--max-tokens 1024] [--json]
```

- **默认离线**：`ProviderSettings` 默认 `heuristic`/`disabled`；启用 HTTP provider 必须同时
  设置 `RECRAFT_AI_PROVIDER=openai|ollama`、`RECRAFT_AI_ENDPOINT`，且
  `RECRAFT_AI_API_KEY`（变量名可配）非空，否则 `enabled=false`。
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
