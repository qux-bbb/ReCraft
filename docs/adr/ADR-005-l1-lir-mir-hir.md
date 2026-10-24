<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# ADR-005: 自研 L1 反编译器（LIR→MIR→HIR），不集成外部实现

- Status: Accepted
- Date: 2026-10-22
- Source: DESIGN section 16 / 15

## Context

需要可控的质量门禁与自研路线（方案 C）。

## Decision

自有 IR/结构器/类型推断，L0/L1/L2 阶梯；外部反编译器仅用于人工对照。

## Consequences

质量可度量（corpus 覆盖 97.4%/结构化 100%；notepad 99.4%/86.1%）；成本高，需持续补语义。

