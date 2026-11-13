# ReCraft 设计文档 v0.1

> 状态：已评审（2026-10-22 通过，进入 M0）
> 日期：2026-10-22
> 作者：Lead（与项目所有者讨论产出）

---

## 0. 已确认的决策

| # | 决策 | 结论 |
|---|---|---|
| 1 | 项目定位 | 开源产品，单人开发，约 3 天/周 |
| 2 | 核心场景 | 恶意样本静态分析、CTF；固件/补丁分析后置 |
| 3 | v1 目标平台 | Windows 首发；core 跨平台；后续至少 Ubuntu |
| 4 | v1 目标文件 | PE（x64 + Win32 x86）与 ELF（i386 + x86-64）；ARM64/固件后置 |
| 5 | 架构 | headless core + 命令总线 + MCP + UI 客户端 |
| 6 | UI | egui 原生（无 webview），现代观感，形态像 IDA |
| 7 | 反编译 | **自研 L1**，不集成 Ghidra / RetDec |
| 8 | 脚本 | v1 仅 CLI + MCP；插件/脚本宿主后置 |
| 9 | 动态分析 | v1 不做 |
| 10 | AI | 原生 MCP；LLM 命名、解释、自然语言查询 |
| 11 | 许可证 | AGPL-3.0-only；贡献 inbound = outbound（DCO），无 CLA |
| 12 | 大样本 | 目标 1 GB |
| 13 | 语言 | UI 英文 + 简中；代码/文档英文 |
| 14 | 分发 | GitHub 公开仓库 + Actions；Windows 绿色版 zip + 安装包 |
| 15 | 仓库 | GitHub 公开：`qux-bbb/ReCraft`（本轮已 `git init`） |
| 16 | 评审状态 | v0.1 已通过（2026-10-22），进入 M0 |

### 非目标（v1 明确不做）

- 调试器、动态插桩、沙箱
- 可编译的 C 输出（L1 只承诺"可读"）
- 跨函数类型推导、结构体自动识别
- IDAPython 兼容、IDA 插件/IDB 兼容
- FLIRT/签名库、BinDiff、多用户协作、云服务
- 加壳/混淆样本的全自动还原（仅尽力识别与降级）

---

## 1. 总体架构

```
┌──────────────────────── recraft-ui (egui) ────────────────────────┐
│ disasm · pseudocode · graph · hex · strings · imports · xrefs · AI │
└───────────────▲───────────────────────────────────────────────────┘
                │ Session 读 API（同进程；单测/远程时可换 JSON-RPC）
┌───────────────┴──────────────┐        ┌──────────────────────────┐
│ recraft-app（编排层）        │◄──────►│ recraft-mcp（工具/资源）  │
│ 后台分析 · 任务队列 · 设置   │        └──────────────────────────┘
└───────────────┬──────────────┘
                │ Command 总线（单写者、可撤销）
┌───────────────┴────────────────────────────────────────────────────┐
│ recraft-core：Session · DB · 地址空间 · flags · 名字/注释 · xref    │
│               funcs · types · journal · undo · 项目读写             │
└───┬──────────────┬──────────────┬──────────────┬───────────────────┘
    │              │              │              │
recraft-format  recraft-decode  recraft-analysis  recraft-ir
 (PE 解析)      (iced-x86 封装) (自动分析)        (语义 IR)
                                                 │
                                          recraft-decompile
                                          (CFG→SSA→结构→C)

recraft-common（地址/ID/错误/配置）被所有 crate 依赖
recraft-storage（SQLite 容器/分页/序列化）支撑 core
recraft-ai（LLM 客户端/提示/脱敏/预算）供 app/mcp/ui 使用
```

### 1.1 crate 划分与依赖方向

