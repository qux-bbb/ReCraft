# FAQ / 十问十答

## 1. ReCraft 是 IDA 的替代品吗？

不是兼容替代品。它是独立实现（clean-room，不使用 IDA SDK/数据库/签名库），目标是覆盖
恶意样本初筛与 CTF 场景的常用工作流。没有 IDAPython、没有 `.idb` 兼容、没有插件生态。

## 2. 反编译质量到什么程度？

伪代码是**可读但不保证可编译**的 C-like 输出，每个 token 可回溯到指令地址（DESIGN §7）。
corpus 基准：语义覆盖 97.4%、结构化成功率 100%；真实样本（notepad）覆盖 99.6%、
结构化 97.6%（494/506），crackme 92.9%（460/495），Go 二进制 91.9%（2998/3261，覆盖率 98.8%）。
复杂异常处理/不可归约控制流会回退到 `goto` 标签，这是设计
允许的降级路径。

## 3. AI 会把我的样本/伪代码发到云端吗？

默认**不联网**：只有真正调用模型的路径才需要先显式配置 provider（DeepSeek / OpenCode Go /
自定义 OpenAI 兼容端点）、endpoint 和 API key——GUI 的**解释**/**建议命名**/**润色**三个按钮，
以及 CLI 的 `ai explain --llm`、`ai suggest --llm`、`ai polish`；未配置时会提示先配置。
CLI 不带 `--llm` 的 `ai explain`/`ai suggest` 与 `ai query`/`ai triage` 全程本地。
配置后发送前仍会经过脱敏（URL/IP/邮箱）与 token 预算裁剪，并且可以用
`recraft-cli ai prompt` 先审阅。详见 [docs/AI.md](AI.md)。

## 4. AI 能直接改我的数据库吗？

不能静默修改。LLM/启发式只能产出带证据地址和置信度的 `Proposal`；用户 review 后
`ai apply`/UI 勾选，经 journaled Command 总线写入，可 `Ctrl+Z` 撤销（DESIGN §9.1）。

## 5. 支持哪些格式？

PE32+（x64）、PE32（x86），以及 ELF32（i386）与 ELF64（x86-64）。ARM64/固件/内存 dump 在
路线图上（见 [docs/ROADMAP.md](ROADMAP.md)）。Go/Rust/C++ 运行时符号恢复已支持（pclntab、
legacy hash、MSVC demangle）。

## 6. 1 GB 样本能打开吗？

能。CLI 门禁（1 GiB overlay 样本）：导入 < 60 s、分析 < 5 s、内存 ≤ 512 MiB；实测
导入约 4 s、分析约 0.3 s、峰值 5.6 MiB（debug 构建），见 [docs/PERF.md](PERF.md)。

## 7. 打开恶意样本安全吗？

ReCraft 只做**静态解析**，不加载、不执行、不解包样本；但解析器仍处理不可信输入，安全
问题请按 [SECURITY.md](../SECURITY.md) 报告。分析时请勿把样本提交到仓库（`.gitignore`
已排除常见二进制）。

## 8. 为什么是 AGPL-3.0-only？

反编译器本身就是「可审计性」的工具：AGPL 保证任何人拿到二进制都能拿到对应源码，改了也
必须回馈，这正是这类工具该守的规则。贡献按 inbound = outbound 接受（`git commit -s` 签
DCO），不需要 CLA，也不做版权转让。

## 9. 有 MCP 吗？怎么接 OpenCode？

有。`recraft-cli serve-mcp`（不跟项目）通过 stdio 提供反汇编、伪代码、xrefs、字符串、导入/导出、
镜像元数据、调用图、字节搜索、函数摘要等只读工具与项目资源，客户端用 `open_program` 打开任意
PE/ELF/`.rcraft`；加 `--write` 后再提供 `rename`、`set_comment`、`set_prototype`、`apply_type`、
`define_function`、`define_struct`、`batch`、`undo` 写工具（走命令总线，每步可撤销，支持
`expected_previous` 乐观并发检查）。带项目启动（`serve-mcp <project>`）仍可用，只是把该项目
预打开。HTTP 形态（`--http`）同样支持空注册表与会话内 `open_program`。
接入步骤与配置示例见 [docs/MCP.md](MCP.md)（`opencode mcp add` 或 `mcp.servers`），演示脚本见
[docs/DEMO.md](DEMO.md)。

## 10. 什么时候有公开预览？

已经发布了：**0.1.0（技术预览）** 的 zip / MSI 在
[GitHub Releases](https://github.com/qux-bbb/ReCraft/releases)，本版未签名（SmartScreen
会提示），请用 `SHA256SUMS.txt` 校验。里程碑见 [docs/ROADMAP.md](ROADMAP.md)，发布检查
清单见 [docs/M4.md](M4.md)。

## 11. 安装后 `recraft-cli` 提示“不是内部或外部命令”？

MSI 默认会把安装目录加入系统 PATH，装完直接可用。若安装时取消了勾选（或静默安装用
`ADDTOPATH=0`），还有两条路：开始菜单里的 **ReCraft 命令行**（打开即位于安装目录）、
`Win+R` 输入 `recraft-cli`（已注册 App Paths）。zip 免安装版不注册任何东西，直接运行
解压目录里的 `recraft-cli.exe`。

## 12. 卸载后为什么 `%LOCALAPPDATA%\ReCraft` 还在？

那里保存的是按用户的界面状态（`layout.json` 的停靠布局/主题、`recent.txt` 的最近项目）。
MSI 是按机器安装，卸载不会替每个用户删除个人数据；要彻底清理，卸载后手动删除该目录即可。
