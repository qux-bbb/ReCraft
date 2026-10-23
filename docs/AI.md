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

- `apply` 把 `rename`/`comment` 映射为 `Command::Rename` / `Command::SetComment`，写入
  journal，可 `recraft undo` 撤销；`ai suggest` 本身永不写库。
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

## 限制与后续

- **尚无 LLM provider**：当前全部是确定性启发式；provider 只负责产出同形状 Proposal。
- **无 UI diff 面板**：M3 后续在 UI 中展示 proposals 列表（证据地址可跳转）→ 勾选 →
  apply；当前通过 CLI JSON review。
- 命名质量有限（类别前缀 + 地址）；接入 LLM 后应结合字符串/调用关系生成更自然的名字。
- 写工具仅 `rename`/`comment`；`set_prototype` / `define_function` / `apply_type` 尚未实现。
- 自然语言查询（"找所有引用 URL 的函数"）与伪代码润色未实现。