| crate | 职责 | 依赖 |
|---|---|---|
| `recraft-common` | 地址、ID、错误类型、配置、范围类型 | 无 |
| `recraft-storage` | SQLite 容器、分页 blob、图片导入、journal 存储 | common |
| `recraft-format` | PE 解析（段/导入/导出/TLS/资源/`.pdata`/重定位） | common |
| `recraft-decode` | iced-x86 封装、统一 `Insn` 模型、解码缓存 | common |
| `recraft-core` | Session、数据模型、Command 总线、undo、读写 | common, storage, format, decode |
| `recraft-analysis` | 递归下降、函数识别、跳表、字符串、Go/Rust、API 分类 | core, decode |
| `recraft-ir` | 语义 IR、x86/x64 提升、lazy flags | common, decode |
| `recraft-decompile` | CFG、SSA、优化、结构恢复、变量、类型、C 生成 | ir, core |
| `recraft-query` | 只读查询/工具层：MCP 与内置 agent 共用的工具 spec、执行器、预算 | common, core, decompile |
| `recraft-app` | 编排：后台任务、分析调度、设置、会话生命周期（尚未填充，见 §1.1） | core, analysis, ir, decompile, ai, mcp |
| `recraft-mcp` | MCP server（stdio/HTTP）、传输、写工具、资源 | core, query |
| `recraft-cli` | headless：analyze/strings/imports/decompile/search/export/serve-mcp | core, analysis, decompile, ai, mcp |
| `recraft-ui` | egui 应用、视图、自绘控件、i18n | core, analysis, decompile, ai |
| `recraft-ai` | provider 抽象、提示模板、脱敏、token 预算、缓存、项目级 agent、样本 triage | common, core, analysis, query |

依赖方向必须单向，`core` 不得依赖 `analysis/decompile/ir`。新增 crate 需在本文档 ADR 中登记。

---

## 2. 并发与命令总线

### 2.1 线程模型

| 线程/池 | 职责 |
|---|---|
| UI 线程 | egui 帧循环，只读快照 + 发命令；单帧预算 < 16 ms |
| 写线程（单） | 消费 Command 队列，串行写 DB + journal；分析结果批量写入也走这里 |
| 分析池（rayon） | 函数级并行分析、字符串扫描、反编译；只产出"结果批次"，不直接写 DB |
| 读连接池 | UI/CLI/MCP 查询（SQLite WAL 支持并发读） |

原则：**单写者 + 多读者**。UI 永远不持有写锁；任何阻塞 > 16 ms 的操作必须后台化。

### 2.2 命令总线

所有状态变更通过命令，命令是 undo 的单位，也是 MCP/CLI 的唯一写入口。

```rust
enum Command {
    Rename       { addr: u64, name: String },
    SetComment   { addr: u64, kind: CommentKind, text: String },
    DefineFunc   { start: u64, end: Option<u64> },
    UndefineFunc { start: u64 },
    SetCodeData  { range: Range, kind: CodeDataKind },
    SetType      { addr: u64, decl: String },
    ApplySig     { func: FuncId, decl: String },
    PatchBytes   { addr: u64, bytes: Vec<u8> },
    Undo,
    Redo,
}

trait CommandBus {
    fn submit(&self, cmd: Command) -> Result<Effect>; // Effect = 失效区域 + 新数据版本
}
```

- 自动分析产生的数据**不是命令**，是"派生数据批次"（见 2.3）。
- 撤销粒度：连续输入合并（评论/改名在空闲 500 ms 后合为一条）。
- 每条命令落 `journal` 表，带逆操作 payload；批量命令单事务。

### 2.3 用户数据 vs 派生数据（核心设计）

| 类别 | 内容 | 存法 | 可重建 |
|---|---|---|---|
| 用户数据 | 改名、注释、类型定义、函数定义、字节补丁、书签、分析配置 | journal + 表 | 否（必须保留） |
| 派生数据 | flags、自动 xref、字符串、函数边界、API 分类、伪代码缓存 | 表（可清空） | 是（image + 用户数据 + 配置 → 重算） |

推论：
- **崩溃恢复** = WAL + journal 回放；"另存为" = 复制 DB。
- **撤销**只回滚用户命令；受影响的派生区域标记 dirty，后台重算。
- **分析配置变更**（如新开"字符串最小长度"）可清派生表重跑，不影响用户数据。
- 导出可复现实验/回归：`image` + `journal` 就够。

---

## 3. 存储与数据模型

### 3.1 容器

单文件 `.rcraft` = SQLite 数据库（WAL 模式），输入镜像以分块 blob 存表内。

- 分块：`image_chunk`，256 KiB/块（1 GB ≈ 4096 行，顺序读友好）。
- 打开流程：读取头 → 校验 → 先映射原文件让 UI 可交互 → 后台导入镜像到 DB（带进度）。
- 打开指标：可交互 < 5 s；导入完成 < 60 s（1 GB，SSD）。
- SQLite 参数：page_size=65536、WAL、synchronous=NORMAL、cache_size=256 MiB、mmap_size=256 MiB。
- 两个连接：写连接（单线程独占）+ 读连接池（WAL 并发）。

