<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# ADR-008: MCP 原生；写操作 propose/apply，人工确认

- Status: Accepted
- Date: 2026-10-22
- Source: DESIGN section 16 / 15

## Context

LLM 不得静默改库。

## Decision

MCP 暴露只读工具/资源 + 写操作 Proposal（证据+置信度）；UI/CLI 人工 review 后经 Command 总线应用。

## Consequences

AI 输出可审计；MCP 服务器默认只读；测试覆盖 propose/apply 链路。

