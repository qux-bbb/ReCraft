<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# ADR-003: 单文件 SQLite 容器 + 分块镜像 + 分页 flags

- Status: Accepted
- Date: 2026-10-22
- Source: DESIGN section 16 / 15

## Context

1 GB 样本需低内存、可撤销编辑与快速查询。

## Decision

项目为单个 SQLite（WAL）文件；镜像分块存储；每字节 flags 分页；xref 用行表（保留换方案的退路）。

## Consequences

1 GiB 导入 4.03 s、分析 0.32 s、峰值 5.6 MiB（PERF.md）；schema 迁移需幂等。