### 3.2 Schema（v0.1 草案）

```sql
meta        (key TEXT PRIMARY KEY, value BLOB)
image_chunk (off INTEGER PRIMARY KEY, data BLOB NOT NULL)      -- 256 KiB 对齐
segment     (id INTEGER PRIMARY KEY, name TEXT, start INTEGER, end INTEGER,
             file_off INTEGER, kind INTEGER, perm INTEGER, bitness INTEGER)

-- 用户数据
name        (addr INTEGER PRIMARY KEY, text TEXT, kind INTEGER, flags INTEGER)
            -- kind: user/import/export/demangled/auto-weak
comment     (addr INTEGER, kind INTEGER, text TEXT, PRIMARY KEY(addr, kind))
func        (id INTEGER PRIMARY KEY, start INTEGER UNIQUE, end INTEGER,
             flags INTEGER, cc INTEGER, name_addr INTEGER, ret_type INTEGER)
func_chunk  (func_id INTEGER, start INTEGER, end INTEGER)
type_def    (id INTEGER PRIMARY KEY, decl TEXT, size INTEGER, kind INTEGER)
struct_member (type_id INTEGER, off INTEGER, name TEXT, type_id INTEGER)
var         (func_id INTEGER, stack_off INTEGER, name TEXT, type_id INTEGER, kind INTEGER)
patch       (addr INTEGER PRIMARY KEY, orig BLOB, new BLOB)
bookmark    (addr INTEGER PRIMARY KEY, note TEXT, color INTEGER)
journal     (seq INTEGER PRIMARY KEY, ts INTEGER, cmd TEXT, payload BLOB, undone INTEGER)
config      (key TEXT PRIMARY KEY, value BLOB)

-- 派生数据（可清空重建）
flags_page  (page_no INTEGER PRIMARY KEY, data BLOB)           -- 64 KiB 字节 → 32 KiB flags
xref        (from_addr INTEGER, to_addr INTEGER, type INTEGER,
             PRIMARY KEY(from_addr, to_addr, type))
string      (addr INTEGER PRIMARY KEY, len INTEGER, enc INTEGER, kind INTEGER)
api_tag     (addr INTEGER, rule_id TEXT, category TEXT)
pseudocode  (func_id INTEGER PRIMARY KEY, text BLOB, span_map BLOB, version TEXT)

CREATE INDEX xref_to ON xref(to_addr);
CREATE INDEX func_start ON func(start);
CREATE INDEX name_text ON name(text);
```

- flags 编码：每字节 4 bit（code/data/head/tail/extra），64 KiB 字节页 → 32 KiB blob，LRU 常驻上限 256 页。
- xref 规模预估：1 GB 样本 2–5 千万条；批量插入用单事务。若实测查询/写入不达标，退回"按目标地址分桶的 blob 链表"（ADR-003 已留后路）。
- `pseudocode` 是缓存，带版本号；IR/反编译器升级后按版本失效。

### 3.3 1 GB 性能预算

| 指标 | 目标 |
|---|---|
| 可交互 | < 5 s |
| 导入完成 | < 60 s |
| 常驻内存 | ≤ 512 MiB（基本不随文件增长） |
| 跳转/滚动延迟 | < 30 ms |
| 全量自动分析 | 后台、可取消、带进度；不阻塞 UI |
| 单函数反编译 | < 100 ms（典型函数） |

实测（2026-10-22，本机 NVMe，`recraft-storage` bench，1 GiB 镜像）：

| 操作 | 实测 | 目标 |
|---|---|---|
| 导入（含 SHA-256 + WAL） | **4.6 s** | < 60 s |
| 重开项目 | **3.1 ms** | < 5 s |
| 1000 次随机 4 KiB 读 | **34 ms**（≈34 µs/次） | — |
| 项目文件体积（1 GiB 镜像） | **1062 MiB** | 合理 |

注：bench 中镜像常驻内存；应用层接入后会改为流式/mmap 以降低峰值内存。flags/xref
的分页读写性能待 T6（命令总线）落地后补测。

---

## 4. PE 加载器

