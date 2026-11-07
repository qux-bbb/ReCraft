<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# ADR-014: MCP 写工具由 `--write` 显式开启，走 Command 总线可 undo

- Status: Accepted
- Date: 2026-11-07
- Supersedes: ADR-008 中「MCP 只读，写操作只走 propose/apply」的约束

## Context

ADR-008 让 MCP 服务器只读，写操作只产生 Proposal，由 UI/CLI 人工确认后应用。实际使用时，
agent 通过 MCP 做逆向需要直接改名、写注释、设原型；而 DSH、Claude Desktop 这类客户端没有
ReCraft 的 Proposal 审阅面板，两段式在这种情况下退化成「模型 propose 完自己再 apply」，
并没有多出一道人工确认。

## Decision

- `serve-mcp <project>` 默认仍然只读，只读契约不变。
- 新增 `--write`：显式开启写工具（`rename`、`set_comment`、`set_prototype`、`apply_type`、
  `define_function`、`undo`）。写工具与 GUI 共用 `Session::execute` 的 Command 总线，在同一个
  SQLite 事务里记录 inverse，因此每一步都可 undo，并留在 journal 里可审计。
- 进程级 `--write` 就是人工授权：由用户启动服务器时决定是否允许改库。
- AI 批量命名等仍保留 Proposal → UI diff → 人工确认的两段式；两条路径共用同一条命令总线。

## Consequences

- MCP 客户端（含 DSH）可以完成重命名/注释/原型等编辑，不必回到 GUI。
- 默认只读仍然成立；需要写时在 `serve-mcp`（或 DSH 配置的 `args`）里加 `--write`。
- 测试覆盖只读服务器拒绝写工具、写工具落库、undo 回退。
