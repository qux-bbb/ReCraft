# 路线图（DESIGN §14 摘要）

> 状态更新于 2026-11-08：M0–M4 的功能、产物与安装器验证完成，**0.1.0（技术预览）已发布**
> （zip + MSI + SHA256SUMS 在 GitHub Releases）。发布后仍待补的外部项只有代码签名（需证书）
> 与干净 VM 复验。详细设计见 [DESIGN.md](DESIGN.md)，里程碑验收记录见 [M4.md](M4.md)。

| 里程碑 | 内容 | 状态 |
|---|---|---|
| M0 | 工作区/存储/PE 导入/反汇编基线、评测脚手架 | ✅ 完成 |
| M1 | 函数发现 + 字符串/xref/API 标签、CLI/UI 基线、undo/redo | ✅ 完成 |
| M2 | IR/lifter 语义覆盖、L1 结构化、L2 签名/ABI/类型、MCP 只读工具 | ✅ 完成（M2 验收时：corpus 覆盖 97.4%、结构化 100%；notepad 覆盖 99.4%、结构化 86.1%） |
| M3 | 质量门禁/golden、AI（解释/命名/摘要/UI propose-apply/http(s) provider/prompt 评测/缓存审计）、1 GiB 性能门禁、打包 | ✅ 完成 |
| M4 | 覆盖率冲刺、安装包验证、文档/演示、公开技术预览 | ✅ 完成并已发布 **0.1.0**（zip + MSI + SHA256SUMS 在 GitHub Releases，结构与载荷脚本化验证）；代码签名与干净 VM 复验为发布后待补项 |
| M5+（预览后） | 动态调试、ARM64、插件/脚本 API、团队协作 | 规划中 |

## DESIGN 差距清单（v0.1 预览前）

已达成（可验证）：

- 反编译质量门禁：corpus 覆盖率 97.4% / 结构化 100%，真实样本 notepad 覆盖率 99.6% /
  结构化 97.6%（494/506），crackme 92.9%（460/495），Go 二进制 91.9%（2998/3261，覆盖率
  98.8%）；阈值 95% / 70%，golden **789** 份快照（notepad 全部 506 个函数）+ CI 回环。
- 调用参数与类型：x64/x86 实参、栈参数恢复、原型覆盖与指针 cast、间接调用/IAT 尾调用。
- AI（DESIGN §9.2 全部条目）：函数解释、命名建议、样本摘要、**自然语言查询**、**伪代码
  润色**（LLM 建议、不写回），provider 抽象/缓存/审计/prompt 评测齐备；命名准确率基准
  `ai eval-names`（`corpus/naming` 基准二进制，盲化候选符号）给出可回归的数字；**项目级
  Agent**（`ai ask`，13 个只读工具 + 工具轨迹）回答需要跨函数取证的问题。
- MCP：stdio + HTTP（bearer token），只读工具/资源，单测 + E2E 真实服务断言。
- CLI：import/analyze/disasm/decompile/strings/imports/funcs/xrefs/cfg/search/callgraph/
  metrics/golden/batch/serve-mcp/ai 全部可用并带 `--json`。
- 发布：绿色 zip（上次打包 13.9 MiB / 51 条目）+ SHA256SUMS，`verify-package.ps1` 自动校验，
  `demo.ps1` 在 release 产物上 PASS，包内 exe 复现源码指标。

仍存在的外部/已知差距（不阻塞代码完成）：

1. **安装器**：WiX 5 MSI 已生成，结构与载荷由 `scripts/verify-msi.ps1` 校验（`msiexec /a`
   解包逐字节比对）；安装→CLI→卸载流程需提权提示符或 CI msi job，发布前在干净
   Win10/Win11 各复验一次（按机器安装，需要一次管理员权限）。
2. **代码签名**：v0.1 决定暂不签名（SHA-256 校验 + SmartScreen 操作说明）；后续可申请
   SignPath Foundation（免费 OSS）或购买 OV 证书，再重打发布产物。
3. **UI 渲染快照 / README 截图**：无头 egui 交互测试已覆盖；README/DEMO 截图已按
   2026-11-07 界面刷新，动图尚未生成。
4. **增强项（非 v1 门槛）**：真正 SSA/phi、共享外键/嵌套结构体布局、`cargo-fuzz`
   （当前用确定性畸形回归替代）。

> i18n 选型偏差已关闭：UI 已迁移到 Fluent（`locales/*.ftl`），语言选择写入
> `layout.json`；剩余动态诊断日志仍为英文。

## 后续优先项（预览后）

1. 动态调试与断点/跟踪（M5）。
2. ARM64 支持，插件/脚本 API（自有 API，不做 IDAPython 兼容）。
3. SSA/phi 与更强的类型推断，提升真实样本结构化率与可读性（评估：全 SSA + phi 成本高、
   对 preview 收益有限；当前先以 SSA-lite + 可见噪声清理为主，列为预览后增强）。
4. 自动更新、MSIX 打包与签名流水线。

## 待办编排：0.2.0–0.4.0（2026-11-14 定）

16 项 open 待办按 6 个波次收口；**0.2.0 只装第一波**，每波结束都过一次发布链。