v1 支持范围（全部容错，坏数据不 panic）：

- DOS/NT 头、COFF、可选头、节表、重叠/越界节、未对齐、伪 PE
- 导入（IAT/延迟导入/绑定导入）、导出（含转发器/序号）、TLS 回调、资源目录
- `.pdata`（x64 异常表）→ **真实函数边界与展开信息**（重要的分析种子）
- 重定位、加载配置、调试目录（PDB 路径仅记录，不解析）
- SEH/C++ EH 结构识别（尽量）、vftable/RTTI（MSVC 布局）
- overlay/stub 识别、Checksum 忽略

容错策略：**结构性问题（头/节表）返回错误；导入/导出/TLS/`.pdata` 的畸形数据降级**——
保留已解析部分并记录 `warnings`，绝不因为一张坏表就拒绝加载恶意样本。未知目录先忽略。

接口：

```rust
pub trait Loader {
    fn detect(bytes: &[u8]) -> Option<LoaderInfo>;
    fn load(&self, bytes: &[u8], db: &mut ProjectDb) -> Result<LoadedImage>;
}
```

新增格式（ELF/Mach-O）只实现 `Loader`，不改 core。

---

## 5. 解码与指令模型

- `iced-x86`（MIT）负责解码；封装为自有 `Insn`：

```rust
struct Insn {
    addr: u64, len: u8, mode: Mode,          // 16/32/64
    mnemonic: Mnemonic, ops: SmallVec<[Operand; 4]>,
    flow: FlowKind,                          // Next/Call/Jmp/Ret/Int/Trap/Invalid
    reads: RegSet, writes: RegSet,
    mem_refs: SmallVec<[MemRef; 2]>,         // 供 xref 建链
    prefixes: PrefixFlags,
}
```

- 解码缓存：按 4 KiB 区域缓存，LRU（例如 4096 区）。
- 不合法指令：标记 `undefined`，不中断分析；用户可强制转数据。
- 16-bit 模式 v1 不支持（明确报错）。

实测（2026-10-23，stateless 逐条解码，NOP 密集）：**约 46 M insn/s**；接入缓存与
分支处理后，分析管线吞吐会低于此值。

---

## 6. 自动分析（v1）

### 6.1 种子与发现

- 入口点、导出、TLS 回调、`.pdata` 区间、异常处理函数、vftable 槽位
- 导入 thunk（IAT 调用点）、CRT 启动链（`__scrt_common_main_seh` → `invoke_main` → `main/WinMain`）
- 函数指针表/跳表（MSVC/GCC 常见模式）、对齐 + prologue 扫描（兜底）
- 尾调用、thunk、covariant thunk 识别
- Go：`pclntab` 多版本解析（1.2 / 1.16 / 1.18 / 1.20+）、`moduledata`、函数名与 `file:line`
- Rust：`rustc-demangle` + `lang_start`/panic 字符串/`main` 链；剥离符号时降级为启发式
- Delphi 等运行时后置

### 6.2 数据与引用

- code/data 判定 + 用户覆盖优先
- 字符串：ASCII/UTF-16LE/UTF-8 启发式；URL/IP/域名/路径/注册表键自动分类
- xref：代码引用、数据引用、偏移引用（`lea`/立即数）、字符串引用、导入引用
- 名字传播：导出名、导入序号、CRT 已知函数、demangle、运行时恢复名

### 6.3 行为分类（产品差异点）

规则数据驱动（YAML/JSON），内置一版恶意样本常用 API 分类：

- 网络：WinINet/WinHTTP/WinSock/`URLDownloadToFile`/`InternetOpen*`/`send`/`recv`
- 持久化：注册表 Run、计划任务、服务
- 注入/执行：`VirtualAllocEx`/`WriteProcessMemory`/`CreateRemoteThread`/`Nt*`
- 回调注册：`SetWindowsHookEx`/`EnumWindows`/`CreateThread`/线程池
- 反分析：`IsDebuggerPresent`/`CheckRemoteDebuggerPresent`/时序/VM 检测

输出：函数 → 标签 + 证据地址；UI 提供"网络相关"筛选，对应验收剧本第 4 步。

### 6.4 增量

- 用户改定义/改数据 → 标记脏区间 → 只重算受影响函数
- 后台任务可取消；结果按批次提交（每批 ≤ 1 万条 xref）

