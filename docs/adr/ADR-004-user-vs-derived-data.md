<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# ADR-004: 用户数据与派生数据分离

- Status: Accepted
- Date: 2026-10-22
- Source: DESIGN section 16 / 15

## Context

分析结果可重建，用户编辑不可丢。

## Decision

用户编辑（名字/注释/原型/结构/类型/标记）经 journal 化 Command；分析输出写入派生表，可整表重建。

## Consequences

undo/redo 只作用用户数据；重新分析不会覆盖用户编辑；MCP 只读派生数据。

