<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# ADR-007: AGPL-3.0-only + 商业双许可 + CLA

- Status: Superseded by [ADR-013](ADR-013-agpl-only-dco.md) (2026-11-04)
- Date: 2026-10-22
- Source: DESIGN section 16 / 15

## Context

开源协作与商业授权并行。

## Decision

代码 AGPL-3.0-only，商业许可另售；贡献需签署 CLA（非 DCO）。

## Consequences

依赖白名单受限（禁 GPL 系）；`scripts/check-license.ps1` 与 cargo-deny 双重门禁。

> 2026-11-04：本决策已被 [ADR-013](ADR-013-agpl-only-dco.md) 取代——许可改为纯
> AGPL-3.0-only，DCO 取代 CLA，商业许可入口移除。此处保留原文以记录历史。

