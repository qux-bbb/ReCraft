<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# ADR-006: 不兼容 IDAPython；自有 API（CLI/MCP 先行）

- Status: Accepted
- Date: 2026-10-22
- Source: DESIGN section 16 / 15

## Context

避免继承 IDA 生态负担，聚焦 AI 原生工作流。

## Decision

不实现 IDAPython/IDC；先提供 CLI 与 MCP（stdio+HTTP），UI 使用同一 app API。

## Consequences

迁移成本由文档/兼容层外置承担；MCP 成为 AI 集成主通道。