| 波次 | 内容 | 人日 | 版本 |
|---|---|---|---|
| W0 | 第 2 行定“删整行”、图标出 3 个方向、排期落库 | 0.5 | — |
| W1 | 字符串识别、ELF crackme 质量、签名库扩容、提示词可调、图标、第 2 行 | 14 | **0.2.0** |
| W2 | AI 自动命名、AI 对话（选中汇编/伪 C/数据） | 11 | 0.3.0 |
| W3 | 多入口循环（10 天硬停）、inline asm 收敛 | 15 | 0.3.0 |
| W4 | 中立解码抽象、ADR-015 | 5.5 | 0.3.0 |
| W5 | ARM64（先定 kill criteria） | 30 | **0.3.0** |
| W6 | UI 现代化、图标迭代、发起签名申请 | 15 | **0.4.0** |
| P | decbench 收口（W1 顺手） | 1 | — |

总量 ≈ 92 人日；按 DESIGN §14 的 3 天/周 ≈ 30 周，加 20% 缓冲 ≈ 37 周。

进度：

- **W1**：✅ 2026-11-14 收口，**0.2.0 已打包**（zip 15.5 MiB + MSI 13.9 MiB +
  `SHA256SUMS.txt`；`package.ps1` / `verify-package.ps1` / `demo.ps1` 全 PASS）。
- **W2**：两项（AI 自动命名、选中内容对话）已实现并提交待确认。
- **W3**：✅ 两项提交（多入口循环按时间盒收在 11 处并记录结论；inline asm 104 → 84）。
- **W4**：✅ 两项提交（ADR-015 选型 + 中立 `DecodedInsn` 边界，x86 输出零变化）。
- **W5**（进行中，2026-11-14）：解码器（yaxpeax-arm）与中立指令已接；ELF-ARM64 可导入、
  可反汇编、可发现函数（Go 的 arm64 样本：1904 函数、45417 条指令、1954 条调用边）。
  **反汇编正确性已达标**：`scripts/compare-arm64.ps1` 抽 100 个函数与 `go tool objdump`
  逐指令比对，地址序列 100/100 一致。
  **A64 lifter 第一版**：`recraft-ir::A64Lifter` 覆盖 mov/add/sub/and/orr/eor、
  ldr/str（含 ldur/stur）、adr/adrp、movz/movn/movk、cmp/cmn/tst、b/b.cond/cbz/cbnz/bl/ret，
  寄存器用 `Reg::A64Xn` 命名空间（与 x86 互不干扰）。`scripts/measure-a64-lift.ps1`
  实测前 100 个函数：**2308 条指令里 1929 条命中建模集合（83.6%）**，高于 kill criteria 的
  60% 线——但这是"lifter 能建模的指令占比"，不等于反编译成功率。
  **A64 伪 C 已通**：`SessionView::decoded_at` 让反编译器按架构取指令，
  `pseudocode_l0_aarch64` 用 A64 lifter 产 IR、复用同一套渲染尾（帧/名称/结构化/打印）；
  样本前 50 个函数全部输出非空伪 C（0 报错）。
  2026-11-14 第二轮打磨：A64 lifter 补了 `ldrb/ldrsb/ldrh/ldrsh/ldrsw/strb/strh`、
  `lsl/lsr/asr`、`neg`、`cset`/`csel`、`stp`/`ldp`、`nop`、`adrp`（页基址），条件分支改成
  比较真实操作数、内部目标落成 `goto loc_…`，负偏移打印成 `sp - 0xc0`；函数范围改用符号表
  的 end（分析不再在 `b` 处截断）。20 个函数的 `__asm` 从 **215 行降到 56 行**。
  仍待:`tbnz/tbz`（25）、`cset` 的跨块标志（16）、`ubfx`（5）、`csinc`（2）。
  2026-11-14 第三轮：补上 `tbz`/`tbnz`（位测试分支）、`ubfx`、`csinc`，并修 `cset` 的别名
  形式（解码器给的是 `csinc xD, xzr, xzr, cond`）；20 个函数的 `__asm` 降到 **8 行**，指令
  命中率 **83.6% → 96.3%**（100 函数 2308 条里 2223 条）。剩下 8 行是 SIMD 的 `stp/ldp v`
  （解码器未区分 SIMD 寄存器号）、`mrs`、`smulh`、`br`。
- 下一步：条件分支与块划分（把 A64 分支落成真正的 `if`/`goto` 而不是尾调用）、AAPCS64
  参数/返回、`stp`/`ldp`、然后是反编译覆盖率的正式测量与 golden。
- 下一步：W5 继续（lifter/AAPCS64），W6 的 UI 现代化与图标迭代排在其后。

规则：

- 每波结束过 `package.ps1` → `verify-package.ps1` → `demo.ps1`，W1 末发 **0.2.0**；
- W3 的两个时间盒任务不阻塞发版：到点没达标就记录结论、保留现状；
- W5 先写死 kill criteria（100 函数与 objdump 逐指令一致、覆盖率 ≥60% 才承诺 L2，否则反编译标 beta），再开工；
- 签名保持 2026-11-08 的暂缓决定，W6 开始时递 SignPath 申请，审批期不占开发时间；
- 单线程推进：W3 只能插在等 CI / 等审核的空档，不与 ARM64、UI 并行。

## 明确不做（v1）

- IDA 数据库/插件/IDAPython 兼容。
- 保证伪代码可编译。
- 云端多租户/在线协作。
- 样本上传或云端分析（prompt 只在显式配置的 provider 上发送，且先脱敏）。