---

## 7. IR 与反编译器（项目主线）

### 7.1 三层 IR

| 层 | 形态 | 用途 |
|---|---|---|
| LIR | 指令级副作用：寄存器/标志/内存读写、临时值 | 提升、可插桩、AI 输入 |
| MIR | SSA 表达式树 + 基本块，已做拷贝/常量传播、DCE | 结构恢复、变量分析 |
| HIR | 结构化语句（if/loop/switch）+ 局部变量 + 类型 | C 生成、UI 展示 |

表示：arena + 索引（`ValueId`/`StmtId`），避免深拷贝；每个节点带 `source_addr`，**任何输出都能回溯到指令地址**（UI 高亮 + AI 引用）。

### 7.2 x86/x64 语义提升

- 语义表按助记符手写（核心 ~150 条覆盖 95% 代码；其余降级为 intrinsic/`asm` 保留原文，**永不失败**）。
- 寄存器视图/掩码：处理 AL/AH/AX/EAX/RAX 别名、写 32 位寄存器清零高 32 位。
- **lazy flags**：不物化 8 个标志，按使用点构造条件表达式；`cmp/test → jcc/cmov/setcc` 归一为比较。
- 栈帧：跟踪 `rsp/esp` 变化，识别帧指针省略（O2）、栈参数、局部变量槽。
- 调用约定：x64 MS ABI；x86 cdecl/stdcall/thiscall/fastcall（从符号/栈平衡/参数使用推断）。

### 7.3 管线

```
decode → LIR → 基本块/CFG → SSA(phi) → 优化 → 结构恢复 → 变量合并
        → 类型推断(基础) → HIR → C 输出 + span map
```

- 结构恢复：支配树 + 后支配；回边识别循环；跳表元数据驱动 `switch`；不可约图**降级为 goto + label**（必须能出结果）。
- 变量合并：SSA 变量 → 按"基变量"（寄存器/栈槽）合并；参数/返回值按 ABI。
- 类型（v1）：`void/int8..64/uint*/char*/void*`、数组、简单函数指针；**不做**结构体自动识别（设计好扩展点，L2 再做）。
- C 生成：优先级正确、表达式简化、可选地址注释；输出**不保证可编译**。

### 7.4 质量门槛（v1 定义"完成"）

| 指标 | 门槛 |
|---|---|
| 指令语义覆盖率（语料中） | ≥ 95%（其余走 intrinsic 降级） |
| 结构化成功率（无 goto fallback） | ≥ 70% |
| 表达式完全重建率 | ≥ 80% |
| 变量/参数识别（50 函数人工审计） | 主观"基本正确" ≥ 70% |
| 回归 | golden 伪代码快照 + 人工 review diff |

**“可读、不保证可编译”的操作性定义**（v1 验收口径）：

- 输出目标是让人回答：这个函数做什么、数据从哪来、调用谁、分支条件是什么。
- 允许存在：不完整/推断类型（`void *`、`*(int *)(a1 + 0x10)`、`field_10`）、推断的调用约定、
  不可约控制流降级为 `goto`、SSE/内联汇编/异常保留为 intrinsic 伪调用、`sub_xxxx` 命名。
- 不要求：能通过 C 编译器、外部符号全部有声明、结构体自动还原、可重编译。
- 硬性要求：每个 token 可回溯到指令地址（UI 双向高亮）；不能崩溃、不能静默丢指令。

可编译（类型系统完整 + 符号全解析）留到 L2/L3，作为可选目标。

### 7.5 语料与评测

- `tests/corpus/c/`：每个特性一个小函数（分支/循环/switch/结构体/函数指针/浮点/内联 asm），用 MSVC + MinGW 在 O0/O1/O2 × x64/x86 编译，PDB 作为函数边界 ground truth。
- 结构化成功率/表达式率在 CI 上跑，作为**质量回归门禁**。
- 真实开源程序（带 PDB）做抽样；恶意样本**不入库**，本地跑。
- 不引入外部反编译器作依赖；如需对照，仅本地手工比对。

---

## 8. UI（egui）

### 8.1 布局

