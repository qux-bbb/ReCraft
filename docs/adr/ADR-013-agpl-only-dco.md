<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# ADR-013: 纯 AGPL-3.0-only，DCO 取代 CLA，取消商业许可

- Status: Accepted
- Date: 2026-11-04
- Supersedes: [ADR-007](ADR-007-agpl-3-0-only-cla.md)
- Source: DESIGN section 11 / 12 / 16

## Context

ADR-007 定的是「AGPL-3.0-only + 商业双许可 + CLA」。这套机器成立的前提是商业许可真的会卖
出去；而 CLA 存在的唯一理由，是拿到「把别人的贡献按其他条款再许可」的权利。

对本项目来说，净效果是负的：

- 商业许可入口没有价格、没有条款、也没有买家（`LICENSE-COMMERCIAL.md` 只有一个询价邮箱）；
- `CONTRIBUTING.md` 原句「Sign the CLA … required by the AGPL + commercial dual-license
  model」等于当面告诉贡献者：签这份 CLA 是为了让维护者能卖掉你的代码；
- `CLA.md` 自己还标着 **Draft**，连作者都没走完流程，却要求别人先签；
- 结果是每个想改代码的人，进门第一件事是签一份「允许商业转售我的贡献」的协议。

## Decision

1. 项目只以 **AGPL-3.0-only** 分发，不再提供商业许可（删除 `LICENSE-COMMERCIAL.md`）。
2. 贡献按 **inbound = outbound** 接受，用 `git commit -s` 签署 DCO；**不使用 CLA**，不做
   版权转让（删除 `CLA.md`）。
3. 依赖白名单不变（仍禁 GPL 系）：GPL-2.0-only 与 AGPL 不兼容；禁 GPL-3.0 是为了保留将来
   对自己代码重新许可的可能，与商业许可无关。

## Consequences

- 第三方贡献那部分**永久不可再许可**。作者自己写的代码仍归作者，理论上可另行授权，但任何
  包含他人贡献的版本都不能闭源分发。这是有意的取舍：贡献门槛优先于尚不存在的商业收入。
- 将来若确实要换许可（例如某个版本改用更宽松的条款），只能从那之后的新贡献开始要求额外
  授权；已合并的旧贡献无法追回。
- `scripts/check-license.ps1` 的必需文件清单去掉这两个文件；`deny.toml`、`Cargo.toml` 注释、
  `NOTICE.md`、README（中/英）、`CONTRIBUTING.md`、`docs/FAQ.md`、`docs/DESIGN.md` §11/§12/§16、
  `docs/M4.md`、`docs/RELEASE.md`、`docs/DEMO.md`、`docs/README.md` 与 ADR-002/ADR-010 同步改写。
