<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# ADR-011: 大样本目标 1 GB，交互优先、后台导入/分析

- Status: Accepted
- Date: 2026-10-22
- Source: DESIGN section 16 / 15

## Context

恶意样本常见大体积；UI 不能卡死。

## Decision

导入/分析在后台线程，可取消；交互操作优先；预算：打开 <5 s、导入 <60 s、稳态 ≤512 MiB。

## Consequences

CI 1 GiB 性能门禁；实测导入 4.03 s、分析 0.32 s、峰值 5.6 MiB。