```
┌ 菜单/命令面板 ─────────────────────────────────────────────┐
│ 函数列表 │ 反汇编（主）            │ 伪代码（同步高亮）    │
│ 字符串   │ 图视图（标签页切换）    │ 反编译选项面板        │
│ 导入导出 │ 十六进制（底部可选）    │ AI 助手               │
└──────────┴─────────────────────────┴───────────────────────┘
状态栏：地址/段/分析进度/后台任务
```

### 8.2 自绘网格（全项目 UI 最大技术风险）

一个虚拟化多列文本控件，承载反汇编/伪代码：

- 列：地址、字节、助记符、操作数、注释；每 token 可着色、可命中、可跳转
- 选区（跨行复制）、内联编辑（改名/注释）、IME、CJK 字体回退
- 键盘流：`Space` 切换视图（仅反汇编；数据行/函数外留在 Text）、`N` 改名、`;` 注释、`/` 伪代码注释（显示在行尾，`;` 在伪代码视图同样可用）、`X` xref、`G` 跳转、`Esc` 返回、`Ctrl+Z/Y`
- 性能：20 万行滚动 60 fps；单帧布局 < 4 ms

**门禁**：已于 2026-10-22 执行并通过（前置 spike `spike/egui-grid`，数据见 ADR-002）。
不达标时转 Tauri 的备选路径仍然保留。产品化时的注意事项：焦点必须使用
`response.id`（自造 id 会被 egui 清掉，Ctrl+C 失效）、无头测试需显式发
`Event::ModifiersChanged`。

### 8.3 图视图

- 分层布局（简化 Sugiyama）+ 回边拉直；节点：基本块，边：true/false/back
- 缩放/平移/缩略图；节点内文本按缩放级别简化
- 5 百–2 千节点规模可交互

### 8.4 i18n

- 实现：**Fluent**（`fluent` + `unic-langid`，Apache-2.0/MIT）；资源
  `crates/recraft-ui/locales/en-US.ftl`、`zh-CN.ftl` 通过 `include_str!` 编译进二进制
- 字符串外置：UI 构造器使用 `t("message.id")` / `t_args("message.id", ...)`，禁止新增
  `t(en, zh)` 内联对；英文默认，zh-CN 与 en 的 message id 必须一致（单测校验）
- 缺失消息回退：当前语言 → 另一语言 → key 本身；参数用 Fluent `{ $name }`
- 语言切换：顶部菜单 / 命令面板；选择写入 `layout.json`，重启后恢复
- 文档双语（README 英文 + 中文）；字体仍用系统 CJK 回退

---

## 9. MCP 与 AI

### 9.1 MCP server

- 传输：stdio 优先，HTTP 可选（带 token 鉴权）
- 读工具（`recraft-query` 提供，MCP 与内置 agent 共用同一份 spec 与执行器）：`list_functions`、`get_disassembly`、`get_pseudocode`（默认 L2）、`xrefs_to/from`、`list_strings`、`list_imports`、`list_exports`、`get_metadata`、`callgraph`、`search_bytes`、`function_summary`、`find_api_callers`
- 写工具（`serve-mcp --write` 显式开启，默认只读）：`rename`、`set_comment`、`set_prototype`、`define_function`、`define_struct`、`apply_type`、`batch`、`undo`
- 资源：`recraft://project/{id}/functions|strings|imports|pseudocode/{func}`
- 提示模板：`triage_sample`、`explain_function`、`propose_names`、`ctf_outline`

写操作协议：写工具直接经 Command 总线应用并写 journal（可 undo），进程级 `--write` 即人工授权；AI 的批量命名建议仍然走 `Proposal`（含证据地址 + 置信度）→ UI 展示 diff → 人工确认的两段式，二者共用同一条命令总线。

### 9.2 AI 功能（v1）

| 功能 | 描述 |
|---|---|
| 函数解释 | 选中函数 → 自然语言解释 + 关键 API/常量列表 |
| 命名建议 | 批量对 `sub_*` 提议名字（引用字符串/API/调用关系），diff 确认 |
| 样本摘要 | 一键 triage：能力、网络指标、可疑函数 top-N |
| 自然语言查询 | "找所有引用 URL 的函数" → 转 MCP 工具调用 |
| 伪代码润色 | L1 输出后处理（标注为 AI 修改，不覆盖原始输出） |

### 9.3 隐私与安全

