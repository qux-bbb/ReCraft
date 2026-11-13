# 变更日志

本项目所有值得注意的变更都会记录在此文件中。

本格式基于 [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)，
并且本项目遵循 [Semantic Versioning](https://semver.org/spec/v2.0.0.html)。

0.1.0 是首个公开技术预览。下面按使用者能感知的能力组织；从 M0 骨架到本版本的**完整**
开发记录（每条修复、每次指标变化、每个负结果）归档在
[docs/CHANGELOG-0.1.0.md](docs/CHANGELOG-0.1.0.md)。

## [未发布]

### 新增

- **内置 CRT 签名从 9 条扩到 43 条，`.map` 逐地址校验零误报。** 覆盖 MSVC 静态 CRT 的启动
  （`mainCRTStartup`、`__scrt_common_main_seh`）、异常与 RTC（`__C_specific_handler`、
  `__CxxFrameHandler3`、`_CxxThrowException`、`_RTC_Initialize`）、内存与字符串
  （`malloc`/`free`/`calloc`、`memcpy`/`memset`、`strchr`/`strrchr`/`strcpy_s`/`strncpy_s`）、
  文件与进程（`fclose`、`exit`/`abort`、`atexit`/`_onexit`、`qsort`）等。签名由项目钉住的
  MSVC crackme 构建的 `crackme.map` 生成（`dump_builtin_signatures` 可重生成，`_RTC_Terminate`
  等与同族函数头部完全相同或只是 `call`+填充的条目被有意排除）。crackme 的自动命名从 10 个
  函数升到 43 个，`main` 的 `sub_` 引用 17 → 2；`notepad.exe`（同族 CRT 的系统二进制）命中
  `atexit`/`__security_init_cookie`/`__report_gsfailure`/`_RTC_Initialize`/`__GSHandlerCheck`
  共 6 个，启动识别另给出 `WinMain`，说明匹配不是撞字节。新增两个单测：用 `.map` 逐地址断言
  “签名只命名该命名的函数”（杜绝误报），以及拒绝重复 pattern（否则名字取决于 JSON 顺序）。

- **字符串视图可以编辑。** 字符串是从镜像重建的派生数据，遇到链接器把数据表紧贴在字面量之前时
  （crackme 的 `enc[31]` 是 `'O'`，于是 42 个 `=` 被列成 `O====…`）用户没有办法修正。现在字符串
  右键菜单提供「编辑字符串」「删除该字符串」「恢复默认」：编辑替换列表与读工具里的显示文本，
  删除把该行移出列表，恢复清掉两者。覆盖写进新的 `string_override` 表（schema v5），属于
  **用户数据**，重新分析与重建字符串后依然保留；它走命令总线，所以 `Ctrl+Z`/`Ctrl+Y` 可撤销重做，
  MCP 的 `list_strings` 与 CLI 的 `strings` 看到的也是修正后的文本。反编译器仍按真实字节工作
  （字面量的含义不随注记改变），789 张 golden 不受影响。

- **MCP 的 HTTP 传输改为有状态会话，一次配置即可打开任意程序。** `serve-mcp --http` 现在
  可以不带项目启动：`initialize` 创建一个 `Mcp-Session-Id` 会话，会话内维护自己的实例注册表，
  `open_program`/`use_instance`/`close_instance` 在 HTTP 下也可用，`DELETE /mcp` 关闭会话；
  不带会话头的简单客户端共用一个 default 会话。会话 30 分钟空闲回收，最多 16 个，同一会话的
  调用串行、不同会话与 `/health` 不受影响。OpenCode 等远程客户端因此不再需要为每个样本重启
  一个服务；stdio 的空注册表形态保持不变。

- **MCP 写工具支持乐观并发检查。** `rename`/`set_comment`/`set_prototype`/`apply_type`/
  `define_function` 新增可选 `expected_previous`：字符串要求当前值完全相等，`null` 要求当前
  没有值，省略则跳过。比较和写入在同一个 immediate 事务里完成，后到的客户端得到
  `stale write: … expected previous …, found …; re-read and retry`，不会静默覆盖别人的编辑。

- **GUI 自动刷新外部改动。** GUI 轮询当前项目的 `PRAGMA data_version`（只对**其它连接**的提交
  变化），MCP/CLI/第二个 GUI 写入后把函数、字符串、导入、反汇编、伪代码与 CFG 标脏并在下一帧
  重载，不再需要关闭重开项目；GUI 自己的写入不会触发刷新。`Session::data_version` 与
  `Session::execute_guarded` 是这套机制在核心层的入口。

- **OpenCode 文档以“一次配置、运行时打开”为主路径。** `docs/MCP.md` 的启动与客户端章节改为
  先给空注册表的 `serve-mcp` 配置（`--write` 单独说明），固定项目作为可选形态；HTTP 章节补
  会话头、`DELETE`、会话隔离与回收，写工具章节补 `expected_previous`。

- **反编译视图可直接加注释。** 伪代码视图支持伪代码注释：光标所在行按 `/`（`;` 也接受，沿用
  全局注释键）弹出“伪代码注释”对话框，支持多行（`Ctrl+Enter` 或 OK 提交），空文本删除。注释以
  `code  // …` 追加在被注释行**末尾**（与反汇编的注释列同一形态），多行注释的后续行紧跟其下、
  与 `//` 对齐；单击该行仍同步反汇编，双击不会把注释里的函数名或字符串当成跳转目标。注释按地址
  存进 `comment` 表的新槽位 `pseudo`（`kind = 3`），与反汇编注释分开保存，像 IDA 一样两套注释
  互不串台；随项目持久化、可 undo/redo，重新反编译后按地址回到原行，地址已不在输出里的注释
  作为“孤立注释”以 `//` 行排在末尾（IDA 的做法）。`comment`/`uncomment` 命令新增
  `--kind pseudo`，批次脚本与 MCP `set_comment` 的 `kind` 也接受 `pseudo`；反编译文本、lint 与
  golden 快照不受影响（注释只属于视图层）。

- **没有精确函数表时的序言回退。** PE32 没有 `.pdata`，此前只靠导出、入口、调用目标和数据
  指针播种，于是 DecBench 里 5 个 PE32 目标（`bin_150/158/198/213/222.exe`）漏掉了那些不被
  直接调用、也没有任何指针指向的函数（`0x4033b0` 是作为立即数传给线程创建 API 的线程例程，
  `0x4013d1` 是链接器留下的无引用代码）。`recraft-analysis` 现在在递归下降收敛后扫描仍未
  覆盖的可执行区间，只在「`ret`/`jmp` 或对齐填充之后」接受 `push` 被调用者保存寄存器、
  无帧 `sub esp, imm`、`lea esp, [esp-…]`、热补丁 `mov edi, edi` 这类序言，并用一次试探性
  CFG 遍历确认它确实解出一段指令（到达 `ret`，或长到不像数据巧合）；多字节 NOP
  （`lea esi, [esi]`、`xchg ax, ax`）按填充处理。回退只在镜像没有精确函数表时启用（PE32、
  剥离符号的 ELF），`.pdata`/`.pclntab`/`.symtab` 存在时不跑，因此 notepad/crackme/corpus/
  corpus86/go 的 789 张 golden 零 diff、lint 0 findings。`analyze` 新增
  `from prologues: N` 统计。DecBench sample-set 的 PE32 产出从 3/8 升到 **8/8**，合计
  **166/166**（不做 ARM32 时的上限）。

- **Rust 语料样本与门禁。** 新增 `corpus/rust/lib.rs`（`cdylib`）与 `corpus/build-rust.ps1`；
  `corpus/build-linux.sh` 在装了 rustc 时另出 `libcorpus-rust.so`。样本补齐只有 Rust 二进制
  才有的形制：panic/unwind 机制、trait object 的 vtable 动态派发、捕获环境闭包、`format!`
  背后的 `core::fmt` 单态化、同一泛型的两个实例。PE 版靠 `#[no_mangle]` 导出表命名
  （346 个函数、结构化 86.1%、覆盖率 97.4%、lint 0 findings）；ELF 版保留 `.symtab`
  （618 个函数、结构化 84.0%、覆盖率 98.0%），`corpus_rust::…` 这类 legacy 名称走
  demangle（哈希后缀被去掉），由
  `the_rust_elf_corpus_demangles_legacy_names` 与 Linux CI 的名称断言守着。Windows/Linux
  两个 CI 任务各加一段语料门禁（metrics 下限 + lint）；两版都**不提交 golden 快照**——
  rustc 版本在 CI 镜像上会漂，字节级快照会变成噪声。

- **ELF32（i386）支持。** `recraft-format` 的 ELF 加载器现在同时接受 `ELFCLASS32` 与
  `ELFCLASS64`：32 位的 ELF 头/程序头/节头/符号表/动态表按各自的字段偏移与宽度读取，导入
  同时识别 x86-64 的 `SHT_RELA`（`.rela.plt`/`.rela.dyn`）与 i386 的 `SHT_REL`
  （`.rel.plt`/`.rel.dyn`，`r_info` 低 8 位是类型、高 24 位是符号），重定位类型沿用两者
  同号的 `JUMP_SLOT`/`GLOB_DAT`/绝对字。`.eh_frame` 的 `DW_EH_PE_absptr` 与 DWARF 行表按
  4 字节地址解析；`meta.format` 记为 `elf32`，`info` 显示 `ELF32 (x86)`。反编译复用既有
  x86 路径（栈参数 `[esp+N]`、`__cdecl`/`__stdcall`、32 位常量折叠），新增
  `CallAbi::SysV32`（无寄存器实参）与 i386 PLT 桩识别（PIC 的 `jmp [ebx+disp]`、非 PIE 的
  绝对 `jmp [disp]`，`ebx` 取 `.got.plt`/`.got`），`call puts@plt` 现在渲染成 `puts(...)`。
  语料：`corpus/build-linux.sh` 在有 32 位 libc 时增编 `libcorpus32.so`（含剥离版），
  `tests/golden/elf32/` 55 张快照；Linux CI 增加 ELF32 门禁。真实样例（gcc 13 `-m32` 的
  PIE / 非 PIE / 共享库 / 剥离二进制、Go 386 静态二进制）实测 lint 全绿、结构化 98%。
  顺手修掉三处缺陷：符号范围探到未映射字节时的越界读、`.plt.got` 桩区间越过节尾并吞掉
  下一节开头、`__x86.get_pc_thunk.bx` 这类带点符号的后缀被当成寄存器改名。

- **真实 crackme 夹具现在也能编成 ELF。** `corpus/real/crackme/build-linux.sh` 用宿主 gcc
  `-O2` 把同一份 `src/crackme.c` 编成动态 PIE（不 strip，`.symtab` 仍命名 `main`；`-O2` 下
  GCC 会把 `verify` 内联进 `main`），`--snapshots` 可生成仅在本机使用的
  `tests/golden/crackme-elf/`。`cargo test -p recraft-decompile --lib real_crackme` 新增两个
  ELF 用例：全量 lint（L1+L2）、结构化率 >80% 与覆盖率 >90% 的下限、存在本地快照时做对比；
  Linux CI 任务会先构建再跑。构建产物缺失时测试打印 `skipping:` 提示后跳过
  （`-- --nocapture` 可见），PE 侧原有的静默跳过也一并补上了提示。

- **ELF 的 PLT 调用按导入名渲染，并按 SysV 恢复实参。** 新增 `recraft-core::plt`：扫描
  `.plt`/`.plt.got`/`.plt.sec` 里的 `jmp [rip+got]` 桩，得到「桩地址 → 导入函数」表。
  反编译时直接调用桩的 `call` 现在打印成 `puts("…")`、`fgets(&var_0, 0x100, stdin)`、
  `__printf_chk(2, "Enter flag: ")` 这样的形式，而不是 `sub_10b0()`；x64 的实参寄存器
  按格式切换（ELF 用 SysV 的 `rdi/rsi/rdx/rcx/r8/r9`，PE 仍是 `rcx/rdx/r8/r9`），
  调用前的寄存器准备行也随之并入实参。`.plt.sec` 的每个桩各自成为 16 字节函数并命名为
  `<导入名>_plt`（如 `puts_plt`），不再把整段 `.plt.sec` 并成一个函数。PE 的
  `call [IAT]` 路径不变，789 张 PE 快照零 diff。已知限制：`.rela.plt` 指向本文件
  已定义符号的 canonical PLT（`-fPIC` C++ 常见）还没有按本地符号命名，仍显示为
  `sub_<addr>`；这类桩本身已拆成 16 字节函数。

- **无帧指针的 ELF 栈帧可以还原了。** GCC/Clang 的 x64 默认省略帧指针（`rbp` 被当通用
  寄存器用，如循环计数器），此前 L2 会把 `buf`/`buf[i]` 这类 `rsp` 相对访问留成裸 `rsp`，
  变量重建再把它变成未赋值的 `vN`。现在把 `mov reg, rsp` 的别名寄存器也当作帧基址
  （优先 `rbp`，没有则取第一个非 `rsp` 别名），`rsp + index` 的无位移寻址渲染成
  `*(u8*)(&var_0 + i)`，局部数组按栈槽命名；`rsp` 与只在入口读取的寄存器保持寄存器名，
  不再凭空生成本地变量。Linux 版 crackme 现在读作 `fgets(&var_0, 0x100, stdin)`、
  `strpbrk(&var_0, "\r\n")`、`strlen(&var_0)`。

- **低地址字符串与 `.rodata` 短字面量。** 字面量识别不再假设 PE 的 `> 0x10000` 绝对地址：
  地址常量按镜像基址 + 镜像大小过滤，ELF（基址 0）的 `0x2008` 这类字符串也能映射；
  不在字符串表里的短字面量从 PE `.rdata` / ELF `.rodata` 直接读取，因此 `"\r\n"`、
  `"C_CR4CK_ME_2024!"` 都能显示。

- **MCP 多实例：不再绑定单个项目。** `serve-mcp` 的项目参数现在可选（省略即空注册表，
  由客户端调用 `open_program`），并新增一组注册表工具：`open_program`（导入 PE/ELF 二进制
  或打开已有 `.rcraft`，默认把数据库放到用户目录 `%LOCALAPPDATA%\ReCraft\projects`，按二进制
  路径哈希命名，重复打开复用原项目并自动分析）、`list_instances`（id/路径/格式/架构/入口/
  函数与字符串数/是否活动）、`use_instance`、`save_project`（WAL checkpoint + `VACUUM INTO`
  一致副本，默认目标为用户目录，拒绝覆盖已有文件）与 `close_instance`。其余工具都接受可选
  `instance` 参数，指定实例即切换活动实例；资源 URI 带实例 id，`resources/list` 列出全部实例。
  多实例只在 stdio 成立（HTTP 无状态，每请求一个 Session，注册表工具会明确拒绝）。
  `get_metadata` 现在按项目的真实格式/架构回报（ELF 不再误报 `pe32`/`x86-64`）。

- **安装包可用性。** 安装向导改为中文：WixUI 自带 zh-cn，自有字符串在
  `packaging/wix/zh-CN.wxl`；`scripts/package.ps1` 新增 `-Culture`（默认 zh-CN，
  `en-US` 用 `packaging/wix/en-US.wxl`），MSI 的 `Language`/`Codepage` 跟着切到 2052/936
  或 1033/1252。目录页之后新增“安装选项”页（PATH 默认勾选、桌面快捷方式默认不勾）：把
  `recraft-cli` 加入系统 PATH、创建桌面快捷方式；静默安装用 `ADDTOPATH=0` /
  `DESKTOPSHORTCUT=1`。新增开始菜单“ReCraft 命令行”（在安装目录
  打开 cmd），`recraft-cli.exe` 补上 App Paths（`Win+R` 可直接启动）；新增 Capabilities +
  RegisteredApplications，ReCraft 出现在“设置 → 默认应用”。安装文档补充“卸载不删除
  `%LOCALAPPDATA%\ReCraft`”的说明；`scripts/verify-msi.ps1` 的结构断言扩到 21 项，完整流程
  增加 PATH/桌面快捷方式/App Paths/Capabilities/卸载清理检查。

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

- **Linux API 行为分类补齐。** 分类表新增裸描述符调用：`open`/`openat`/`read`/`write`/
  `pread`/`close`/`stat`/`ftruncate` 等按**整名精确匹配**（子串匹配会把 `OpenProcess`、
  `ReadFile` 误判成文件操作），并忽略 `@GLIBC_2.2.5` 这类版本后缀；另补 `dlsym`/`dlclose`、
  `mprotect`、`brk`/`sbrk`、`clone`。`/proc` 读取单独做路径分类：`/proc/self/*` →
  anti-analysis、`/proc/<pid>/{maps,mem,status,fd,…}` → process、`/proc/net/*` → network、
  其余 `/proc/*` → file。`ai triage` 新增 `paths_by_category` 和每个可疑函数的 `paths`，
  `/proc` 字面量会按数据引用记到具体函数上并参与可疑度加权。新增
  `corpus/linux/behaviors.c`（open/write、socket/connect、ptrace、dlopen、三种 `/proc`
  读取）作为回归样本，由 `build-linux.sh` 构建，`recraft-ai` 的 ELF triage 测试用它验证
  API 与路径标签都非空。

- **DWARF 行号表。** 新增 `recraft-format/src/dwarf.rs`：解析 `.debug_line`（DWARF 2–5，含
  v5 的 directory/file 表与 `.debug_line_str` 间接字符串），得到按地址排序的 `file:line`
  行。`Image` 增加 `debug_lines`（serde default，旧项目照常打开），ELF 加载器自动读取；
  `recraft-cli lines <project> [--addr] [--json]` 打印映射。ELF 语料新增 `-g` 构建
  （`libcorpus-debug.so`），CI 的 Linux 作业验证行表能指回 `functions.c`。
  类型信息（`.debug_info`）未做——行表只给位置，不给类型。
- **DecBench 对照可用。** 新增 `recraft-cli decbench <kit> -o <dir>`：读 DecBench 评测套件的
  `functions.json`，逐二进制 import/analyze 后按目标地址渲染 L2 伪 C，写成套件要求的
  `results/<bin>.c` + `results.json`。sample-set（223 个二进制、250 个函数）实测 161 个
  二进制/函数产出（11-09 首跑为 147，ELF64 现为 158/158 全覆盖）、`package.py` 校验通过，
  耗时约 168 秒（debug 构建）；指标含义、实测数字与差距见 `docs/DECBENCH.md`。
- **Itanium C++ 名字可读了。** `recraft-common::demangle` 在 MSVC 与 Rust 之后新增 Itanium
  ABI 分支（`cpp_demangle`，MIT/Apache）：ELF 上的 `_Z…`、Mach-O 的 `__Z…` 现在显示为
  `recraft::square(int)` 这样的名字，并随符号名进入名字表。配套加了
  `corpus/linux/functions.cpp`（命名空间、类方法、重载、模板）与 CI 的 C++ 语料门禁
  （`tests/golden/elf-cpp`）。

- **字符串视图向 IDA 对齐，最短长度可配置。** `隐藏头部/节名` 收窄为 `隐藏头部字符串`：只隐藏
  `HEADER` 段里的 DOS stub 与节表字节（IDA 不把 PE 头部装进任何段，`is_loaded(IMAGE_BASE)
  == 0`，这类字符串在 IDA 的列表里根本不存在），`.rdata` 里的 `.text$mn`/`.CRT$XCA`/`.xdata`
  等节名不再隐藏——IDA 8.3 的列表照列它们，而点前缀通配还会误伤 `.tar`/`.exe`/`.gitignore`/
  `../action.c` 与 ELF 正则里的 `.^$?*+|[({`（实测 134 462 条字符串里点前缀规则隐藏 384 条，
  其中 272 条被代码引用）。`Strings → 设置…`（对应 IDA 的 *Setup…*）新增 `最短长度` 与 `重建`：
  长度按字符计、不含终止符，存进项目 `meta`（`strings.min_len`），重建在后台线程重新抽取并写回，
  列表与伪代码里的字面量一起刷新；`analyze --min-string-len N` 使用同一设置。

### 修复

- **ELF（gcc）反编译的两个编译器簿记问题。** 一是字符串字面量：GCC 把一个地址存在寄存器里复用，
  L2 曾输出 `v5 = "===="; v6 = v5; puts(v6);`，现在只被赋值为字面量的局部量会在使用点展开并删掉
  声明（`strpbrk(&var_0, "\r\n")`、`__printf_chk(2, "Enter flag: ")`）；`&vN`、二次赋值或 `vN++`
  时仍保留变量。二是栈保护：`mov rax, fs:[0x28]` / `var_108 = rax` 与每个出口的三行检查属于编译器
  簿记，当该槽只被这套 idiom 使用时整组隐藏，`return 0;` 不再被遮住、也不再出现 `return eax;`。
  crackme.elf 的 `main` 因此从“字面量装进变量 + `return eax`”变成可读的调用点；剩余 5 处 `goto`
  是 `docs/NEXT.md` §1 的多入口循环缺口（W3），已有回归测试钉住这两项。

- **ELF 调用第 5、6 个寄存器实参不再让整次导出 panic。** SysV 的整数实参寄存器有六个
  （`rdi/rsi/rdx/rcx/r8/r9`），但记录「实参寄存器最近写入值」的 `arg_values` 只有四个槽
  （`arg_slot()` 也只认前四个）。直接调用带五个以上实参、且第五个正好是 `r8` 时，
  `call_argument()` 用 `slot` 索引该数组越界 panic，`recraft-cli decbench` 因此在中途
  （写出 64 个 `.c` 后）中止。读路径改为 `.get(slot)`，缺失时退回按表达式渲染寄存器名，
  与未被跟踪的槽行为一致。顺带把「不支持的 ELF 机器」报错写清楚：`EM_ARM`（0x28）现在
  明确提示需要 ARM32 lifter，不再误写成「arm64 with the ARM64 lifter」。

- **GUI 的「导入」也能选 ELF 了。** 文件对话框此前只挂 PE 扩展名（`exe`/`dll`/`sys`），而 ELF
  样本多半没有扩展名（`a.out`、`busybox`），在过滤后的对话框里根本不显示，看起来就像「只能导入
  PE」；导入本身与格式无关（`Session::create` 按魔术字分派，拖放和 CLI 早就能导入 ELF）。现在
  对话框列出 `elf`/`so`/`bin`/`out` 并增加「所有文件」兜底（rfd 只在没有任何过滤项时才自动加
  `*.*`），菜单和命令面板的文案由「导入 PE…」改为「导入可执行文件…」，拖放遮罩与空视图提示同步
  改成 PE/ELF。新增两个 UI 测试：无扩展名的 ELF 导入后 `format == Elf` 且自动开始分析；导入
  过滤表里同时保留 PE 与 ELF 扩展名。

- **反汇编列标题与各列对齐，字形也不再被顶部裁掉。** 列标题改成常驻后还留着两处毛病。一是整行
  标题用 `{:<14}` 按字符补空格，而中文列名比一个等宽字符宽，`助记符`/`操作数` 就比它们下面的
  列整体右移；现在每个列名按列起点（第 0/14/60/69/105 个等宽格，与行里的 `{:width$}` 字段一致）
  单独绘制。二是标题按 15 px 行高居中、又裁剪在那条 15 px 的带子里，中文字形框比带子高，顶部
  被削掉约 1 px；现在带子高度按实际行高撑开（`max(row_height, 行高 + 2)`），且只做水平裁剪，
  字形上下都完整，标题与第一行之间也不会叠字。

- **反汇编视图的列标题不再随滚动消失。** 列标题（地址/字节/助记符/操作数/注释）原来画在滚动区
  内容里，是列表的第 0 行，滑轮一滚就跟着指令一起滚出顶部——只有回到最顶才能再看到。现在它和
  字符串窗口、交叉引用对话框一样固定在列表上方：竖向滚动不动它，横向滚动时按偏移跟随内容。

- **字符串视图里带换行的字面量不再挤坏相邻行。** PE 的 DOS stub（
  `This program cannot be run in DOS mode.\r\r\n$`）这类含控制字符的字面量此前被 egui 按 `\n`
  折成两行、以行中心垂直居中，于是第一行压到上面的列标题、第二行压到下一行，两行又都被行矩形
  裁掉，看上去就是「字符串只显示了一半」。字符串窗口现在按 IDA 的一行一条渲染：控制字符转义成
  可见文本（`\n`/`\r`/`\t`/`\xNN`），行内不再出现换行，Ctrl+C 复制的整行也保持单行。
  同时按 IDA 的做法把「字符串」列宽按最宽的一条字面量撑开，内容超出窗口时整行（含列标题）
  可以横向滚动，长字符串不会再被静默截断在窗格右边缘。

- **输出窗口的日志行不再相贴。** 输出窗口原来固定 11 px 等宽字、行距只有 3 px，一行字的下缘
  正好压在下一行的上缘上；中文行最上面一两行笔画本来就淡，贴着上一行看过去就像「顶部被挡住」。
  现在输出窗口用与列表一致的等宽字号，逐行按 galley 尺寸排布并留出真实行距，列表顶部也加了
  内边距：行与行、首行与标签栏都分得开。（顺带核查过：等宽字族的中文回退字形本身完整，用平顶
  字形「口」实测顶横都在，`MONO_CJK_Y_OFFSET_FACTOR` 的基线补正不需要改。）

- **输出窗口的日志可以选中复制了，行为与 IDA 的 Output 窗口一致。** 上面那次「逐行按 galley
  排布」把日志行从可选择的标签换成了直接绘制的 galley，鼠标能滚不能选，Ctrl+C 也没有内容可拷。
  现在输出窗口有一套与反汇编/反编译视图相同的手动文本选择：按下锚定、拖动跨行选中（拖出窗口上下
  边缘会继续选到首/末行）、双击选词、Shift+点击扩选；**Ctrl+A 选中整个输出窗口**（不是光标所在
  的一行），Ctrl+C 按行复制选中文本并像其他窗格一样在状态栏报告行数；右键菜单提供「复制」与
  「全选」。选中高亮按 galley 的行分段绘制，长日志行折行后每一段都正确高亮；中文与 `error:`
  红字配色、行距、自动滚到底部都不变。日志超过 500 行触发裁剪时选择会被清空，避免高亮错位。
  复制后的状态提示按实际选择报告：整行选择是「已复制 N 行」，只选了一行的一部分（或双击选中的
  一个词）则报「已复制 M 个字符」，不会再把片段说成「已复制 1 行」。

- **宽字符串字面量不再丢掉首字符。** 反编译器的字面量兜底只认窄串：不在字符串表里的短宽字面量
  会被读成 `"S"`、`"%"`、`"E"`（notepad 的 `RegSetKeyValueW(..., "Software\\Microsoft\\Notepad", …)`
  此前显示成 `"S"`）。`read_c_literal` 现在先识别 UTF-16LE（高字节为 NUL、双字节 NUL 结尾），
  再退回窄串；notepad 的 17 份 golden 快照随之更新，恢复出 `Software\\Microsoft\\Notepad`、
  `Windows.Foundation.Uri`、`%d`、`"\r\n"`、`"*.*"` 等完整字面量。

- **ELF 的 API 标签现在落在调用点上。** 普通 ELF 通过 PLT 桩（`jmp [rip+got]`）调用导入
  函数，而桩本身不在函数列表里，所以此前只有少数 `-fno-plt` 风格的直接 GOT 引用能产生
  标签，真实 ELF 样本的 triage 几乎是空的。`analyze` 现在扫描可执行段里的 `ff 25` 桩
  （含 CET 的 `endbr64`/`bnd` 前缀），把 `call <桩>` 解析成导入名并在调用点打标签：样本上的
  API 标签从个位数升到数千，`suspicious` 能正确归到 `main` 等具体函数；PE 的 IAT 路径不受
  影响，L2 输出不变。
- **`Reg` 前缀不再把无关符号判成注册表。** 旧规则是 `contains("REG")`，ELF 的
  `_ITM_deregisterTMCloneTable`、`__register_frame_info` 都被标成 registry，`RegisterClassW`
  也永远到不了 ui 规则；现在按 `RegOpen`/`RegSet`/`RegQuery`/`RegClose` 这类动词匹配，
  ELF 样本上的假 registry 标签归零，真正的 `Reg*` API 不受影响。
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

- **Strings 视图改用 IDA 同款的 `Shift+F12`。** IDA 的 Strings 窗口就是 `Shift+F12`，ReCraft
  此前把 Strings 绑在 `F4`。现在 `Shift+F12` 打开/聚焦 Strings（页签关掉时重新停靠），Views
  菜单的工具提示同步为该组合；`F4` 不再响应（IDA 的 F4 属于调试器，ReCraft 没有对应命令）。
  新增无头测试：关掉 Strings 页签后按 `Shift+F12` 能重新停靠并置于前台。

- **最短字符串长度默认 4 → 5。** 与 IDA 8.3 的 *Minimal string length* 默认值一致（实测：把
  `strwinsetup_t.minlen` 设成 4 会多出 `.bss` 这类 4 字符串，设成 8 只剩 28 条；IDA 的 Length
  列含终止符，但最小长度不含）。该值现在是项目设置 `strings.min_len`，`analyze` 时读取、GUI
  的 `设置… → 重建` 时写回；代码里 `4` 的硬编码全部换成该设置或
  `DEFAULT_MIN_STRING_LEN`。notepad 的 golden 快照因此更新 17 份（同时包含上一条宽字面量修复），
  crackme/corpus/corpus86/go 与 `lint`、`metrics` 不变（notepad 仍是 494/506 结构化、覆盖 99.6%）。

- **`recraft-ai` 依赖 `recraft-analysis`。** triage 要与 `analyze` 共用同一份 API 与
  `/proc` 分类规则，不再复制一份；依赖方向仍单向（`recraft-analysis` 不反向依赖
  `recraft-ai`），构建依赖图无环。
- **加载层不再假设「PE 即全部格式」。** `recraft-format` 现在有格式中立的 `Image`、`Format`
  与 `Arch`，`Loader` trait 返回带格式与架构的 `LoadedImage`，`load()` 按魔术字分派（PE 是
  唯一实现，其他格式报 `unrecognised executable format`）。项目把 `format` 与新增的 `arch`
  写进 `meta`，镜像 JSON 的键从 `pe_meta` 改为 `image_meta`，旧项目仍从 `pe_meta` 读取，
  不需要重新导入。这是一次零输出变化的重构：789 个 golden 快照、`lint` 与反编译文本不变。
- **CI 增加 Linux 作业。** `core crates (Linux)` 在 `ubuntu-24.04` 上跑 11 个可移植 crate 的
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
- **CI 动作升到 Node 24，Linux 作业固定镜像。** `actions/checkout` v4 → v7、
  `actions/upload-artifact` v4 → v7，消掉 GitHub Actions 报的 Node.js 20 弃用警告（这些动作从
  v5 起声明 `node24`，要求 runner ≥ 2.327.1，GitHub 托管 runner 已满足；v6/v7 的其余改动——
  凭据写独立文件、`pull_request_target` 禁止检出 fork——与本仓库无关）。`core` 与 `licenses`
  两个作业改用 `ubuntu-24.04`：`ubuntu-latest` 从 2026-10-19 起迁往 Ubuntu 26，而提交的
  `tests/golden/elf{,-cpp}` 依赖镜像里的 gcc/glibc，静默换镜像会让快照抖动；升级镜像要在
  同一个 PR 里用 `RECRAFT_REGEN_ELF_SNAPSHOTS=1` 重建快照。

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
