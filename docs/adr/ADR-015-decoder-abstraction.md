<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# ADR-015: 解码抽象与 ARM64 解码器选型（保留 iced，新增中立指令边界）

- Status: Accepted
- Date: 2026-11-14
- Related: 待办「更换反汇编框架」（改法：不换 iced，加多架构解码抽象）、W5 ARM64 波次

## Context

`recraft-decode` 目前直接 `pub use iced_x86`，公开 API 返回 iced 的 `Mnemonic` /
`Register` / `OpKind` / `MemorySize`；`recraft-ir` 的 `lift.rs` 里有 29 处 iced 类型引用。
也就是说 **iced 已经是解码层的公开契约**：换掉它等于重写 lifter 的输入层，而 0.3.0 的
ARM64 本来就要写一个新的 lifter。

于是问题拆成两个：

1. x86 侧要不要把 iced 换成 capstone？
2. ARM64 侧用哪个解码器？

评估时间：2026-11-14；数据来自 crates.io / 各项目文档与仓库（链接见文末）。

## Decision

### 1. x86 保留 iced

iced 是原生 Rust（无 FFI、无 `cc` 构建、无 `unsafe` 绑定）、解码快、元数据完整
（RIP 相对地址、操作数读写访问、RFLAGS 读写的精确建模、指令长度/前缀）。这些正是
lifter 逐条依赖的东西。capstone 对 x86 没有功能收益，却要引入 C 工具链、unsafe 绑定、
额外的二进制体积与新的崩溃面（C 库里的段错误不受 Rust 的 `forbid(unsafe_code)` 保护）。

### 2. 在 `recraft-decode` 里加中立指令边界

新增中立的 `DecodedInsn`（助记符、操作数种类/寄存器/内存、分支目标、隐式读写、
长度），x86 后端把 iced 的 `Instruction` 适配成它，**对外 API 与输出逐字节不变**
（用现有 919 份 golden 与 `cargo test --workspace` 做回归）。lifter 逐步改为消费
`DecodedInsn`；ARM64 的 lifter 只依赖这个中立层，不依赖 iced。

`DecodedInsn` 只覆盖 lifter 真正需要的字段，不追求成为完整的指令数据库：宁可多一次
适配层里的 match，也不要把两个后端都钉死在一个大而全的 schema 上。

### 3. ARM64 用 yaxpeax-arm（capstone 备选）

| 维度 | yaxpeax-arm 0.5.0 | capstone 5.x | bad64 |
|---|---|---|---|
| 语言 / 构建 | 纯 Rust，`cargo` 直接构建 | C 库 + `cc` + unsafe 绑定 | Rust 绑定 Binary Ninja 的 C 插件（`cc` + unsafe） |
| 许可 | **0BSD**（宽松，无署名要求） | BSD-3-Clause | Apache-2.0（解码器本体） |
| 覆盖 | AArch64/A64（+ ARMv7/Thumb） | 多架构（ARM/ARM64/x86/MIPS/PPC…） | 仅 AArch64 |
| 元数据 | 解码结果含操作数与条件码；flags 语义要自己推导 | 操作数、隐式读写、分组信息更全 | 操作数与 IL（Binary Ninja 语义） |
| 体积 / 依赖 | 小（`bitvec` + `yaxpeax-arch`） | 静态链接 C 库，体积与构建复杂度更高 | 需链接 C 插件 |
| 维护 | 2026-09 仍在发版（0.5.0） | 活跃，工业级 | 绑定跟随 Binary Ninja 上游 |
| 风险 | flags 建模要自己补 | FFI 崩溃面、构建链 | FFI + 上游商业产品的构建链 |

**结论：ARM64 用 yaxpeax-arm**，理由按权重排序——纯 Rust 与现有 `forbid(unsafe_code)`
策略一致、0BSD 与 AGPL-3.0-only 分发无冲突、解码结果足够我们自己的 lifter 用（flags
本来就要在我们这边建模）、依赖面小。**capstone 作为备选**：如果 0.3.0 的 spike 发现
yaxpeax-arm 在分支目标/操作数元数据上缺口大到需要大量补丁，再切 capstone——切换点就是
`DecodedInsn` 适配层，代价可控。bad64 不选：它绑定的是 Binary Ninja 的 C 插件，构建链
与 FFI 风险最高，覆盖面还只有 AArch64。

## Consequences

- 0.3.0 的 ARM64 支持不碰 x86 的 lifter；x86 输出保持逐字节一致（golden 守门）。
- `DecodedInsn` 是唯一需要同时满足两个后端的地方；适配层的每个字段都要有 x86 侧的单测。
- 选型结论可回退：换 capstone 只需重写适配层，不动 lifter 与反编译器。
- 代价：多一层转换（每条指令一次），预期在解码缓存命中路径之外增加个位数百分比；
  0.3.0 spike 里用 notepad + 一个 ARM64 样本实测解码吞吐与端到端分析时间。
- 许可：0BSD 不需要在发布物里附署名；依赖与版本仍记录在本 ADR 与 `Cargo.lock`。

## Spike 验收（W5 开工前）

1. `DecodedInsn` 适配层落地后，x86 全量 golden 与 `cargo test --workspace` 逐字节不变；
2. ARM64 侧在真实样本（Windows on ARM 的 `notepad.exe` 或 Go 的 arm64 构建）上解出
   与 `objdump -d` 一致的 100 个函数；
3. 覆盖率 ≥ 60% 才承诺 ARM64 的 L2，否则反编译输出标 `beta`；
4. 解码吞吐：与 iced 的 x86 路径同量级（不要求更快，只要求不是瓶颈）。

## References

- [yaxpeax-arm on crates.io](https://crates.io/crates/yaxpeax-arm)（0BSD，0.5.0，2026-09）
- [Capstone](https://www.capstone-engine.org/)（BSD-3-Clause，C）
- [bad64](https://github.com/yrp604/bad64)（Binary Ninja arm64 解码器的 Rust 绑定）
- [iced-x86](https://github.com/icedland/iced)（MIT，本项目 x86 后端）