- 默认 **不联网**、无遥测；AI 需显式配置 provider
- provider 可插拔：OpenAI 兼容 / 本地 Ollama；样本默认不出网
- 发送最小上下文（仅函数 + 相关名/字符串），支持字符串脱敏
- 审计日志（谁改了什么、AI 提案来源）、token 预算、速率限制、结果缓存
- 评测：用 PDB 符号做命名准确率回归；固定样本 + 固定 prompt 的回归集

---

## 10. CLI / headless

```
recraft-cli analyze <file> [--json] [--out project.rcraft]
recraft-cli strings|imports|funcs|callgraph <file> [--json]
recraft-cli decompile <file> --addr 0x401000 [--json]
recraft-cli search --bytes/--string ...
recraft-cli serve-mcp [project.rcraft] [--http]
recraft-cli batch <script.json>
```

CLI 与 UI 直接使用 `recraft-core` 的 `Session`，MCP 与内置 agent 共用 `recraft-query` 的只读
工具层；`recraft-app` 是规划中的编排层，目前仍是空壳（见 §1.1）。CI 用 CLI 跑质量门禁。

---

## 11. 测试与质量

| 层 | 手段 |
|---|---|
| PE 解析 | 单元测试 + `cargo-fuzz`（离线）；畸形样本库 |
| 解码 | 随机字节/已知指令集对照 iced 输出 |
| 分析 | 语料矩阵 + PDB ground truth（函数边界 precision/recall） |
| 反编译 | golden 快照 + 指标门禁（第 7.4 节） |
| DB/undo | 属性测试：随机命令序列 → undo 全部 → 状态与初始一致 |
| 大样本 | 合成 1 GB PE 生成器（CI 只跑较小规模，手动跑全量） |
| UI | 快照测试（egui 渲染结果）+ 手工清单 |
| MCP | 协议一致性测试 + 工具级集成测试 |

---

## 12. 许可与合规

- 代码：`AGPL-3.0-only`（每文件 SPDX 头）；不提供商业许可
- 贡献：inbound = outbound，`git commit -s` 签署 DCO；**不使用 CLA**，不做版权转让（ADR-013）
- `TRADEMARK.md`：名字/logo 不得被 fork 使用
- 依赖白名单：MIT / Apache-2.0 / BSD-2/3 / ISC / Zlib / MPL-2.0 / Unlicense；**禁止 GPL-2.0-only 与 GPL-3.0**（前者与 AGPL 不兼容，后者会堵死将来对自己代码的重新许可）
- CI：`cargo-deny`（licenses/bans/sources）；`REUSE` 合规检查；`NOTICE` 汇总
- 样本、签名、恶意代码一律不入库

关键依赖（许可已核）：egui/eframe（MIT/Apache-2.0）、iced-x86（MIT）、rusqlite（MIT, bundled SQLite public domain）、goblin/pelite（MIT）、rustc-demangle（MIT/Apache-2.0）、rayon（MIT/Apache-2.0）、serde（MIT/Apache-2.0）、clap（MIT/Apache-2.0）、tracing（MIT）、fluent（Apache-2.0）、rmcp（MIT，待核）。

---

## 13. 发布与分发

- 仓库：GitHub 公开；分支 `main` + PR 流程；`CONTRIBUTING.md`（DCO 签署说明）
- CI（GitHub Actions）：`windows-latest` 构建/测试/clippy/fmt/deny；后续 `ubuntu-latest`
- 产物：绿色版 zip（解压即用）+ WiX MSI 安装包（按机器、需管理员，可选文件关联 `.rcraft`）
- 版本：SemVer；`CHANGELOG.md`
- 更新：v1 手动下载（自动更新后置）；崩溃报告默认关闭（opt-in）
- 文档：README（截图/GIF）、快速上手、设计文档、安全说明（不执行样本）

---

## 14. 里程碑（C3：垂直切片 + 覆盖率阶梯）

按 3 天/周（≈13 工作日/月）估算，含 20% 缓冲：

