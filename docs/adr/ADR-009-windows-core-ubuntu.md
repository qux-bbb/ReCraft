<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# ADR-009: Windows 首发，core 跨平台，Ubuntu 后置

- Status: Accepted
- Date: 2026-10-22
- Source: DESIGN section 16 / 15

## Context

单人 3 天/周，先聚焦恶意样本主战场。

## Decision

v1 仅 Windows x64/x86 PE；core 保持跨平台，CI 后续加 ubuntu-latest。

## Consequences

签名/安装器按 Windows 生态选型；ELF/ARM64 列入 M5+。

