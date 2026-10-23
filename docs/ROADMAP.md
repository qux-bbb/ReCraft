# 路线图（DESIGN §14 摘要）

> 状态更新于 2026-10-24：M0–M3 完成，M4 功能与产物验证完成，仅剩两项外部依赖项
> （安装器实测需要可提权的干净虚拟机；代码签名需要证书）。详细设计见
> [DESIGN.md](DESIGN.md)，里程碑验收记录见 [M4.md](M4.md)。

| 里程碑 | 内容 | 状态 |
|---|---|---|
| M0 | 工作区/存储/PE 导入/反汇编基线、评测脚手架 | ✅ 完成 |
| M1 | 函数发现 + 字符串/xref/API 标签、CLI/UI 基线、undo/redo | ✅ 完成 |
| M2 | IR/lifter 语义覆盖、L1 结构化、L2 签名/ABI/类型、MCP 只读工具 | ✅ 完成（corpus 覆盖 97.4%、结构化 100%；notepad 覆盖 99.4%、结构化 86.1%） |
| M3 | 质量门禁/golden、AI（解释/命名/摘要/UI propose-apply/http(s) provider/prompt 评测/缓存审计）、1 GiB 性能门禁、打包 | ✅ 完成 |
| M4 | 覆盖率冲刺、安装包验证、文档/演示、公开技术预览 | ✅ 除外部依赖项（安装器实测、代码签名）外完成；zip + SHA256SUMS 为正式产物 |
| M5+（预览后） | 动态调试、ELF/ARM64、插件/脚本 API、团队协作 | 规划中 |

## DESIGN 差距清单（v0.1 预览前）

已达成（可验证）：

- 反编译质量门禁：corpus 覆盖率 97.4% / 结构化 100%，真实样本 notepad 覆盖率 99.4% /
  结构化 86.1%（阈值 95% / 70%），golden 31 份快照 + CI 回环。
- 调用参数与类型：x64/x86 实参、栈参数恢复、原型覆盖与指针 cast、间接调用/IAT 尾调用。
- MCP：stdio + HTTP（bearer token），只读工具/资源，单测 + E2E 真实服务断言。
- CLI：import/analyze/disasm/decompile/strings/imports/funcs/xrefs/cfg/search/callgraph/
  metrics/golden/batch/serve-mcp/ai 全部可用并带 `--json`。
- 发布：绿色 zip（10.2 MiB / 23 条目）+ SHA256SUMS，`verify-package.ps1` 自动校验，
  `demo.ps1` 在 release 产物上 PASS，包内 exe 复现源码指标。

仍存在的外部/已知差距（不阻塞代码完成）：

1. **安装器实测**：本会话无交互式 UAC，`makensis` 不可用；NSIS 脚本已备，需在允许提权的
   干净 Win10/Win11 上执行安装→启动→卸载（步骤见 [RELEASE.md](RELEASE.md)）。
2. **代码签名**：需证书；当前 SmartScreen 会提示。
3. **i18n 选型偏差**：DESIGN 指定 fluent/rust-i18n，实现为自研 `t()` 双语表（功能等价，
   EN + zh-CN 完整）。
4. **UI 渲染快照 / README 截图**：无头 egui 交互测试已覆盖，截图类素材尚未生成。
5. **增强项（非 v1 门槛）**：真正 SSA/phi、共享外键/嵌套结构体布局、`cargo-fuzz`
   （当前用确定性畸形回归替代）、增量分析（API 已有，MCP/UI 未暴露）。

## 后续优先项（预览后）

1. 动态调试与断点/跟踪（M5）。
2. ELF / ARM64 支持，插件/脚本 API（自有 API，不做 IDAPython 兼容）。
3. SSA/phi 与更强的类型推断，提升真实样本结构化率与可读性。
4. 自动更新、MSIX 打包与签名流水线。

## 明确不做（v1）

- IDA 数据库/插件/IDAPython 兼容。
- 保证伪代码可编译。
- 云端多租户/在线协作。
- 样本上传或云端分析（AI 默认离线，显式配置才会发送脱敏 prompt）。