| 阶段 | 周期 | 交付物 | 退出标准 |
|---|---|---|---|
| **M0 走通骨架** | 第 1–8 周 | 3 天 spike；workspace + CI；storage/schema；PE 加载；解码；Session/命令总线；线性视图；改名/注释/存盘；CLI 骨架；**LIR + L0 伪代码视图**（直线代码） | 能打开真实 PE、看到指令与 L0 伪代码、改名后重开还在；spike 5 条达标 |
| **M1 分析 + L1-alpha** | 第 9–20 周 | 递归下降/函数识别/跳表/字符串/导入导出/`.pdata`；xref；图视图；Hex；搜索；undo；CFG+SSA+表达式+if/loop 结构 | 验收剧本前三步可用；语料结构化成功率 ≥ 40% |
| **M2 语义 + 平台** | 第 21–32 周 | switch、变量合并、ABI（x64 MS / x86 cdecl、stdcall）、基础类型、Win32 x86、Go/Rust、API 分类面板、i18n、MCP 读工具 | 验收剧本四步全通；结构化成功率 ≥ 60% |
| **M3 质量 + AI** | 第 33–42 周 | 指标门禁、golden 回归、AI（解释/命名/摘要/查询）、写操作 propose-apply、1 GB 性能、打包 | 结构化成功率 ≥ 70%；1 GB 指标达标；内部 alpha |
| **M4 预览** | 第 43–52 周 | 覆盖率冲刺、安装包/绿色版、文档/演示、公开技术预览 | 可发布 v0.1-preview |

**C 选择带来的现实结论：公开预览约 10–12 个月，6 个月目标作废。**

### 风险与 kill criteria

| 风险 | 缓解 |
|---|---|
| 反编译范围爆炸 | M2 末门禁：结构化成功率 < 40% 或表达式率 < 70% → 降级为"反汇编 + AI 伪代码"为卖点，自研反编译转 beta |
| egui 网格不达标 | M0 spike 门禁；不达标转 Tauri（重开 ADR） |
| 1 GB 性能 | M1 起就用合成大样本跑基准；xref 有退路方案 |
| 单人节奏断裂 | 每里程碑可演示产出；M3 发内部 alpha 收反馈 |
| 分析质量被加壳样本击穿 | v1 明确"未加壳/可静态"范围；文档写清限制 |
| AGPL 劝退企业用户 | README 写清许可性质与理由（可审计性优先）；不另售商业许可，也不假装有 |

---

## 15. 评审结论（2026-10-22 已确认）

1. **接受 10–12 个月公开预览**（C 路线的必然代价）。
2. 反编译门槛按 **“可读、不保证可编译”** 定义（操作口径见 §7.4）。
3. **允许**在本地测试中用外部反编译器做人工对照：不随产品分发、不作为依赖、不进入 CI。
4. 仓库：GitHub `qux-bbb/ReCraft`，公开；本轮已执行 `git init`（初始分支 `main`）。
5. M0 开工顺序：**egui 3 天 spike + workspace 骨架**（spike 位于 `spike/egui-grid`，
   该目录被根 workspace `exclude`，验证完即删，结论写入 ADR-002）。

---

## 16. ADR 索引

| ADR | 决策 | 状态 |
|---|---|---|
| ADR-001 | headless core + 命令总线 + UI 客户端 | 已定 |
| ADR-002 | egui 原生 UI（spike 通过：20 万行网格 p95 0.62 ms、500 节点图 p95 2.43 ms） | 已定 |
| ADR-003 | 单文件 SQLite 容器 + 分块镜像 + 分页 flags；xref 表方案留退路 | 已定 |
| ADR-004 | 用户数据（journal）与派生数据（可重建）分离 | 已定 |
| ADR-005 | 自研 L1 反编译器（LIR→MIR→HIR），不集成外部实现 | 已定 |
| ADR-006 | 不兼容 IDAPython；自有 API（CLI/MCP 先行） | 已定 |
| ADR-007 | AGPL-3.0-only + 商业双许可 + CLA | 已被 ADR-013 取代 |
| ADR-008 | MCP 原生；写操作 propose/apply，人工确认 | 已定 |
| ADR-009 | Windows 首发，core 跨平台，Ubuntu 后置 | 已定 |
| ADR-010 | 依赖白名单（cargo-deny），禁 GPL 系 | 已定 |
| ADR-011 | 大样本目标 1 GB，交互优先、后台导入/分析 | 已定 |
| ADR-012 | 多窗口 | 已定 |
| ADR-013 | 纯 AGPL-3.0-only，DCO 取代 CLA，取消商业许可 | 已定 |
| ADR-014 | MCP 写工具由 `--write` 显式开启，走 Command 总线可 undo（部分取代 ADR-008） | 已定 |
