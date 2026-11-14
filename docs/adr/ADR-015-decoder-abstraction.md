<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# ADR-015: 解码抽象与 ARM64 解码器选型（保留 iced，新增中立指令边界）

- Status: Accepted（2026-11-14 修订：ARM64 解码器改为 capstone）
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

> **已被 2026-11-14 的决策变更取代**：实际实现已切到 capstone，见文末「决策变更」一节。
> 以下表格与理由保留为当时的记录。

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

## 对照实验：capstone vs yaxpeax-arm（2026-11-14）

用户提出「yaxpeax-arm 的 star 太少」（GitHub 51 star，capstone 9,187）。这是维护风险而非
质量问题，所以先做了一次对照实验再决定（capstone-rs 0.13 + capstone 5，`full` feature）：

**样本**：Go linux/arm64 样例的 `.text` 段，154,029 个 4 字节字；另一个是仓库里的
32 函数 ELF 夹具（104 条指令，纯代码）。

| 指标 | yaxpeax-arm | capstone |
|---|---|---|
| 解码成功（raw `.text`，含数据） | **99.9%** | 97.3% |
| 解码成功（纯代码夹具） | 100% | 100% |
| 助记符一致（两者都解出） | 97.58%（3,626 处是 `mov` vs `orr` 别名命名） | — |
| 规范化后整句一致 | 47.86% | — |
| 依赖形态 | 纯 Rust / 0BSD | C 构建 + unsafe FFI / BSD-3 |

**顺带查出并修掉我们适配层的三个真 bug**（对照实验的主要收获）：

1. `xsp` / `wsp`：栈指针被拼成 `xsp`，正确应为 `sp` / `wsp`；
2. SIMD 寄存器只输出一个裸 `v`，丢掉编号与元素宽度，现在输出 `v3`、`v3.4s`、`v3.s[1]`、
   `{v2.2d, v3.2d, v4.2d}`；
3. `movk`（以及 `movz`/`movn`）的 16 位 lane 与 `lsl #0/16/32/48` 被折叠：lifter 把**每一段
   都当成 `<< 16`**，Go 二进制里常见的 64 位常量重建因此全错。现在按编码字恢复
   `imm16` 与 `hw*16`，伪 C 变成 `x6 = ((x6 | (0x38e3 << 0x20)) | (0xe38e << 0x30))`。

另外记录两处 capstone 的代价：capstone-rs 的 `Insn::mnemonic()/op_str()` 被
`cfg!(feature = "full")` 门控（即必须把所有架构一起编进来），且 0.14.0 开 `full` 会编译失败
（`ArchDetail` 的 E0392），只能退 0.13。

**当时的结论：保留 yaxpeax-arm**。理由：在同一份代码上两者的**识别能力等价**（纯代码 100%
一致），差异集中在文本格式与别名；yaxpeax-arm 的原始覆盖率更高，且纯 Rust 与
`forbid(unsafe_code)` 一致。

## 决策变更（2026-11-14）：ARM64 改用 capstone

用户看过上面的数据后仍然决定**切到 capstone**，理由是维护风险：yaxpeax-arm 只有 51 star、
单一维护者；capstone 有 9,187 star、被主流逆向工具长期使用，出问题时社区与文档都更厚。
这个理由优先于"纯 Rust"的洁癖，所以按用户决定执行。

落地内容（同一批提交）：

- `crates/recraft-decode`：移除 `yaxpeax-arm` / `yaxpeax-arch`，改用
  `capstone = "=0.13.0"`（`full` feature，C 构建由 `capstone-sys` 完成）；A64 适配层重写为
  capstone 的 operand detail → `DecodedInsn`，线程本地复用 handle（每条指令新建代价太高）；
- x86 仍是 iced，`DecodedInsn` 边界不变，lifter 与反编译器零改动；
- capstone 用别名（`lr`/`fp`/`ip0`/`ip1`）而不是编号寄存器，适配层做规范化回 `x30`/`x29`/
  `x16`/`x17`；`udf #imm` capstone 会拒绝解码，适配层按编码补回（分析器把它当作
  intrinsic 与 walk 终止符）；
- 验证：`compare-arm64.ps1` 100/100 函数与 `go tool objdump` 逐指令一致、
  `measure-a64-lift.ps1` 覆盖率仍是 **96.3%**（2223/2308，与 yaxpeax 时相同）、
  A64 的 32 份 golden 与全部测试不变。

代价（明确接受）：多了 C 工具链构建（MSVC/cc）与 unsafe FFI 面（隔离在 `capstone` crate
内，本仓库仍 `forbid(unsafe_code)`）；包体增长（capstone 静态链接）；capstone-rs 的文本
API 要求 `full` feature，把所有架构一起编进来。

## Consequences

- 0.3.0 的 ARM64 支持不碰 x86 的 lifter；x86 输出保持逐字节一致（golden 守门）。
- `DecodedInsn` 是唯一需要同时满足两个后端的地方；适配层的每个字段都要有 x86 侧的单测。
- 选型结论可回退：换解码器只需重写适配层，不动 lifter 与反编译器（这次切换就是这么做的）。
- 代价：多一层转换（每条指令一次）；capstone 另加 C 构建与静态链接体积。
- 许可：capstone（MIT 绑定 + BSD-3-Clause 引擎）与 AGPL-3.0-only 兼容，`deny.toml` 已允许
  BSD-3-Clause；依赖与版本记录在本 ADR 与 `Cargo.lock`。

## Spike 验收（W5 开工前）

1. `DecodedInsn` 适配层落地后，x86 全量 golden 与 `cargo test --workspace` 逐字节不变；
2. ARM64 侧在真实样本（Windows on ARM 的 `notepad.exe` 或 Go 的 arm64 构建）上解出
   与 `objdump -d` 一致的 100 个函数；
3. 覆盖率 ≥ 60% 才承诺 ARM64 的 L2，否则反编译输出标 `beta`；
4. 解码吞吐：与 iced 的 x86 路径同量级（不要求更快，只要求不是瓶颈）。

## References

- [yaxpeax-arm on crates.io](https://crates.io/crates/yaxpeax-arm)（0BSD，0.5.0，2026-09；已不再使用）
- [Capstone](https://www.capstone-engine.org/)（BSD-3-Clause，C）
- [bad64](https://github.com/yrp604/bad64)（Binary Ninja arm64 解码器的 Rust 绑定）
- [iced-x86](https://github.com/icedland/iced)（MIT，本项目 x86 后端）
