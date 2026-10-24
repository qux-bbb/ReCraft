<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# ADR-001: headless core + 命令总线 + UI 客户端

- Status: Accepted
- Date: 2026-10-22
- Source: DESIGN section 16 / 15

## Context

需要 CLI/UI/MCP 共享同一行为，且 AI/自动化可在无 UI 场景工作。

## Decision

core（Session/Command/undo）不依赖 egui；app 层提供用例；UI、CLI、MCP 都是客户端。

## Consequences

行为一致性由 core 测试保证；UI 可替换（ADR-002）；headless 测试与 CI 容易。

