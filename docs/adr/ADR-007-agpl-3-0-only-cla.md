<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# ADR-007: AGPL-3.0-only + 商业双许可 + CLA

- Status: Accepted
- Date: 2026-10-22
- Source: DESIGN section 16 / 15

## Context

开源协作与商业授权并行。

## Decision

代码 AGPL-3.0-only，商业许可另售；贡献需签署 CLA（非 DCO）。

## Consequences

依赖白名单受限（禁 GPL 系）；`scripts/check-license.ps1` 与 cargo-deny 双重门禁。

