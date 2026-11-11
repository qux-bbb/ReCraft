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

## 明确不做（v1）

- IDA 数据库/插件/IDAPython 兼容。
- 保证伪代码可编译。
- 云端多租户/在线协作。
- 样本上传或云端分析（prompt 只在显式配置的 provider 上发送，且先脱敏）。
