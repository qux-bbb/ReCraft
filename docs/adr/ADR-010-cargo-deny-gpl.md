<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# ADR-010: 依赖白名单（cargo-deny），禁 GPL 系

- Status: Accepted
- Date: 2026-10-22
- Source: DESIGN section 16 / 15

## Context

AGPL + 商业双许可要求依赖许可兼容。

## Decision

允许 MIT/Apache-2.0/BSD/ISC/Zlib/MPL-2.0/Unlicense；禁止 GPL-2.0-only 与 GPL-3.0；CI cargo-deny。

## Consequences

新增依赖需过 deny.toml；NOTICE 汇总直接依赖。

