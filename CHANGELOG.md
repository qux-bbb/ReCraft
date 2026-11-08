# 变更日志

本项目所有值得注意的变更都会记录在此文件中。

本格式基于 [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)，
并且本项目遵循 [Semantic Versioning](https://semver.org/spec/v2.0.0.html)。

0.1.0 是首个公开技术预览。下面按使用者能感知的能力组织；从 M0 骨架到本版本的**完整**
开发记录（每条修复、每次指标变化、每个负结果）归档在
[docs/CHANGELOG-0.1.0.md](docs/CHANGELOG-0.1.0.md)。

## [未发布]

### 新增

- **ELF64（x86-64）支持。** `recraft-format` 新增 ELF 加载器：ELF/程序头/节表、`.dynsym` +
  `.symtab`、`PT_DYNAMIC`（SONAME、`DT_NEEDED`）、GOT/PLT 重定位（`R_X86_64_JUMP_SLOT`、
  `GLOB_DAT`）→ 导入表、`.eh_frame` FDE 与符号大小 → 函数范围，符号名直接进入名字表。
  `import` 与 GUI 拖放按魔术字自动识别，`info` 显示 `ELF64 (x64)`，`Session` 记录
  `format=elf64` / `arch=x64`。`cargo test -p recraft-decompile --test elf_smoke` 用合成的
  ELF 跑通 import → analyze → decompile → lint（`main` 读作 `return 1;`），不依赖 Linux
  工具链。已知限制：仅 ELF64 小端 x86-64（ARM64 随 ARM64 lifter）、不解析 DWARF/IFUNC/
  符号版本、`.symtab` 是主要命名来源（stripped 二进制靠 `.eh_frame` 划范围）。

- **Linux ELF 语料与门禁。** `corpus/linux/functions.c` 是同一组形状的可移植 C 样本
  （30 个导出函数，外加 `printf`/`malloc`/`memcpy`/`getenv` 等真实 libc 调用，因此能看到
  PLT/GOT 重定位），`corpus/build-linux.sh` 用 `gcc -O2` 打成 `libcorpus.so` 并产出
  `strip --strip-all` 的副本。CI 的 Linux 作业现在会：编译语料 → 跑可移植 crate 的测试
  （含 ELF 的 lint 门禁与 stripped 二进制的 `.eh_frame` 函数发现）→ 生成/校验 ELF golden。
  API 行为分类补了 POSIX 对应项（`ptrace`、`fork`/`exec`、`dlopen`、`fopen`/`opendir`、
  `mmap`、`pthread_*`、OpenSSL）。

- **DWARF 行号表。** 新增 `recraft-format/src/dwarf.rs`：解析 `.debug_line`（DWARF 2–5，含
  v5 的 directory/file 表与 `.debug_line_str` 间接字符串），得到按地址排序的 `file:line`
  行。`Image` 增加 `debug_lines`（serde default，旧项目照常打开），ELF 加载器自动读取；
  `recraft-cli lines <project> [--addr] [--json]` 打印映射。ELF 语料新增 `-g` 构建
  （`libcorpus-debug.so`），CI 的 Linux 作业验证行表能指回 `functions.c`。
  类型信息（`.debug_info`）未做——行表只给位置，不给类型。
- **DecBench 对照可用。** 新增 `recraft-cli decbench <kit> -o <dir>`：读 DecBench 评测套件的
  `functions.json`，逐二进制 import/analyze 后按目标地址渲染 L2 伪 C，写成套件要求的
  `results/<bin>.c` + `results.json`。sample-set（223 个二进制、250 个函数）实测 147 个
  二进制产出、`package.py` 校验通过，耗时约 65 秒；指标含义、实测数字与差距见
  `docs/DECBENCH.md`。
- **Itanium C++ 名字可读了。** `recraft-common::demangle` 在 MSVC 与 Rust 之后新增 Itanium
  ABI 分支（`cpp_demangle`，MIT/Apache）：ELF 上的 `_Z…`、Mach-O 的 `__Z…` 现在显示为
  `recraft::square(int)` 这样的名字，并随符号名进入名字表。配套加了
  `corpus/linux/functions.cpp`（命名空间、类方法、重载、模板）与 CI 的 C++ 语料门禁
  （`tests/golden/elf-cpp`）。

### 修复

- **冷片段合并只对 PE 生效。** `merge_pdata_functions` 是 `.pdata` 的概念（把落到下一段里的
  冷片段接回原函数）；ELF 的范围来自 `.symtab`/`.eh_frame`，本来就精确，`analyze` 现在对
  非 PE 格式直接按原样使用，不再套用 PE 的合并启发式。
- **重叠段不再撞 UNIQUE 约束。** 真实 ELF 里 `.tbss` 与 `.bss` 同址（PE 也可能重叠），
  `Segments::from_image` 现在按地址排序后丢弃落在前一段内部的段（相邻段保留），修复了
  `UNIQUE constraint failed: segment.start` 导致整个二进制打不开的问题；DecBench 套件里
  另有 5 个 ELF64 因此从跳过变可用。
- **`recraft-cli … | head` 不再 panic。** Rust 默认忽略 `SIGPIPE`，管道被消费方提前关闭时
  `println!` 会以 broken pipe 失败并 panic（退出码 101）。现在 Unix 恢复 `SIGPIPE` 的默认
  行为（安静退出，状态 141），Windows 把 `os error 109/232` 的 stdout 写失败转成安静退出
  （状态 0）。CI 的 Linux 作业加了对应的回归检查。
- **`endbr64`/`endbr32` 不再算未建模指令。** Ubuntu 的 gcc 默认开 `-fcf-protection`，在
  每个函数入口插一条 CET 落点；提升器此前把它当 `__asm`，于是第一版 Linux 语料的
  覆盖率只有 **84.8%**（43 个函数几乎每个一条）。现在与 `nop` 一样只产生 fallthrough，
  并补了单元测试。

### 变更

- **加载层不再假设「PE 即全部格式」。** `recraft-format` 现在有格式中立的 `Image`、`Format`
  与 `Arch`，`Loader` trait 返回带格式与架构的 `LoadedImage`，`load()` 按魔术字分派（PE 是
  唯一实现，其他格式报 `unrecognised executable format`）。项目把 `format` 与新增的 `arch`
  写进 `meta`，镜像 JSON 的键从 `pe_meta` 改为 `image_meta`，旧项目仍从 `pe_meta` 读取，
  不需要重新导入。这是一次零输出变化的重构：789 个 golden 快照、`lint` 与反编译文本不变。
- **CI 增加 Linux 作业。** `core crates (Linux)` 在 `ubuntu-latest` 上跑 11 个可移植 crate 的
  测试，用来持续证明 ADR-009 的可移植性；它也是 ELF 语料（只能在 Linux 上编译）的前置条件。
- **调用约定改为按架构查表。** `recover_l2` 不再用 `image().is_64` 二分，而是按
  `session.arch()` 查表：x86 仍靠尾声的 `ret imm16` 区分 cdecl/stdcall，x64 与将来的
  AArch64 用无关键字约定。`recraft-core` 重导出 `Arch`/`Format`/`Image`/`LoadedImage`，
  反编译器因此不必直接依赖加载层。
- **lint/渲染改为只读快照 + 按函数并行。** 反编译渲染现在只依赖只读的 `SessionView`
  （`Session` 与新增的 `SessionSnapshot` 各实现一次；快照预读段字节、名字、原型、字符串
  元数据并实现 `Sync`），`recraft lint` 用 rayon 按函数并行。同时把 `DecodeCache` 命中时
  O(缓存大小) 的 LRU 扫描改成 O(1) 查找 + FIFO 淘汰。Go 3261 个函数的全量 L1 自检
  **19 分 39 秒 → 约 2.7 秒**，L1+L2 约 14 秒；notepad 54.8 s → 8.5 s、crackme 40.5 s →
  0.5 s；五套 lint 输出逐字节不变，789 份 golden 快照不变。

## [0.1.0] - 2026-11-08

首个公开技术预览：Windows PE 静态分析 + 自研反编译器 + IDA 风格原生界面 + 内置 MCP
服务器 + AI 辅助。

### 摘要

- **分析**：PE32/PE32+（x64 + Win32），函数发现、字符串、四类交叉引用、API 行为标签、
  Go/Rust/C++ 运行时符号恢复。
- **反编译**：自研 LIR → SSA-lite → 结构化伪 C；L2 带签名、ABI 参数、局部变量、结构体
  字段与数组下标。输出的每个 token 都能映射回指令地址。
- **质量门禁**（全部可复现）：corpus 结构化 100% / 语义覆盖 97.4–97.6%；notepad
  结构化 **494/506（97.6%）** / 覆盖 99.6%；crackme 460/495（92.9%）；Go 2998/3261
  （91.9%）；**789** 份 golden 快照 + 六条输出不变量。
- **界面**：egui 原生 IDA 风格工作区，中英双语，布局/主题/缩放持久化。
- **命令总线**：每次编辑（重命名/注释/原型/类型）都记入日志，可撤销，可批量原子应用。
- **MCP**：stdio 与 HTTP 两种传输，只读工具 + `--write` 写工具，分页、结构化结果、
  资源与 prompt 模板。
- **AI**：解释、命名、摘要、自然语言查询、伪代码润色，以及项目级 agent；**只建议，
  不静默写入**。
- **CLI**：全部功能可用并带 `--json`。
- **发布**：便携 zip + WiX v5 MSI + SHA256SUMS，配套校验脚本与 CI。

### 新增

**分析与格式**

- PE32/PE32+ 加载：节、导入、导出、TLS 回调、`.pdata` 展开信息；畸形输入不 panic
  （确定性畸形回归 1 300 例）。
- 函数发现：入口、导出、TLS、`.pdata`、调用目标、跳转表、Go `pclntab`、扫描兜底。
  间接调用不再截断函数扫描，notepad 由此从 502 增到 **506** 个函数。
- 字符串与交叉引用：code/data/import/offset 四类，字符串按 ASCII 与 UTF-16LE 解码。
- API 行为分类（网络、持久化、注入、回调、反分析）与 `ai triage` 初筛。
- 增量重分析 `reanalyze --addr A --len N`：只重算区间内函数，区间外逐字节不变。

**反编译**

- L0 平铺 / L1 结构 / L2 类型三层；L2 恢复签名、参数（x64 四个寄存器 + 栈参数）、
  局部变量、结构体字段与 `byte_140018380[i]` 形式的数组访问。
- 局部变量重建：notepad 含寄存器名的行从 21 037 降到 **146**（严格口径，−99.3%），
  477/502 个函数体内不再出现机器寄存器名（`__asm` 除外）。
- 控制流恢复：`if`/`else if`/`while`/`do-while`/`switch` 重建、共享尾块复制、循环双出口
  节点分裂、后支配 join、常量与地址折叠。
- CRT 命名：内置 8 条静态 CRT 签名（`printf`/`puts`/`fgets`/`fflush`/`strpbrk`/`strcmp`/
  `strlen`/`__acrt_iob_func`）、`signature-export`/`signature-import`（函数头字节模式，
  地址字节通配）、启动代码识别（`main`/`WinMain`/`DllMain`）。
- 内联汇编大幅收敛：`bsf`/`bsr`/`popcnt` → `ctz`/`clz`/`popcnt`，`cdq`/`cqo`/`cwd`、
  `imul`、`lock xadd`、`sbb`、`bts`/`btr`/`btc`、`cmpxchg` 等提升为 C；notepad
  `__asm` 138 → **104** 处。
- L2 非法 C 清零：`while … else`、循环外 `break`/`continue`、`return __asm` 均为 0。

**界面**

- IDA 风格工作区：菜单栏、工具栏、导航带、Functions、图概览、Output、状态栏；
  统一深色调色板（面板 `#1e1e1e`、视图 `#141414`）与 Consolas/Segoe UI/微软雅黑字体。
- 文档标签：反汇编、反编译、十六进制、字符串、局部类型、导入、导出、AI；Names、Xrefs、
  Segments 默认隐藏但可从 Views 打开。
- 视图操作：`Space` 图/文本切换、`Tab` 在反汇编与伪 C 之间往返、`F5` 反编译、
  `X` 交叉引用对话框、`Ctrl+P` 命令面板、`Ctrl+W` 关闭项目、`Ctrl+=`/`-`/`0` 缩放。
- 语法着色：反汇编的助记符/操作数/寄存器/字面量/字符串分色，伪 C 与 AI 润色结果同样着色。
- 图视图：分层 CFG 布局、按边种类着色、Fit/缩放、可点击的概览迷你地图。
- 字符串视图：IDA 式 Address/Length/Type/String 四列、斑马行、跟随光标；字符串窗口滚动
  按地址空间行走（指令、`db` 字面量、原始 `db` 三种行）。
- 十六进制视图按 IDA 转储布局（16 位地址、紧凑大写字节、ASCII 尾部）。
- Fluent 本地化（`en-US.ftl` / `zh-CN.ftl`，148 条消息），语言写入 `layout.json`。
- 拖放打开 PE 或 `.rcraft`、最近项目、欢迎页在无项目时始终在前台。
- 中英混排基线对齐修复，底部状态栏统一等宽。

**存储与命令总线**

- SQLite + WAL；用户数据与派生数据分离，派生数据可随时重建。
- 日志化命令总线：所有写入带 `origin`（`gui`/`cli`/`mcp`/`ai`/`user`）与 `batch`，
  `undo` 还原最近一条或整个批次并报告还原内容。
- 伪代码缓存：按"可能改变输出的全部输入"取指纹，shell32 上 14 768 行的函数重复读取
  从 9.8 s 降到 **16 ms**。

**MCP**

- stdio 与 streamable-HTTP 两种传输：bearer token（常数时间比较）、非回环 `Origin` 拒绝
  （DNS 重绑定防护）、每个连接独立 `Session`，慢反编译不再阻塞其他请求。
- 只读工具：`list_functions`、`list_strings`、`xrefs_to`、`xrefs_from`、`callgraph`、
  `search_bytes`、`list_exports`、`get_metadata`、`get_pseudocode`、`function_summary`、
  `find_api_callers` 等；全部支持 `offset` 分页并报告 `next_offset`。
- `serve-mcp --write`：`rename`、`set_comment`、`set_prototype`、`apply_type`、
  `define_function`、`undo`、`batch`；每次写入走 GUI 同一条命令总线，可撤销。
- 工具带 `title`/`annotations`/`outputSchema`，结果附 `structuredContent`；提供
  `triage_sample`、`explain_function`、`propose_names`、`ctf_outline` prompt 模板与
  资源订阅通知；协商 `2024-11-05` / `2025-06-18` 协议版本。

**AI**

- 函数解释、重命名建议、样本摘要、自然语言查询、伪代码润色；结果流式输出并显示思考过程
  （可折叠）。
- 项目级 agent（`ai ask`）：在 13 个只读工具上多轮取证，回答跨函数问题，工具轨迹可展开。
- provider：内置 DeepSeek、OpenCode Go，支持任意 OpenAI 兼容端点与本地 HTTP；密钥用
  Windows DPAPI 按当前用户密封保存。
- prompt 脱敏、结果缓存、调用审计、token 预算与 `ai eval-names` 命名基准（含人工别名集）。
- 写入永远是建议：重命名/原型经用户确认后走命令总线。

**命令行**

- `import`、`analyze`、`disasm`、`decompile`、`strings`、`imports`、`funcs`、`xrefs`、
  `cfg`、`search`、`callgraph`、`metrics`、`lint`、`golden`、`export-pseudo`、
  `signature-export`/`-import`、`serve-mcp`、`ai`、`batch`、`reanalyze`，全部支持 `--json`。
- `golden` 快照 + `lint` 六条输出不变量（goto 有标签、标签唯一、括号平衡、无条件空、
  签名声明的 `argN` 齐全、`loc_` 不在签名之上），作为改动护栏。

**打包与合规**

- 便携 zip（含整棵 `docs/`）与 WiX v5 MSI：按机器安装到 `%ProgramFiles%\ReCraft`、
  `.rcraft` 关联、App Paths、开始菜单、VC++ 运行库启动检查。
- `SHA256SUMS.txt`、`verify-package.ps1`（哈希/条目/包内二进制全链路）、
  `verify-msi.ps1`（结构/载荷/安装→CLI→卸载）。
- CI：fmt/clippy/test、corpus 质量门禁、e2e、1 GiB 性能门禁、文档链接检查、zip 与 MSI
  两个打包作业。
- 许可为 **AGPL-3.0-only**（ADR-013，移除商业许可与 CLA），贡献走 DCO；SPDX 头检查、
  `cargo-deny` 许可证/禁用/来源策略、`NOTICE.md` 组件清单。

### 修复

- **反编译器语义错误**（均带回归测试与 golden）：循环双出口丢弃"搜索耗尽"边；残留基本
  块落空进无关代码；共享尾块复制改写条件跳转；常量折叠越过定义支配；条件跳转的
  fallthrough 被假定为下一条基本块（crackme `main` 曾在循环耗尽路径打印 "Correct!"）；
  `while (1)` 循环体落空时缺 `break`。
- **界面**：无项目时反汇编标签（欢迎页）被 `layout.json` 恢复的标签盖住；AI 面板内容被
  裁到面板下方而"点了没反应"；中英混排基线错位 3 px；数据视图向上滚动时字符串变成
  反汇编垃圾。
- **MCP**：`filter` 在 `limit` 之后执行导致匹配被静默丢弃；`get_pseudocode` 单次返回
  1.5 MB 撑爆客户端；HTTP 令牌比较可被时间侧信道。
- **AI**：DeepSeek 预置模型 id 已退役；润色在官方端点等约一分钟才出首个 token；
  OpenCode Go 推理模型润色超时。

### 变更

- NSIS 安装器整体移除，改为 WiX v5 MSI（单一安装器 + 便携 zip 覆盖免安装场景）。
- AI 改为纯侧栏助手（无 AI 停靠标签），移除离线/启发式 provider：未配置 LLM 时明确拒绝
  运行，而不是回退到规则式结果。
- 默认文档标签顺序调整为 反汇编、反编译、十六进制、字符串、局部类型、导入、导出、AI。
- 主视图更名：合并视图 → **反汇编**，伪代码窗格 → **反编译**。

### 已知限制

- **多入口循环**（不可规约流图）仍需 CFG 级重写：notepad 有 12 个函数保留 `loc_`/`goto`
  （其中 `sub_140013154` 一个占 371 处），Go 的 263 个回退几乎都在 runtime 内部。
  IDA 在这些形状上同样输出 `goto`。
- 仍有内联汇编：notepad 104 处，集中在原子指令、间接 `jmp` 与未建模标志的分支。
- 只做静态分析：**没有调试器**、断点或动态跟踪（M5 规划中）。
- 只支持 Windows PE：**没有 ELF / ARM64**，也没有 IDA 插件、IDAPython 或 `.idb` 兼容。
- **不保证伪代码可编译**：可读性与地址映射优先，需要补 `extern` 声明与类型。
- 未做代码签名：SmartScreen 会提示"未知发布者"，请用 `SHA256SUMS.txt` 校验。
- AI 需要自备 provider（HTTPS 或本地 HTTP），仓库不携带任何模型或密钥。

### 验证

发布前在本机与 CI 上重跑（2026-11-08，Windows 10 x64 / MSVC 19.4x）：

- `cargo fmt --check`、`cargo clippy --workspace --all-targets -D warnings`、
  `cargo test --workspace`：全绿。
- `recraft-cli lint`：notepad 506 函数 / 1 012 段文本、crackme 495 / 990 段，
  **0 findings**。
- `recraft-cli golden --check`：**789/789** 通过（notepad 506、crackme 160、corpus 31、
  corpus86 32、Go 60），0 mismatch。
- `recraft-cli metrics`：corpus x64 31/31 结构化、覆盖 97.4%；x86 32/32、97.6%。
- `scripts/package.ps1` + `verify-package.ps1` + `verify-msi.ps1`：zip、MSI、哈希与
  包内二进制全链路通过。

[未发布]: https://github.com/qux-bbb/ReCraft/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/qux-bbb/ReCraft/releases/tag/v0.1.0
