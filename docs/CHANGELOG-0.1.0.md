# ReCraft 0.1.0 开发明细（存档）

本文件是 `CHANGELOG.md` 中 0.1.0 条目的完整开发记录：从 M0 骨架到公开预览的全部增量、
修复与移除，按提交顺序排列。面向用户的摘要见 [`CHANGELOG.md`](../CHANGELOG.md)。

保留原文，仅去掉顶层标题层级（原来的 `## [未发布]` 与重复的空行）。

---
### 新增

- **CTF crackme 现在可在仓库内复现。** `corpus/real/crackme/` 在测试夹具的 `src/crackme.c`
  （源码 SHA-256 已固定）旁镜像了一份 `build.ps1`，后者用 MSVC（`/O2 /MT /GS- /TC /Brepro`）
  逐字节重建 146,432 字节的用例二进制，为 `eval --map` 生成 `crackme.map`，
  并在较新的工具链偏离固定 SHA-256 时发出警告。只提交源码和脚本 —— exe、map 和
  `.rcraft` 仍是构建产物。当重建结果一致时，快照继续共享 `tests/golden/crackme`（160，
  `--limit 160`）；一旦出现分歧，`build.ps1 -Snapshots` 会填充 `tests/golden/crackme-real`，
  此后门禁优先使用该目录。新增的 `cargo test -p recraft-decompile --lib real_crackme`
  运行 `lint`（L1+L2，前 160 个）和 golden 快照检查，在干净的检出（没有二进制）上跳过，
  并且不进入 CI，因为 windows-latest 的 MSVC 版本会让提交的快照变得不稳定。

- **渲染后的伪代码会缓存在项目中。** `get_pseudocode` 把它的 L2 文本存进 `pseudocode` 表
  （此前一直是未使用的表结构），键是可能改变它的一切的指纹 —— 函数范围、名称、原型、参数类型、
  结构体布局数量以及反编译器版本（`recraft-decompile::VERSION`）。对 shell32 上那个
  14 768 行的函数重复读取从 **9.8 s 降到 16 ms**；结果带有 `cached: true`。缓存是派生数据：
  每条命令、撤销、重做和重新分析都会删除它，写入失败（例如只读文件）只意味着“再次反编译”。

- **函数列表只需一次 JOIN，摘要只读取函数自身的范围。** `Session::functions_named` 取代了
  每行一次 `name_at` 查询（5 000 次查找：17.9 ms → JOIN 在 13.5 ms 内返回全部 31 803 行；
  `list_functions(5000)` 从 53 ms 降到 21 ms）。`function_summary` 改用新的
  `xrefs_from_range`，不再每次调用都加载两张完整 xref 表 —— 在 shell32 上 131 ms → 34 ms ——
  而 `callgraph` 不再为每条边重新扫描函数表。

- **`recraft-mcp` 拆分为多个模块。** 2 276 行的 `lib.rs` 变成了 `lib.rs`（协议 + 服务器 +
  测试）、`http.rs`（传输）、`schema.rs`（工具规格）、`write.rs`（日志化编辑）和
  `resources.rs`（资源 + 提示）；`lib.rs` 降到 1 292 行，其中约 1 000 行是测试。

- **导出、映像元数据和提示模板。** 每个 DLL 和每个分诊问题都需要的两个读工具此前缺失：
  `list_exports`（名称/序号/VA/转发器，过滤 + 分页）和 `get_metadata`（格式、架构、基址、
  入口、大小、子系统、时间戳、TLS 回调、带权限的节、函数/导入/导出/字符串计数、解析器警告）。
  MCP 服务器现在还公开 `prompts`：`triage_sample`、`explain_function`、`propose_names` 和
  `ctf_outline`，每个都附带一行映像摘要渲染，让模型从事实出发（`prompts/list`、`prompts/get`）。
  这两个工具通过 `recraft-query` 与应用内 agent 共享。

- **客户端可以据以行动的工具有元数据。** 每个 MCP 工具现在都带有 `title`、`annotations` 和
  `outputSchema`，成功的 `tools/call` 结果会在 JSON 文本块旁边包含 `structuredContent`。
  读工具声明 `readOnlyHint: true`；写工具声明 `readOnlyHint: false`/`destructiveHint: true`，
  其中 `undo` 和 `batch` 还带有 `idempotentHint: false`。客户端可以自动批准只读界面，
  而不必每次调用都提示；2025-06-18 客户端可以直接消费结构化结果。

- **MCP HTTP 传输并发处理请求。** 现在每个连接都运行在自己的线程上，拥有自己的 `Session`
  （最多 32 个在途请求），因此耗时数秒的反编译不再阻塞 `ping`、`tools/list` 或下一个客户端 ——
  之前测得：一次 6 秒的 `get_pseudocode` 调用让 `ping` 卡了 5 991 ms。携带非回环 `Origin`
  的请求会以 403 拒绝（DNS 重绑定防护，streamable-HTTP 规范要求），不支持的
  `MCP-Protocol-Version` 头返回 400，无法打开项目的请求返回 500，而不是断开连接。

- **MCP 与内置 agent 共用一层查询。** 读工具从 `recraft-mcp` 移到新的 `recraft-query` crate：
  一份 JSON schema 和一个执行器，由 MCP 服务器（`Budget::mcp`）和应用内 agent
  （`Budget::agent`，行/行数上限更小）调用，因此两个界面不再可能在名称、参数或结果形态上漂移。
  agent 因此免费获得 `list_imports`、`callgraph`、`search_bytes`、分页和地址校验，MCP 则获得
  `find_api_callers`。`get_pseudocode` 现在在所有地方都默认 **L2**（MCP 此前默认 L1，
  向外部客户端展示比 GUI 更差的渲染）；`function_summary` 还会返回原型和调用者函数列表。
  `recraft-mcp` 不再依赖空的 `recraft-app` 外壳，DESIGN §1 也不再声称 CLI/UI 通过一个
  并不存在的 app API 通信。

- **日志知道是谁写的，撤销会报告它还原了什么。** Schema v4 为日志新增 `origin`
  （`gui`/`cli`/`mcp`/`ai`/`user`）和 `batch`；现有项目就地迁移。`Session::execute_batch`
  在一个 batch id 下以单个事务应用多条命令，`undo` 还原最新命令 —— 或整个批次 —— 并返回
  被还原的记录（`seq`/`cmd`/`origin`/`ts`/`batch`）。MCP：每个写结果现在都携带 `seq`；
  `undo` 接受 `seq`（受限于最新应用的记录，因此客户端无法悄悄还原 GUI 用户后来的编辑）
  和 `dryRun`；新增的 `batch` 工具原子地应用 `{tool, arguments}` 条目，让 AI 的批量重命名
  只需一次撤销。单个写工具与 `batch` 共用同一个参数解析器（`Edit`），因此它们的形态不会漂移。

- **MCP 列表工具支持分页，伪代码按行分页。** 每个列表形态的工具（`list_functions`、
  `list_strings`、`xrefs_to`、`xrefs_from`、`callgraph`、`search_bytes`）都接受 `offset`，
  并在返回 `count`/`total` 的同时返回 `offset`/`next_offset`，因此一个有 31 803 个函数的
  DLL（shell32）可以被遍历到底，而不是停在 5 000 行上限；`callgraph` 还会报告 `edge_total`。
  `get_pseudocode` 接受 `startLine` 并返回 `start_line`/`end_line`/`returned_lines`/
  `next_start_line`，因此一个 13 084 行函数的第 2 001 行之后也可到达，而无需提高 `maxLines`；
  逐行的 `lines` 数组现在是可选项（`includeLines`），并且不再重复 `text`（该函数从 1.8 MB
  降到 0.7 MB）。`docs/MCP.md` 记录了分页契约。

- **MCP 可以在 `--write` 之后写入。** `recraft-cli serve-mcp <project> --write` 在读工具旁边
  注册 `rename`、`set_comment`、`set_prototype`、`apply_type`、`define_function` 和 `undo`。
  每次写入都经过与 GUI 相同的日志化命令总线，因此会连同其逆操作一起记录，`undo` 可以还原它；
  不带该标志时服务器保持只读，写调用返回 `isError: true`。HTTP 模式新增了
  `serve_http_writable`。测试覆盖该选择性开关、重命名在重新打开后仍然有效及其撤销、
  注释/原型的往返，以及只读拒绝（ADR-014 部分取代 ADR-008）。

- **CRT 启动调用会命名 `main` 或 `WinMain`。** 静态链接的 MSVC 程序没有入口符号，但它的启动
  代码以固定形态调用程序入口。控制台程序在 `ecx` 中得到 `argc`，在 `rdx`/`r8` 中得到
  `argv`/`envp`；GUI 程序在 `rcx` 中得到 `hInstance`，在 `rdx` 中得到 `hPrevInstance`，
  在 `r8` 中得到 `lpCmdLine`，在 `r9` 中得到 `nShowCmd`。两种情况都会保存返回值，随后立即
  运行退出路径（`__scrt_is_managed_app`）。`recraft-decompile::startup::find_entry` 跟随入口点
  的尾跳转（`mainCRTStartup` → `__scrt_common_main_seh`）并对调用分类，而 `apply_builtin` ——
  `analyze` 运行的步骤 —— 将其命名为 `main` 或 `WinMain`。内置签名集还新增了
  `__scrt_common_main_seh`，因此启动函数本身也会被命名。在 crackme 样本上，分析报告十个名称
  （八个 CRT 函数、`main`、`__scrt_common_main_seh`），L2 视图读作 `int32 main(void)`；
  在 notepad 上它报告一个名称，启动代码调用 `WinMain(...)`。`--no-signatures` 保留旧行为。
  两个样本的快照都变了（`sub_140001010.c` → `main.c`、`sub_1400012a8.c` →
  `__scrt_common_main_seh.c`、`sub_1400101d8.c` → `WinMain.c`）；全部 789 个通过。
- **DLL 的入口会被命名为 `DllMain`。** `_DllMainCRTStartup` 调用
  `DllMain(hinstDLL, fdwReason, lpvReserved)`，并检测 BOOL 结果，而不是把它保存给退出路径，
  因此形态与 `main`/`WinMain` 不同：`find_entry` 先尝试控制台/GUI 形态，回退到“调用前设置
  `r8` 加上随后几条指令内的 `test eax, eax`”。在 VS2022 构建的 DLL（`DllMain` 被命名）以及
  `main`/`WinMain` 样本上验证过。`wmain` 和 `wWinMain` 仍命名为 `main`/`WinMain`：在这个 CRT
  中，窄字符和宽字符的 argv 辅助函数字节相同，因此无法从代码上区分这两个构建（IDA 也是这样
  猜的）。32 位程序尚未覆盖：它们的入口参数压在栈上，且 SEH 包裹的启动代码会让此处的线性
  遍历看不到该调用，因此需要一次对 `__try` 布局建模的控制流遍历。

- **`analyze` 会匹配一次内置 CRT 签名，因此新项目打开时就带有真实名称。** 在函数发现和字符串
  提取之后，分析会为每个匹配八个内置签名的未命名函数命名；`--no-signatures` 关闭该行为，
  JSON 报告新增 `signature_names`，文本报告新增一行 `built-in signatures: N function(s) named`。
  GUI 的导入/重新分析工作线程调用同一步骤，因此打开项目就能看到 `puts`/`printf`/…，
  而无需先运行 `signature-import`。匹配逻辑移入 `recraft-decompile::signature`
  （`parse_document`、`user_name`、`apply`、`apply_builtin`），CLI 命令和 GUI 都使用它。
  它是幂等的：已命名的函数不会再被重命名。在 crackme 样本上，六个函数获得了 CRT 名称，
  因此它们的快照被重命名（`printf`、`puts`、`fgets`、`fflush`、`strpbrk`、`__acrt_iob_func`），
  三个调用点快照也变了；notepad、corpus、corpus86 和 Go 不受影响，全部 789 个快照通过。

- **内置 CRT 签名集，用 `signature-import --builtin` 匹配。** 二进制现在携带八个从样本
  crackme 导出的 x64 Visual C++ 静态 CRT 签名（`printf`、`puts`、`fgets`、`fflush`、
  `strpbrk`、`strcmp`、`strlen`、`__acrt_iob_func`），因此新项目无需准备签名文件即可命名：
  `recraft-cli signature-import <project> --builtin`。`--from` 现在是可选的（一个文件或一个
  包含 `*.json` 的目录）；两者都不传则是错误。第一个来源获胜，因此 `--from` 库可以覆盖内置
  名称。在 crackme 样本上，内置集命名了八个函数（九次匹配，因为 `fgets` thunk 与其目标共享
  一个签名），随后 `main` 读作 `puts`/`printf`/`fgets`/`strpbrk`/`strcmp`/`strlen`/
  `fflush(__acrt_iob_func(1))`。模式与构建相关：不同的 CRT 版本或编译器标志仍需要自己的库。
  一个单元测试解析内嵌文档并检查每个模式都有效。

- **`if` 内部的向后跳转变成 `while (1) { … continue; }`。** `fold_backward_gotos_into_loops`
  需要 latch 与标签同缩进才能构建 `do … while`；嵌套在 `if` 里的 latch 会被留成标签加 `goto`。
  现在从标签到其所在基本块末尾的整个区域都在 `while (1)` 下运行，跳转变成了 `continue`。
  该 pass 只在以下情况触发：对该标签的每一处引用都在该区域内、该区域没有 `break`/`continue`
  （它们会重新绑定到新循环）、其最后一条语句是 `return`/`goto`（否则包装器会在原代码落出的
  地方循环），并且它至多 200 行（包装巨型函数会为一个跳转缩进数千行）。crackme 的结构化
  函数从 459 增至 460，goto 回退函数从 36 降到 35；notepad 和 Go 不变。受影响的函数在快照
  样本之外，因此 789 个 golden 快照仍然通过；五个项目的 lint 均为 0 发现，222 个单元测试
  通过（新增 2 个覆盖该折叠和落空拒绝）。

- **跳到函数尾声的跳转会折叠成 `if (c) { … }`。** 当函数体的最后一个基本块位于最后一个
  `return;` 之后时，进入它的唯一方式就是指向它的跳转，因此该基本块可以被复制进分支并丢弃
  其标签。该 pass 在 `fold_dispatch_ladders` *之后*运行：在 goto 折叠循环内部，复制出来的
  文本会改变 if/else 阶梯的形态，某个分支可能以不同方式折叠（这在早前一轮尝试过并回退）。
  复制出的尾部可以包含跳到同一函数其他标签的跳转，但不能包含 `break`、`continue` 或标签定义；
  如果尾部已经以 `return;` 结尾，则不会再添加额外的 `return;`。两个 notepad 快照发生变化
  （`sub_140007570`、`sub_14000e950`，各一个 `goto` 加一个标签）。五个项目的 lint 均为 0
  发现，789 个 golden 快照通过，220 个单元测试通过。

- **当共享尾部本身结束路径时，条件跳转会折叠成 `if (c) { tail }`。** 先前的修复阻止了复制
  丢弃条件，但也拒绝了所有条件位置。以 `return` 或 `goto` 结尾的尾部会自行结束路径，
  因此在 `if (c) { … }` 下复制它是精确的；可能落空的尾部则保留其 `goto`。现在复制还会按
  尾部自身的基础缩进平移每一行，而不是逐行裁剪 —— 后者曾压平尾部的内部嵌套。crackme 的
  结构化函数从 459 增至 460，notepad 从 493 增至 494，同时有四个函数（每个样本两个）回到
  正确的 `goto`，因为它们的尾部确实会落空。五个项目的 lint 均为 0 发现，789 个 golden
  快照通过，220 个单元测试通过。

- **被引用两次的共享尾部会复制进两处，从而移除更多 `goto`。** 上一轮只处理恰好被引用一次的
  标签；两个分支落到同一尾部很常见（两个 `switch` case 落进同一个尾声），因此现在最多复制
  两处引用并丢弃标签。三处或更多引用则保持原样：它们没有带来进一步缩减，只会让重复代码
  成倍增加。crackme 的结构化率从 458 升至 462（92.5%→93.3%），goto 回退函数从 37 降到 33，
  notepad 从 493 升至 495（97.4%→97.8%），回退从 13 降到 11；corpus 和 corpus86 不变。
  十一个快照发生变化（尾部被复制进第二个位置），五个项目的 lint 均为 0 发现，789 个
  golden 快照通过。

- **共享尾部会被复制进跳转到它的分支，从而消除一类 `goto`。**
  `if (a) { P; L: T } else { Q; goto L; }` 是编译器在两个分支之间共享代码的方式。从 `L`
  到第一个分支末尾的区域会闭合该分支，因此它无法变成循环；去掉 `goto` 的唯一办法就是把尾部
  复制进第二个分支。该 pass 仅当标签恰好被引用一次、且尾部不包含 `goto`、标签、`break`、
  `continue`、`case` 或 `return`（它们会在新位置绑定到不同目标）时，才复制至多 20 行的尾部。
  notepad 的结构化率从 487 升至 493（96.2%→97.4%），goto 回退函数从 19 降到 13，标签从
  268 降到 260；crackme 从 455 升至 458（91.9%→92.5%），回退从 40 降到 37，标签从 201 降到
  192；corpus 和 corpus86 保持 100%。十个快照发生变化（尾部被复制，每个增加 11..14 行，
  标签和 `goto` 消失）。三个单元测试覆盖该复制、二次引用拒绝和循环控制拒绝；五个项目的
  lint 均为 0 发现，789 个 golden 快照通过。

- **函数签名可以导出和导入，因此一次学到的名称能到达另一个二进制。**
  `recraft-decompile::signature` 把函数头部变成字节模式，其中编码地址的字节被通配：相对分支、
  RIP 相对位移以及任何大到足以成为地址的立即数（`>= 0x10000`）匹配任意值，其他每个字节都
  必须精确匹配。增量链接 thunk 没有值得匹配的字节，因此导出和匹配都会看穿它直达实现。
  有两个命令使用它：`signature-export --out <json> <project>` 为项目已命名的每个函数写一条
  `{name, pattern}` 条目（通过 `rename`，或 `prototype` 里的名称），
  `signature-import --from <json> <project> [--dry-run]` 会为头部匹配的每个函数命名。
  已在使用的名称会被跳过 —— thunk 先被匹配，因此调用点读作源码级名称 —— 已经有名称的函数
  永远不会被重命名。在 crackme 样本上，导出写出十个签名（`main`、`printf`、`puts`、`fgets`、
  `fflush`、`strpbrk`、`strcmp`、`strlen`、`__acrt_iob_func`、`start`），把它们导入新项目后，
  `main` 无需任何原型就读作 `puts(…)`、`printf(…)`、`fgets(…)`、`strpbrk(…)`、`strcmp(…)`、
  `strlen(…)` 和 `fflush(__acrt_iob_func(1))`；而动态链接 CRT 的 notepad 什么也匹配不到，
  符合预期。已经带有名称的被调用者不再产生自引用的 `// inferred: printf = printf` 头部注释。
  两个单元测试覆盖模式格式；五个项目的 lint 均为 0 发现，789 个 golden 快照不变。
- **操作数本身已经写明的 `/* signed */` 注释会从 L2 中移除。** 打印器在无法重新定型操作数时
  会写出 `(a < b /* signed */)`，但值流通常会把有符号转换放回第一个操作数
  （`(int32)v1 < 3`），此时该注释只是在重复转换。变量恢复之后的一个 pass 会在外层括号组以
  有符号转换开头时删除它：crackme 的 L2 从 191 条注释降到 69 条，notepad 保持 35 条，
  `main` 的循环读作 `while ((int32)v1 < 3)`。一个单元测试覆盖两种形态；五个项目的 lint 均为
  0 发现（包括 Go 的 3261 段 L2 文本），789 个 golden 快照不变。
- **`fgets` 也能被识别，分类器现在会跟随增量链接 thunk。** `main` 调用了 `sub_140004bd4`，
  它是跳到真正 `sub_140004a60` 的一条指令 `jmp` thunk；现在角色解析会穿过最多两跳 thunk。
  `fgets` 的证据是三参数形态（缓冲区、大小、流）、写入内存的字节存储、`EINVAL`（0x16）和
  换行（0xa）常量，以及 —— 把它与 `_read` 区分开的部分 —— 返回值为第一个参数的副本，
  通过把入口 `r12 = rcx` 别名传播到 `rax` 写入来跟踪。没有最后这项检查时，同一样本的
  `_read`（`sub_140010950`：三个参数、字节存储、`0x16`）也会匹配；有了它，四个样本
  （1064 个函数）仍然只命名三个被调用者和一个可变参数函数。指令扫描此前还把 32 位操作数
  拼写与 64 位参数寄存器相比较，因此 `mov eax, ecx` 从未把 `rcx` 标记为传入；现在操作数先用
  `full_register()` 规范化，这才让三参数形态真正可见。`main` 现在读作
  `fgets(&var_20, 0x100, v5)`；五个项目的 lint 均为 0 发现，789 个 golden 快照验证通过
  （一个 crackme 快照跟随该名称），214 个测试通过。
- **一次复制随后对同一局部变量自我更新，会折叠成一次赋值。** MSVC 把 `i % 16` 编译成
  `mov rcx, rax; and ecx, 0xf`，值流将其命名为对同一个局部变量的两次赋值
  （`v12 = v10; v12 = (uint32)((uint32)v12 & 0xf);`）。这些语句相邻，因此没有东西会观察到
  中间值，把复制代入更新是精确的：`v12 = (uint32)((uint32)v10 & 0xf);`。该折叠仅在同一缩进下、
  对下一行读取其目标的普通 `=` 生效，在代入前给复合值加括号（因此 `a + b & 0xf` 不会改变
  含义），并在之后合并重复的转换（`v1 = (uint32)((uint32)(arg1) + 1)`）。crackme 的 L2
  又少了 61 行；五个项目的 lint 均为 0 发现，789 个 golden 快照不变，214 个反编译器测试通过。
- **只定义为一个常量、而下次调用已经写明该常量的局部变量，会从 L2 函数体中移除。**
  常量参数设置 `ecx = 0; call(0)` 在文本层面被折叠进调用，但当该寄存器在函数后面被复用时，
  死写清扫无法删除这条赋值，于是 L2 为一个没人读取的值保留了一条声明和一条赋值
  （`uint32 v4; v4 = 0;`）。变量恢复之后的一个 pass 会在以下条件全部满足时删除这一对：
  该局部变量恰好只有一个定义、其值是常量、函数中没有任何地方读取它，并且下一条语句是一次
  已经携带该常量的调用。仅仅看起来没被读取还不够 —— 一个定义仍可能到达值流无法命名的汇合点
  （`__asm` 之后的 `return rax`）—— 这就是为什么“被调用消费”是规则的一部分。crackme 的
  L2 在其 495 个函数中少了 198 行，`main` 不再声明 `v4`；一个单元测试覆盖该形态，五个项目的
  lint 均为 0 发现，789 个 golden 快照不变。
- **静态链接的 CRT 辅助函数根据其结构而不是符号表推断名称。** IDA 用 FLIRT 签名给它们命名；
  ReCraft 只有用户原型，因此 `crackme main` 仍然调用 `sub_140001170("Enter flag: ")`、
  `sub_140016b60(&var_20, …)` 和 `sub_140016be0(&var_20)`。现在有两条证据规则在渲染时给它们
  命名 —— 不会向项目写入任何内容，函数头部会列出推断了什么以及为什么
  （`// inferred: sub_140001170 = printf, …`）。被调用者的入口把四个参数寄存器全部溢出到
  home area 时，它就是可变参数的；当每个调用点都传入字符串字面量作为第一个参数时，它就是
  `printf`（宽字面量则是 `wprintf`）—— `fprintf`/`sprintf` 的调用者首先传入流或缓冲区，
  因此无法匹配。一个没有调用的被调用者若具体化 MSVC 的零字节扫描常量，当它入口读取的寄存器
  恰好是 `{rcx}` 时就是 `strlen`，当是 `{rcx, rdx}` 外加一个 `sbb`（无分支 -1/0/1 返回）时
  就是 `strcmp`；`memcmp` 还会读取第三个参数，因此被排除。在 crackme（495 个函数）、
  notepad（506）、corpus（31）和 corpus86（32）中，只命名了两个被调用者和一个可变参数函数，
  没有误报，`main` 现在读作 `printf("Enter flag: ")`、`strcmp(&var_20, "flag{…}")`、
  `strlen(&var_20)`。五个项目的 lint 均为 0 发现，全部 789 个 golden 快照验证通过，
  新增两个单元测试。
- **可变参数调用遗留的参数不再是调用者的参数，折叠进参数的常量也不再留下其设置行。**
  `main` 的签名曾是 `(int64 arg1, int64 arg2, int64 arg3, int64 arg4)`，因为 `recover_l2`
  看到 `rdx`/`r8`/`r9` 被 `printf("Enter flag: ", rdx, r8, r9)` 消费 —— 这些值是调用点为
  ABI 加载的，而不是源函数的参数。`scan_incoming_args` 现在根据格式字符串计算可变参数
  被调用者的参数个数（`1 + conversions`，当第一个参数不是字面量时回退到被调用者自己的计数），
  因此 `crackme main` 读作 `int32 sub_140001010(void)`，与源码的 `int main(void)` 一样，
  其调用者显式打印设置（`r8 = rdi; rdx = rbx; sub_140001010(*(u32*)(rax));`）。比较全部
  495 个 crackme 和 506 个 notepad 函数的 L2 签名，只有 `main` 变了。另外，死写清扫现在会
  删除一种参数设置：其常量下次调用已经写明（`ecx = 0; call(0)` → `call(0)`，
  `edx = 0; ecx = 0; sub(0, 0, 0, 0)` → `sub(0, 0, 0, 0)`），这让 27 个 crackme/notepad
  函数各减少 1–2 行。五个项目的 lint 均为 0 发现，全部 789 个 golden 快照验证通过，
  211 个反编译器测试通过。
- **可变参数调用的参数个数来自格式字符串，L2 局部变量按其读取宽度声明。**
  `printf("Enter flag: ")` 打印了调用点为 ABI 加载的遗留 `rdx, r8, r9` 寄存器，验证循环的
  字节累加器则变成由掩码拼出的 `uint64 v13`（`v13 = (v13 & 0xffff…ff00) | (uint8)(…)`）。
  两处改动解决了这两个问题：`is_variadic_target` 通过把四个参数寄存器全部溢出到调用者
  home area 的 MSVC 序言识别可变参数被调用者（`va_start` 会把它们读回），随后
  `trim_variadic_calls` 只保留字面量格式字符串消费的 `1 + <conversions>` 个参数（包括
  `*` 宽度/精度和 `%%`）；在 `variables.rs` 中，声明宽度现在取自最宽的**读取**（定义可能
  更宽 —— 编译器在 32/64 位寄存器中工作 —— 而收窄会像寄存器写入那样精确截断），部分写入
  不再算作全宽读取，冗余的寄存器宽度转换会被丢弃（`v = (uint32)(x)` 赋给同宽/更宽的局部
  变量变成 `v = x`，`v = (uint32)((uint32)v & 0xf)` 变成 `v &= 0xf`）。`crackme main`
  现在读作 `uint8 v13; … v13 = *(u8*)(&var_20 + v11); v13 ^= byte_140018368[v12];
  v13 += (uint8)v11; if (v13 != byte_1400183a0[v10]) …` 并调用
  `sub_140001170("Enter flag: ")`。L1 在一个快照中变化（crackme `main`）；
  notepad/corpus/corpus86/go 不变。五个项目的 lint 均为 0 发现，789 个 golden 快照验证通过，
  新增五个单元测试。
- **常量映像基址会折叠进它所加到的地址，地址读作数组下标。** `crackme main` 的验证循环在
  循环上方打印 `v3 = 0x140000000;`，在循环内部打印 `*(u8*)(((i + v3) + 0x18380))`，因此三张
  flag 表从未获得名称。现在三个文本 pass 协同处理这条链：`fold_constant_registers` 在赋值是
  该寄存器最后一次写入时把常量寄存器折叠进嵌套基本块（映像基址在序言中只加载一次；旧的
  “同一基本块、十行”窗口永远够不到循环体），并把 `*(T*)(…)` 操作数内部的读取视为折叠点；
  `merge_address_constants` 把 `(x + 0x140000000) + 0x18380` 合并为 `(x + 0x140018380)`，
  同时保持括号平衡；`fold_literal_arithmetic` 求值那些只有折叠之后才会相遇的配对
  （`(0 + 0x…)`、`(0x… - 0x…)`）；随后 `name_global_data` 把
  `*(u8*)((i + 0x140018380))` 渲染为 `byte_140018380[i]`。验证循环现在读作
  `byte_140018380[i]`、`*(u8*)(&var_20 + perm)`、`byte_140018368[i & 0xf]` 和
  `byte_1400183a0[i]`，与源码的 `perm`/`key_str`/`enc` 一致。打印器还会把
  `[rsp + index + disp]` 渲染为 `*(u8*)(&var_20 + i)`；旧的 `((rsp + i) + 0x20)` 拼写让
  L2 变量 pass 把 `rsp` 变成一个未初始化的局部变量（一个正确性 bug）。结构化率和 goto 计数
  不变（crackme 455/495，notepad 487/506）；notepad 丢弃死常量赋值
  （`r9d = 1; SendMessageW(…, r9d, …)` 变成字面量参数），使那个 9 777 行的函数少 24 行；
  五个项目的 lint 均为 0 发现，全部 789 个 golden 快照已刷新。四个单元测试覆盖这些新折叠。
- **一个调用只读工具的项目级 agent。** 面板此前只能回答光标下的函数；“持久化在哪里处理？”
  需要模型自己选择读什么。`ai ask <project> "<question>"` 和面板的 Analysis agent 框在
  `list_functions`、`function_info`、`pseudocode`、`xrefs_to`、`xrefs_from`、
  `find_api_callers`、`search_strings` 和 `disassemble` 上运行一个工具循环（最多 8 个模型
  轮次）—— 与 MCP 服务器暴露的是同一批证据。每个工具都是只读且有界的（列表/行/长度限制）；
  错误的工具名或参数会以 `{"error": …}` 返回给模型，让它自我纠正；始终不回答的模型会以明确
  消息失败。桌面端在工作线程上运行该循环，线程打开自己的 SQLite 连接，把每次调用的一行跟踪
  流式写入可折叠的“tool calls · N”区域，并把答案渲染为散文。重命名仍然是文本建议：agent
  从不写入项目。测试脚本化了该循环（工具结果按 `tool_call_id` 配对、未知工具恢复、轮次上限）
  和面板路径（跟踪 + 答案状态）；`docs/AI.md` 记录了这些工具和只读契约。
- **命名基准接受人工整理的别名。** `ai eval-names --aliases corpus/naming/aliases.json`
  在严格的 `exact` 旁边报告 `accepted` 计数：`read_config_file` 对 `open_config_file`
  算命中而不是未命中，而 `exact` 仍然只与真实符号比较。清单按基准函数手写，CLI 拒绝格式
  错误的 JSON。在 `deepseek-flash` 上测得：0–1/27 exact，**4/27 accepted（14.8%）**，
  18/27 partial（66.7%），平均重叠 0.52（启发式：0 accepted，3/27 partial）。模型输出是
  随机的，因此文档给出范围，并把 accepted/partial 视为稳定数字。
- **Agent 答案会流式输出，工具调用前的叙述会被撤回。** 工具调用轮次现在使用 SSE 解码器，
  在累积 `delta.tool_calls` 片段（index/id/name/arguments 跨事件拆分）的同时转发
  `delta.content`，因此最终答案会逐字打出，而不是在死转圈之后才出现。模型经常在工具调用前
  叙述（“I'll look for logging APIs”）；`AgentEvent::TurnStart/Retract` 让面板丢弃该文本并
  把它保留为跟踪上下文，CLI 则把它打印到 stderr —— 中文答案不再以英文旁白开头。忽略
  `stream: true` 的网关仍可通过一次性回退工作。解码器和撤回序列都有单元测试（内容、拆分
  参数、两次调用、叙述顺序）。
- **“Dig deeper” 在快速 Explain 路径与 agent 之间搭桥。** Explain 仍然只是关于一个函数的
  一次请求；它旁边的小 **Dig deeper** 按钮会用该函数的名称和地址预填 agent 问题，并在现有
  对话中运行循环，因此想要调用者/行为的读者不必自己输入地址。对话还上限为 60 条消息：
  最旧的轮次在问题边界处被丢弃，这会让每个 `tool_calls` 轮次紧挨着回答它的工具结果
  （有单元测试）。
- **agent 保留对话，答案会链接其地址。** 后续问题（“那它的调用者呢？”）现在基于上一轮构建，
  而不是重新开始：面板保存整个消息历史（系统提示一次、问题、工具往来和答案），
  **New chat** 按钮会丢弃它。`run_agent_with_history` 拥有该对话，因此 GUI 和单元测试走
  同一条路径。解释或 agent 答案中引用的地址会在结果窗格下方变成可点击链接（去重、仅限已知
  函数内的地址、最多 10 个），因此无需重新输入地址就能核对一项断言。
- **可度量的命名流水线，以及给它打分的基准。** Suggest names 过去会把批次浪费在地址最低的
  `sub_*` 函数上（按证据顺序，然后在提示里硬性截到 12 个），并把启发式回退当作模型命名呈现。
  候选现在按证据权重和调用者数量排序，每个提示块最多携带 14 行反编译函数体（旧提示一行都没有，
  因此命名全靠猜），候选会打包到 token 预算的 80% 而不是固定数量，每个提议都带一个 `source`
  （`model` / `heuristic` / `external`）并显示在面板中。`ai eval-names` 针对二进制仍然携带的
  符号给提议打分，对每个候选自身的符号（以及递归自引用）做遮蔽，让模型无法读到答案；`--llm`
  运行真实流水线，离线模式在没有密钥的情况下给启发式下限打分。`corpus/naming/` 是一个新的
  基准二进制，包含 26 个基于真实 Win32 API 和字符串的语义函数，由 `corpus/build-naming.ps1`
  构建。其基线：启发式 0/27 exact，平均 token 重叠 0.10；`deepseek-flash` 1/27 exact，
  9/27 partial（重叠 ≥ 0.5），平均 0.31 —— 在携带证据的 22 个候选上，1 个 exact、9 个
  partial（平均 0.38），而无证据函数得分为 0。打分有意取下界：`read_config_file` 对
  `open_config_file` 算 partial 而非 exact；人工整理的别名集是文档中写明的下一步。
- **AI 面板会批量应用建议，并让答案附着在其函数上。** 重命名行带有复选框以及全选 / 清除勾选，
  应用所选会通过命令总线应用每个勾选的提议（每次重命名都是自己的日志记录，因此撤销会一次
  收回一个）；被拒绝的提议会报告实际落地了多少个，而不是假装整批成功。解释/润色结果窗格会
  标出它所属的函数、链接到它，并在光标移到另一个函数时发出警告 —— 此前旧答案会悄悄看起来
  像是当前的。

- **AI 面板的 Explain 和 Suggest names 操作现在会真正询问模型。** 两者看起来是模型驱动的
  （它们以已配置的 provider 为门槛），但只是格式化本地证据，因此没有任何内容离开进程。
  Explain 现在构建与 `ai explain --llm` 相同的脱敏提示，并把散文式答案流式写入结果窗格；
  Suggest names 把有证据支撑的 `sub_*` 候选送入重命名提示，并把回复中的
  `0xADDR name -- rationale` 行解析回提议 —— 未匹配的候选保留其基于证据的回退名称，证据地址
  保持可链接，置信度至少提升到 0.7，不可用的回复则保留本地列表并如实说明。两者都在工作线程
  上运行，并共用直接回答的 token 上限（OpenCode Go 上为 8k），因此面板从不阻塞。重命名模板
  现在是 `rename-v2`，具有严格的行契约（由 `parse_rename_reply` / `apply_rename_reply` 解析，
  离线单元测试），CLI 新增 `ai suggest --llm`，`PROMPT_TEMPLATE_VERSION` 为
  `rc-ai-2026-11-v1`，因此旧模板的缓存响应会未命中一次并被重写。
- 一旦润色答案出现在屏幕上，AI 面板就会有一个 **Re-polish** 按钮。面板的 `PromptCache`
  通常会用相同文本立即回答重复的 Polish，这让对同一函数的第二次尝试看起来像失效的按钮。
  Re-polish 重新构建提示、跳过缓存查找并流式输出新答案，随后替换缓存条目；Polish 本身现在
  会把缓存命中报告为“polish done … (cached)”。该按钮只对润色结果出现（Explain 报告是散文，
  没有可重做的东西），并在作业运行时禁用。一个测试预置缓存，检查 Polish 复用它而 Re-polish
  启动新作业。
- **`Tab` 在反汇编与伪 C 之间切换，就像 IDA 一样。** 在列表中它反编译光标下的函数（与 `F5`
  完全相同：同样的视图切换、同样的光标行居中）；在反编译视图中它返回光标所在行的反汇编地址
  并保持该行高亮。该切换是视图变化而非跳转：它不添加导航历史条目，返回时会重新居中同一行，
  而不是再次反编译该函数。该绑定在 Ctrl/Cmd 组合键守卫之后读取，因此组合键仍会到达平台，
  `Space` 仍会切换 Graph/Text；辅助窗格（Functions、Strings、Hex、…）忽略 `Tab`。egui
  自带的 Tab 焦点导航在帧开头被取消，因为它会在第一次按下时把焦点环交给第一个可聚焦控件
  （菜单栏），并吞掉之后每一次；获得焦点的文本字段仍保留键盘，因此在过滤框中输入不会切换
  视图。四个测试覆盖按键绑定、从列表到伪 C 的方向、往返（锚点、高亮、历史、不重新反编译）
  以及无焦点环时的重复按下；欢迎页速查表、反编译提示行、`docs/DEMO.md` 和 `docs/M4.md`
  都以两种语言记录该按键。
- **文件 → 关闭项目（`Ctrl+W`）会卸载已打开的项目** 并返回欢迎屏，因此切换项目不再需要重启。
  关闭会先取消正在运行的后台分析（其工作线程拥有第二个 SQLite 连接）并丢弃 AI 作业，然后
  清除所有由项目派生的内容：列表、反编译视图、Functions/Strings/Names/Imports 列表、xrefs
  对话框、图、导航历史和 AI 结果。布局、主题、语言、provider 设置和最近项目列表不是项目
  状态，保持原样。该命令也在命令面板（`Ctrl+P`）中，并作为项目路径旁的 `×` 按钮；没有打开
  项目时它被禁用。五个测试覆盖完整卸载、分析取消、无项目空操作、`Ctrl+W` 组合键和命令面板
  命令。
- **WiX v5 MSI 安装程序**（`packaging/wix/recraft.wxs`、
  `dist/recraft-v<version>-windows-x64.msi`）取代 NSIS 安装包，成为项目唯一的安装程序。
  其布局遵循 PhotoCraft 安装程序（`storytold/photocraft`）：安装到 `%ProgramFiles%\ReCraft`，
  只需一次 UAC 提示，带有开始菜单快捷方式、应用和功能条目、`.rcraft` 项目关联（GUI 已经把
  项目或 PE 路径作为 `argv[1]` 接收）、供 Win+R 使用的 App Paths、取自 `assets/recraft.png`
  的向导美术，以及完成页上可选的“启动 ReCraft”。当 `vcruntime140.dll` 缺失时，启动条件会
  提前终止安装，因为 GUI 链接动态 MSVC 运行时；主版本升级会替换同版本重建。
  `scripts/package.ps1` 用仓库固定的 WiX 5.0.2 构建它（`dotnet tool restore`、
  `.config/dotnet-tools.json`；WiX 6/7 需要 OSMF EULA），把它纳入 `SHA256SUMS.txt`，
  `scripts/verify-msi.ps1` 校验表与提取的负载（`msiexec /a`，与暂存树逐字节相同）以及从
  提权提示符执行的安装/CLI/卸载循环。便携 zip 仍是每用户、免安装的路径。
- 暂存的 `docs/` 树现在复制整个 `docs/`（包括 `docs/cases/`），而不是只复制 `docs/*.md` 和
  `docs/adr/`，因此 zip、MSI 负载和仓库文档是同一套。
- 反编译视图现在像 Hex-Rays 一样给伪 C 着色：关键字和类型名蓝色、寄存器紫色、函数/标签/
  全局名金色、数字取立即数颜色、字符串字面量和注释绿色，而局部变量、标点和空白保持纯文本
  颜色。`append_pseudo_line` 在一趟中扫描每一行（注释和带引号的字面量会吞掉其内容），并且
  只追加 `LayoutJob` 段，因此行文本不变，单击/双击命中测试继续工作。两个主题共享现有的语义
  调色板；4 个新测试覆盖分类器、段拆分、注释/字符串包含以及渲染后的视图。
- AI 面板有一个 **Thinking mode** 开关。开启后，润色会保留 provider 的推理过程，流式输出的
  `reasoning_content` 显示在可折叠的“Thinking”区域中，该区域在模型推理时展开，并在答案
  完成时自动折叠。`Provider::complete_stream_with_reasoning` 转发推理增量（普通流式入口会
  忽略它们），UI 以 `AiMessage::Reasoning` 接收它们。
- AI 面板的 **润色结果像反编译视图一样有语法着色**。`AiResultKind` 记录结果窗格中存放的是
  润色后的 C（`Code`）还是散文报告（`Prose`）；`append_ai_result` 让代码走现有的
  `append_pseudo_line` 高亮器（关键字/类型蓝色、寄存器紫色、名称金色、数字和字符串/注释
  绿色），并让解释报告保持单一的纯文本段。复制按钮仍然复制未修改的文本，布局仍像以前一样
  按窗格宽度换行。

- 应用图标：六边形内带电路节点的几何薄荷色 **R**，由 `assets/generate_icon.py` 生成（PNG + 多尺寸 ICO + 原始 RGBA）。图标通过 Windows SDK 资源编译器嵌入 `recraft-ui.exe`，设为 eframe 窗口图标，并被 NSIS 安装程序使用。
- 把样本拖放到窗口上即可打开：`.rcraft` 项目直接加载，PE 文件被导入为同级的 `.rcraft` 项目，拖动时居中的拖放覆盖层会显示可接受的目标。拖入的文件会在下一帧之前处理。
- AI 现在是**仅侧栏**助手：没有 AI 停靠标签页。右侧栏默认显示，只能通过 `×` 或 Options 折叠，拖动其内边缘可调整大小（限制在 300–620px），绝不会变成按钮。折叠后它会留下一个**可拖动的浮动 AI 胶囊**（圆角、带阴影、前景覆盖层；不占布局宽度）；拖动它并单击可重新打开。其位置持久化在 `layout.json` 中。`F10` 和 AI 菜单项会打开侧栏。provider 设置位于头部的 ⚙ 弹出面板。
- provider 设置从内联面板移到 AI 头部的**齿轮弹窗**（也可从 Options 打开），因此主窗格保持聚焦于操作/结果。
- 把 AI 窗格重新设计为紧凑的助手仪表盘：标题/状态头部、圆角操作/结果/查询/建议卡片、带复制按钮的全宽可读结果区、按钮始终可见的专用查询行、置信度徽章和逐提议的证据链接，以及主要样式的 Polish 按钮。provider 设置仍是底部的可折叠区域。
- UI 本地化从内联 `t(en, zh)` 对迁移到 **Fluent**。`en-US.ftl` 和 `zh-CN.ftl` 通过 `include_str!` 嵌入；全部 148 个静态 UI 对、菜单栏、窗格标题（包括 `Hex` → 十六进制）、命令面板标签、图例/缩放控件和格式化状态文本现在都使用稳定的消息 ID，格式化状态用 `t_args`。测试强制各语言的消息 ID 一致以及回退行为。所选语言存储在 `layout.json` 中并在启动时恢复。
- AI 设置 API 密钥字段有**显示/隐藏**和**复制**控件，因此保存的（DPAPI 解密的）密钥无需编辑 `ai-settings.json` 即可查看或复制。
- AI provider 设置现在提供内置 **DeepSeek** 和 **OpenCode Go** 模式以及 `Offline` / `Custom`；选择模式会填充其端点和模型列表，设置面板直接接受 API 密钥。密钥**为当前用户密封**持久化（Windows DPAPI；`ai-settings.json` 中的 `api_key_sealed`）并在启动时预填；持久化设置仍存储端点/模型/密钥环境变量名。provider 请求发送 `User-Agent: recraft/<version>`，对 OpenCode Go 还发送稳定的 `x-opencode-session` 头。
- 当调用已经渲染捕获的值时，L2 调用点不再打印多余的参数寄存器赋值：`fgets((char*)(rsp + 0x20), 0x100, (void*)rax)` 不再重复 `rcx = …; edx = …;`。寄存器值参数保留其赋值，因此语义不变；回归测试已更新（`recursive(rcx)`）。
- AI 润色管线现在有端到端冒烟测试：`scripts/ai-polish-mock.ps1` 启动本地 OpenAI 兼容桩（`scripts/ai_mock_server.py`），配置 `RECRAFT_AI_PROVIDER=openai`，并在 crackme 项目上运行 `recraft ai polish`（prompt → HTTP → 缓存 → 打印建议）。该桩返回固定源码，因此它验证传输/配置而不是模型质量；真实 provider 走同一路径。
- 反编译器输出在字符串密集的函数中更像源码：命名存储字符串的地址常量渲染为带引号的 C 字面量（包括 `"\r\n"` 之类的短 `.rdata` 字面量），用户原型的函数名在调用点使用（`printf`、`fgets`、`strcmp`、…）而不是 `sub_*`。新增 `Session::string_at` 和一个 C 转义单元测试。
- 端到端 CTF 案例研究 `docs/cases/crackme_win_x64.md` 和复现脚本 `scripts/case-crackme.ps1`：字符串 → xref → 反汇编 → L2 反编译 → AI 润色源码。恢复出的 flag 是 `flag{shuffl3d_x0r3d_4nd_sh1ft3d}`；显而易见的 `flag{strings_are_not_enough_you_know}` 是被 `strcmp` 拒绝的诱饵。

- 导航到已知字符串地址现在会在**数据视图**中打开反汇编：字符串渲染为 `db '…',0`（UTF-16LE 则是 `dw`），带有解析出的名称和 `; DATA XREF: …` 注释，同一区域中后续字符串会作为更多数据行列出，而不是被解码为指令。这与 IDA 的 Strings → 双击行为一致，且不会显示垃圾反汇编。新增 `string_address_renders_data_rows`。

- 字符串视图现在像 IDA：四列**Address（segment:address）/ Length（十六进制）/ Type（C 或 C16）/ String**、斑马行以及跟随反汇编光标的持久选择。完整的存储字符串按其编码（ASCII 或 UTF-16LE）解码，而不是把 64 字节当作有损 UTF-8 读取，因此宽字符串不再乱码，长字符串不再被截断。单击选中并同步反汇编光标，双击导航。`隐藏头部/节名` 开关（默认开启）会去掉头部/节名噪声，同时保留子串过滤。新增 `strings_decode_encodings_and_filter_noise` 和 `docs/images/strings.png`。

- 默认文档标签现在遵循要求的顺序：**反汇编、反编译、十六进制、字符串、局部类型、导入、导出、AI**；**Names、Xrefs 和 Segments 默认隐藏**，仍可从 Views 菜单到达。`Hex View-1` 重命名为 **Hex**。已保存布局版本升到 8，因此现有布局回退到这个新默认；测试现在断言预期的默认标签和隐藏窗格。

- 重命名了两个主视图以匹配其功能：合并视图现在是 **Disassembly / 反汇编**（Graph 和 Text 模式在同一个标签内），伪代码窗格现在是 **Decompile / 反编译**。更新了菜单、标签、欢迎页快捷键（Space = Graph/Text）、演示文档和截图（新增 `docs/images/decompile.png`）。窗格 id 不变，只改显示名。

- 单一**代码视图**窗格（取代独立的 "IDA View-A" 和 "Text View" 标签）：一个停靠标签，内部 **Graph/Text** 模式存储为 `view_mode`。工具栏的 `Graph`/`Text` 按钮和 **Space** 切换同一视图的模式而不是切换标签；双击图块会把同一视图切到 Text 并导航。从列表/十六进制导航会在 Text 模式下聚焦代码视图。保存的布局现在记录一个视图加模式（`SavedLayout` v7）；窗格枚举简化为一个 `View` 窗格，旧的 Graph 标签已消失。

- 多窗口评估（ADR-012）：新增 `examples/viewport_spike.rs`，通过 `Context::show_viewport_immediate` + `ViewportBuilder` 打开第二个**真实 OS 窗口**。在 Windows 10 + 固定 Glow 后端上验证：两个原生顶层窗口，子窗口报告 `ViewportClass::Immediate`（不是嵌入式回退），分离窗口可独立截图（`docs/images/detached-spike.png`）。决定：v0.1 保留 egui_dock 的应用内浮动标签；把它们提升为 OS 窗口排在预览之后并附具体工作列表，而延迟的 `'static` 视口不在 M5 范围内。

- 代码视图缩放：**View → Zoom in / Zoom out / Reset zoom**（快捷键 `Ctrl+=`、`Ctrl+-`、`Ctrl+0`）缩放 Text View、伪代码和 Hex View 的等宽字体与行高（75 %–200 %），周围界面保持不变；egui 的全局 GUI 缩放快捷键被禁用，因此该快捷键只影响代码视图。列会自动跟随字体大小，缩放因子随布局持久化（`SavedLayout` v6）。新增 `zoom_actions_clamp_and_reset` 和一项持久化断言；截图 `docs/images/zoom.png`。

- 图概览现在是真正的迷你地图：它复用图视图的 CFG 布局（`graph_layout`），绘制缩放后的节点矩形及其边，高亮包含当前地址的基本块，并在图窗格完成布局后绘制图视图的可见视口框。点击节点会跳到该基本块。一个单元测试覆盖共享布局几何，截图为 `docs/images/graph-overview.png`。

- 导航带现在反映真实内容并与图例一致：可执行段为青色（指令），可读数据段绿色，未探索区域灰色；已发现函数以蓝色 "Regular function" 颜色覆盖上半部，导入/IAT 地址显示为品红色 "External symbol" 刻度，标记使用当前地址颜色，悬停显示地址 · 段 · 函数。图例复用同一组 `NAV_*` 常量，因此导航带和图例不再可能漂移。

- Text View 操作数语法着色：寄存器（紫色）、数字字面量/立即数（青色）、`sub_`/`loc_` 之类的符号名（棕色）和带引号字符串（绿色）分别着色，而不是整个操作数字符串用一种颜色；两个主题各有变体。操作数分类有单元测试（`operand_tokens_are_classified_like_ida`）。截图：`docs/images/text-view.png`。

- IDA 布局第 4 步（图视图 + 打磨）：**IDA View-A 现在像参考那样以图模式打开**（Text View 是文本文档标签）。图节点使用 IDA 风格外观 —— 深色主体、青色边框、带基本块地址和 `entry` 标记的浅色头部条 —— 以及逐指令的助记符/操作数/注释颜色；边是曲线，按种类着色（fallthrough/jump 青色、taken 绿色、not-taken 红色、跳转表紫色）并以箭头结尾。图工具栏新增缩放控件（`Fit`、`-`、`+`、50–200 %）和图状态行（缩放/基本块/十六进制同步）；Output 高度约 18% 与参考一致，Functions/图概览的分割也重新调整。已保存布局版本升到 4。截图更新为 `docs/images/ida-layout.png`，演示/changelog 已更新。

- IDA 布局第 3 步（反汇编渲染）：去掉单独的 "Name" 列，把名称渲染为 IDA 风格的内联标签（`start:`），放在指令上方的专用标签行；列现在是 Address / Bytes / Mnemonic / Operands / Comment，使用 IDA 颜色（黄色地址和标签、灰色字节、蓝色助记符、浅色操作数、绿色注释），选中/当前行高亮（IDA 蓝）也覆盖标签行。行保持紧凑的 15 px 高度，标签行则增加自己的基线。

- IDA 布局第 2 步（主题 + 字体）：在第一帧安装类似 IDA 的排版 —— 等宽文本用 Consolas、UI 用 Segoe UI、CJK 回退用 Microsoft YaHei（系统字体文件是可选的；egui 默认值作为回退）。统一深色调色板（面板 `#1e1e1e`、视图 `#141414`、文本 `#d4d4d4`）、IDA 的蓝色当前行高亮（`#264f78`）、更微妙的悬停色调、12.5 px Consolas 下的紧凑 15 px 代码行、更小的 UI 文本样式/按钮内边距，以及匹配新调色板的停靠标签/外观颜色（包括可见的标签关闭按钮）。字体文件缺失时（例如 CI）干净回退，因此无头测试继续通过（30 个 UI 测试）。

- IDA 布局第 1 步（对齐参考截图，不追求像素级）：用 IDA 风格工作区取代通用工具列 —— 菜单栏（File/Edit/Jump/Search/View/Options/Windows/Help）、工具栏（后退/前进、Text/Graph 切换、Analyze、AI、语言、项目路径）、带 IDA 风格图例的全局导航带、带 Function name / Segment 列和函数图标的 **Functions** 窗口、其下方的**图概览**迷你地图、文档标签行（IDA View-A、Graph View、Hex View-1、Pseudocode、Imports、Exports、Strings、Names、Xrefs、Segments、Local Types、AI）、带状态日志的 **Output** 窗口，以及带地址/字节/函数/十六进制同步字段的 AU 风格状态栏。新增 Exports 窗格（PE 导出）、视图页脚行、IDA 风格窗口标题（`ReCraft - <file>  <path>`），把项目加载/分析消息记录到 Output，并把已保存布局版本升到 3。截图在 `docs/images/ida-layout.png`；演示走查已更新；30 个 UI 测试通过。

- 真实停靠第 4 步（类似 IDA 的默认布局和外观）：默认布局现在镜像 IDA —— 左侧工具列（Functions/…/Structures）、中间反汇编/图、右侧停靠伪 C、底部横跨全宽的十六进制；反汇编和伪 C 窗格在窄时双向滚动，因此列不再被截断。egui_dock 外观调向 IDA（深色标签栏、紧凑标签、带色调的活动标签、微妙分隔线）。布局文件现在带版本（`SavedLayout`，v2），因此过时布局会回退到新默认，而不是复活旧排列；演示截图 `docs/images/dock-step1.png` 已随新布局刷新，布局往返测试检查版本拒绝。

- 真实停靠第 3 步（持久化 + 焦点快捷键）：停靠布局用 egui_dock 的 serde 特性序列化，并在布局稳定后自动保存到 `%LOCALAPPDATA%\ReCraft\layout.json`（分割比例、标签顺序、已关闭标签和浮动窗格），启动时恢复；`Reset layout` 清除文件并重建默认。在停靠区拥有有限矩形之前会跳过保存（egui 会把 NaN 序列化为 null）。新的窗格焦点快捷键：`F2` Functions、`F3` Names、`F4` Strings、`F6` Imports、`F7` Xrefs、`F8` Segments、`F9` Hex、`F10` AI、`F11` Structures，绑定在 Views 菜单中作为工具提示显示。测试覆盖 JSON 往返（关闭的窗格保持关闭）和 `F2` 焦点快捷键（30 个 UI 测试）。

- 真实停靠第 2 步（停靠区内的导航一致性）：每个工具窗格都保留双击跟随 —— Functions/Strings/Imports/Xrefs/Names/Segments 跳入反汇编并高亮目标行，Hex 双击和图块双击现在也会聚焦反汇编窗格，且从任何窗格导航都会聚焦反汇编停靠标签（而不只是切换逻辑视图）。`focus_pane` 现在还会设置聚焦的节点/表面，因此 `Views` 能可靠激活标签、重新停靠已关闭的窗格，并断言默认布局包含全部 12 个窗格。测试覆盖默认布局完整性、导航焦点和重新打开已关闭窗格（28 个 UI 测试）。

- 真实停靠第 1 步（目标：IDA 风格可停靠工作区）：新增 **egui_dock 0.21.1**（兼容 egui 0.36，MIT），用单一停靠状态工作区取代浮动工具窗口 —— 中间标签（反汇编/伪 C/图）、左侧工具列（Functions/Strings/Imports/Xrefs/Names/Segments/AI/Structures）和右下 Hex 窗格。标签可拖动以拆分/合并窗格、通过分隔线调整大小，并可拖出为浮动窗口；主视图标签不可关闭，工具标签可以。`Views` 菜单聚焦任何窗格（关闭时重新停靠）并重置布局；`Space` 通过聚焦相应停靠标签循环主视图。布局状态保存在 `ReCraftApp`（`DockState<Pane>`）中，停靠布局的截图签入在 `docs/images/dock-step1.png`。测试已按窗格/停靠 API 更新（25 个 UI 测试通过）。

- IDA 风格第 6 步 —— 数据/类型操作：在当前反汇编行上 `C`（代码）、`D`（数据）、`A`（ASCII 数据）、`U`（取消定义）和 `P`（创建函数），在右键菜单中与移除函数、Edit prototype… 和 Structures… 一起镜像；全部经过日志化命令（可撤销）并刷新每个视图。新的 **Structures** 窗口列出用户布局，从 `0:tag:u32, 16:value:i32` 字段规格定义/更新它们，并把布局绑定到所选函数的参数字段（复用 `DefineStruct`/`ApplyType`）；原型编辑可通过行菜单用现有编辑对话框进行。`A` 把范围标记为数据 —— 字符串提取仍是派生的，因此它列出可打印串而不是存储用户字符串。测试覆盖标志标记、结构体定义/参数绑定、原型设置/清除和字段解析器。

- IDA 风格第 5 步 —— 视图联动与外观：反汇编获得 IDA 风格**导航带**（带当前位置标记的段映射，单击跳转）；伪 C 视图高亮与当前反汇编地址匹配的行，单击同步反汇编光标（`Action::SyncDisasm`，留在伪 C），双击跳到反汇编标签；十六进制字节单击也会同步反汇编光标；调色板向 IDA 颜色靠拢（金色地址、柔和蓝色助记符、浅黄色名称、绿色注释），状态栏显示所选名称和所属函数范围。测试覆盖 SyncDisasm 语义；演示走查已更新。

- IDA 风格第 4 步 —— 分离工具窗口：Functions/Strings/Imports/Xrefs/Names/Segments（以及 AI）现在是独立可移动/可调整大小/可关闭的 egui 窗口，而不是一个固定左面板中的标签；顶栏新增 **Views** 菜单以切换每个窗口，`X` 打开 Xrefs 窗口，每个列表都支持双击跳入反汇编并高亮目标行（新的 **Names** 窗口由 `Session::names()` 支持，名称和字符串都有过滤器）。布局现在让中央反汇编/伪代码/图视图占满全宽；测试无头渲染所有窗口并覆盖 X 快捷键。

- IDA 风格第 1 步 —— 整行反汇编交互：反汇编现在每行是一个全宽命中区域（没有网格单元），因此单击任何位置都会选中该行并高亮整行（IDA 当前行颜色加悬停色调）；双击任何位置跟随操作数；右键打开上下文菜单，含 Follow operand/Rename/Comment/Cross references/Copy address/Undo/Redo；工具栏新增可用的 ◀/▶ 后退/前进按钮（新的 `Action::Forward` 和 `forward()` 复用现有历史栈）。测试覆盖后退/前进往返；提示栏和演示走查已更新。

- 可用性第 2 轮 —— AI 面板中的离线自然语言**查询框**：`urls`、`callers of <api/function>` 和 `text <substring>` 意图复用 `answer_query`，结果显示种类/地址/文本/函数，单击条目反向跳入反汇编；Clear 也会重置答案。无需 CLI 即可使用离线自然语言查询。

- 可用性第 2 轮 —— 导航增强：**Functions** 侧列表新增过滤框（名称/地址子串，不区分大小写），并显示解析后的函数名而不是裸范围（悬停保留精确范围；双击跳到反汇编）；**伪 C 行单击**现在反向跳入反汇编视图，而不是重新居中伪代码视图；**Xrefs** 列表继续显示引用类型（call/jump/data），获得跟随提示，双击跳到反汇编标签中的引用指令。新测试覆盖过滤匹配器、并行函数标签和反向跳转动作。

- 可用性第 2 轮 —— 反编译器可见噪声清理：在伪 C 中抑制栈指针簿记（`rsp = rsp ± N`），把 `mov r11, rsp` 帧命名为 `frame`，折叠代数噪声（`xor reg, reg`、`and reg, 0`、`sub reg, reg`、`x ± 0`、移位 0），修复渲染为指针转换的值转换（`(uint32*)(x)` → `(uint32)(x)`），并把尾调用渲染为 `return sub_…();` 而不是注释。语料 golden 快照已重新生成（31/31 干净），notepad 指标不变（502/99.4%/86.1%）；新增 golden 噪声回归测试，docs/M2 记录了剩余 frame/var 工件和 SSA/phi 评估（推迟：成本超过预览收益，保留 SSA-lite 寄存器模型）。

- 可用性第 2 轮 —— 无需 CLI 配置即可使用 AI 面板：面板内 **Provider settings**（种类/端点/模型/API 密钥环境变量/最大 token）持久化到 `%LOCALAPPDATA%\ReCraft\ai-settings.json`（`ProviderSettings::load/save/load_from/save_to`；密钥从不存储，`enabled` 由环境重新计算），外加 **Explain**（离线函数摘要/API/字符串/调用，渲染在面板中）和 **Polish**（构建润色提示并在后台线程调用已启用的 provider，因此 UI 从不阻塞）。Clear 重置解释/状态；文档 AI/DEMO/README 已更新。

- 可用性第 2 轮 —— 底部**十六进制转储**面板（DESIGN §8 后续）：跟随选中的反汇编/伪代码地址（16 字节行、地址栏、ASCII 列）、字节/字显示切换（小端 16 位字）、单击选中字节/字、双击导航到该处反汇编、“Follow selection”按钮重新锚定，以及顶栏 **Hex** 复选框显示/隐藏它。无头测试覆盖行对齐、选择同步和字节/字渲染。

- 可用性第 1 轮（预览后反馈）：首次运行的**欢迎面板**带快捷操作（Open project / Import PE）、持久化的**最近项目**（最多 8 个，存于 `%LOCALAPPDATA%\ReCraft\recent.txt`，缺失文件会被清理）和快捷键速查；反汇编**跟随**导航通过双击或 Enter（解析操作数文本中第一个已映射的十六进制地址/符号，没有时回退为状态提示，仍记录在后退历史中）。无头测试覆盖 token 提取、最近项目去重/上限、跟随导航和欢迎页渲染。

- v0.1 预览版**决定不签名**发布（代码签名推迟）：README 新增“安装（未签名预览构建）”一节（SHA-256 校验、SmartScreen “更多信息 → 仍要运行”、每用户安装程序），`docs/RELEASE.md` 记录该决定、免费 OSS 途径（SignPath Foundation、Certum 的 ≥1k-star 政策）以及恢复时的确切签名步骤；M4/ROADMAP 把签名标为推迟而不是阻塞。

- NSIS 安装程序已生成并端到端验证（DESIGN §13）：本次会话成功安装了 NSIS 3.13（`winget install NSIS.NSIS`），`scripts/package.ps1` 现在在安装程序之后写校验和，因此 `SHA256SUMS.txt` 覆盖 `setup.exe`，安装程序在每用户安装（不提权）的同时写入 `Publisher`/`DisplayIcon`/`InstallLocation`/`QuietUninstallString`/`NoModify`/`NoRepair`。新的 `scripts/verify-installer.ps1` 执行静默安装到临时目录、断言文件/开始菜单快捷方式/卸载注册表项、运行已安装的 CLI（notepad：502 个函数，99.4% 覆盖率）、启动 GUI 并检查其窗口，然后静默卸载并验证目录/快捷方式/注册表已消失（在 `finally` 中清理）。在 Windows 10 上用自定义 `/D=` 目录和默认 `%LOCALAPPDATA%\ReCraft` 本地验证；`verify-package.ps1` 还检查安装程序哈希和 PE 头，CI 新增 `installer` 作业（choco NSIS → -RequireInstaller → verify-package → verify-installer → artifact）。ZIP 现在 33 个条目 / 10.3 MiB，setup.exe 10.3 MiB。

- 为 DESIGN §16 的每个决定编写 ADR：`docs/adr/` 现在包含 ADR-001 和 ADR-003…ADR-011（无头核心、SQLite 容器/标志、用户数据与派生数据、自有反编译器阶梯、无 IDAPython/自有 API、AGPL+商业+CLA、MCP propose/apply、Windows 优先/核心可移植、依赖白名单、1 GB 目标）以及详细的 ADR-002；docs/README 索引该目录，M4 审计将 §16 标记为完成。

- 增量范围重分析（DESIGN §6.4）：`AnalysisOptions.seed_range` 把种子发现限制在一个范围（范围起点成为额外的代码种子），`recraft_analysis::reanalyze_range` 只替换与该范围相交的自动函数/名称/xref/API 标签，其他每个函数逐字节不变（`Session::replace_auto_*_in_range`）。暴露为 `recraft reanalyze <project> --addr A --len N [--json]`；`mark` 现在在代码/数据变更后自动触发范围重分析。语料单元测试断言范围外无泄漏且未受影响函数保持精确起止；E2E 覆盖 CLI。docs/M4 审计表和 M2 获得 §6.4 证据，ROADMAP 去掉该缺口。

- 许可证/声明合规（DESIGN §12）：新增 `NOTICE.md`（直接依赖、SQLite 公有领域、Noto CJK OFL、样本政策）、`scripts/check-license.ps1`（校验必需法律文件、生成 golden 快照之外所有手写源文件的 SPDX 头，以及 NOTICE 对直接依赖的覆盖），并把它接入 CI 的 `licenses` 作业、与 cargo-deny 并列。docs/M4.md 新增 DESIGN §0–§16 验收表及逐节证据：v1 功能已实现；§6.4 函数级增量重分析和 §8.4 i18n 实现选择仍是已记录的偏差，而安装程序验证/代码签名仍受外部阻塞。

- UI 命令面板（DESIGN §8.1 "menu / command palette"）：`Ctrl+P` 或 `F1` 打开可模糊过滤的列表（open/import/analyze/undo/redo/view switch/goto/rename/comment/xrefs/AI suggestions/language），方向键选择、Enter 执行；状态栏提示以 EN/zh-CN 列出该快捷键。无头 egui 测试驱动真实的 `Ctrl+P` 事件（含 `Event::ModifiersChanged`），过滤到 "rename"，按 Enter 并断言编辑对话框打开。

- DESIGN §10 CLI 便利：只读命令（`strings`、`imports`、`funcs`、`disasm`、`decompile`、`xrefs`、`cfg`、`metrics`、`golden`、`search`、`callgraph`）现在直接接受 PE 文件路径，并自动导入+分析到临时项目（SQLite 项目按头部识别并照旧使用）；`recraft analyze <pe> --out project.rcraft` 执行设计中的一次性导入+分析形式。E2E 覆盖两条路径；README 记录了它们。

- UI 键盘流程完成 DESIGN §8.2：`X` 现在为当前指令打开交叉引用面板（侧标签 `Xrefs` + 刷新的 xref 列表），加入现有的 `Space`（视图切换）、`N`（重命名）、`;`（注释）、`G`（goto）、`Esc`（后退）和 `Ctrl+Z/Y`（撤销/重做）；状态栏提示以 EN/zh-CN 列出新快捷键。新的无头 egui 测试发送真实 `Key::X` 事件并断言面板切换。

- AI 自然语言查询和伪代码润色（完成 DESIGN §9.2）：`recraft ai query <project> "<question>" [--json]` 识别离线意图（`urls`、`api_callers`、`function_callers`、`text`），从本地表（字符串/xref/API 标签/函数）作答，从自由形式问题中提取标识符（`CreateFileW`、`Microsoft`）并支持 zh-CN 措辞；`recraft ai polish <project> --addr <f>` 通过配置的 provider 和 `POLISH_SYSTEM`（保持语义的润色）发送 L1 输出，打印建议并附“未存储”横幅，复用 prompt 缓存并写审计记录；离线时以配置提示失败。新增 `POLISH_SYSTEM`/`build_polish_prompt`、四个查询单元测试（URL/api-caller/未知目标/text）、一个润色提示测试、E2E 查询断言以及 docs/AI.md + ROADMAP 更新（DESIGN §9.2 现已完全覆盖）。

- 标志语义工作之后的发布重建：`package.ps1`（zip 10.2 MiB，23 个条目）→ `verify-package.ps1` PACKAGE PASS → `demo.ps1` 在暂存二进制上 DEMO PASS。打包的 exe 复现了 notepad 的源码指标（502 个函数，99.4% 覆盖率，432/502 = 86.1% 结构化），剩余 239 条内建指令；docs/RELEASE.md 和 docs/M4.md 记录了这些数字。

- 消费标志的指令语义：`bt` 现在记录 CF（= 被测试的位），消费分支把它重建为 `((value >> bit) & 1) != 0`（之后只定义 jb/jae）；`js`/`jns` 对 `cmp`/`sub`/`test`/位运算/`inc`/`dec` 变成产生结果与零的精确有符号比较；`setcc` 从跟踪的比较写入 0/1 而不消费它。溢出/奇偶条件现在保留为汇编，而不是打印错误的占位符。notepad：“未建模标志”分支从 270 降到 23，语义覆盖率从 98.5% 升到 **99.4%**（239/37522 内建指令），结构化率从 84.7% 升到 **86.1%**（432/502）；语料保持 100% 结构化 / 97.4% 覆盖率。提升测试覆盖 bt/jae、add/js 和 cmp/sete；docs/QUALITY、docs/M2、docs/M4 和 CHANGELOG 已更新。

- 共享尾部链复制：当分支目标是一条已发射的直线链（≤24 条语句、以 `return`/尾调用结尾、没有循环头或 switch 分派）时，结构化器会在到达它的分支内重新发射该链，而不是打印跳到更早发射代码的 `goto`。notepad 结构化率从 81.5% 升到 **84.7%**（425/502），残留 `goto` 行从 1181 降到 1088，回退函数从 93 降到 77；覆盖率保持 98.5%（561/37522）。corpus bytes_op golden 快照现在显示复制的提前返回尾声（语义不变，已审阅）；语料保持 100% 结构化 / 97.4% 覆盖率。docs/QUALITY、docs/M2、docs/M4 已更新。

- 针对真实样本的函数范围与结构化修复：(1) 分析现在把重叠的函数范围钳制到下一个已知起点（一个合并的 `sub_*` 范围横跨数十个真实函数，导致反编译选错），并且 (2) 结构化器会在到达已发射 *叶子* 尾声（≤12 条语句、以 `return` 或尾调用结尾）的分支内重新发射它，而不是打印跳到共享尾声的 `goto`。notepad：502 个不重叠函数，结构化 **81.5%**（409/502，从 407 上升），覆盖率 **98.5%**（561/37522 内建指令），残留 `goto` 计数降到 1181；语料保持 100% 结构化 / 97.4% 覆盖率，golden 快照不变。docs/QUALITY、docs/M2、docs/M4 已更新。

- CFG/语义工作之后的发布重建：`package.ps1`（zip 10.2 MiB，23 个条目）→ `verify-package.ps1` PACKAGE PASS（哈希、必需条目、打包 exe 流水线）→ `demo.ps1` 在暂存二进制上 DEMO PASS。打包的 exe 复现了 notepad 的源码指标（502 个函数，98.5% 覆盖率，81.1% 结构化）；docs/RELEASE.md 和 docs/M4.md 记录了该次运行。

- 间接调用和 IAT 尾调用：`call qword ptr [IAT]`/`call reg` 变成 `Stmt::CallIndirect` 并打印 `(*{fnptr})()`；`jmp qword ptr [rip+disp]` 建模为间接调用加返回（尾调用）。寄存器/索引间接跳转保持不变，以便跳转表检测。对 notepad 的实际影响：其 1571 条间接调用内建指令变为零，因此语义覆盖率从 94.2% 升到 **98.5%**（571/37602 内建指令），结构化率不变为 81.1%（407/502）；语料保持 97.4%（10/391），golden 快照干净。新的提升测试覆盖两种间接调用形式；docs/QUALITY、docs/M2、docs/M4 和 CHANGELOG 记录了这些数字。

- 语义覆盖 + 诚实的指标：非对齐/SSE 移动（`movups`/`movdqu`/`movaps`/`movdqa` 内存操作数）现在通过 `MemorySize::size()` 解码（此前 `Packed128_*` 未映射并回退为 `__asm`），x86 循环移位（`ror`/`rol`）被提升并打印为 `ror(a, b)`，`metrics --json` 新增 `intrinsic_top` 直方图。`build_cfg` 现在丢弃从入口不可达的基本块（对齐 `int3`、跳转表字节、空隙），`measure` 只统计 CFG 可达指令：在 notepad 上可达指令数为 37602，其中 2183 条内建指令，因此语义覆盖率为 **94.2%**（线性扫描时报告的是 84.9%），结构化率 **81.1%**（407/502）；`int3` 内建指令从 8547 降到 33。语料覆盖率 97.4%（10/391），golden `nested_loops` 因现在建模的 `movdqa` 加载而更新；docs/QUALITY、docs/M2、docs/M4 记录了修正后的指标定义。

- E2E 中的 MCP HTTP 传输：`scripts/e2e.ps1` 现在在随机端口上用生成的 token 后台启动真实服务器，等待 `/health`，断言携带 token 的 `initialize` 返回 `serverInfo`，验证不带 token 的请求以 401 拒绝，并总是在 `finally` 块中停止进程。docs/MCP.md 和 docs/QUALITY.md 记录了覆盖情况；补充了现有的 stdio 步骤。

- PE 加载器的确定性畸形输入回归（DESIGN §11 离线模糊替代）：`crates/recraft-format/tests/malformed.rs` 喂入 500 个随机 blob、400 个 notepad.exe 截断以及 DOS/NT 边界切分，还有 400 个头部/节表位翻转变异；加载和每个公共访问器都绝不能 panic，成功加载的变异体要通过节查找、RVA 映射、权限、导入/导出/TLS/pdata 演练。作为工作区测试的一部分约 0.06 s 跑完；cargo-fuzz 仍是仅 nightly 的可选额外项。docs/QUALITY.md 记录了它。

- M2/MCP/CLI 工作之后的发布回归：重建 `dist`（zip 10.2 MiB，23 个条目）并重跑完整链条 —— `package.ps1` PASS、`verify-package.ps1` PACKAGE PASS（哈希 + 条目 + 打包 exe 的 info/import/analyze/decompile/AI/metrics，502 个函数），以及在暂存发布二进制上运行 `demo.ps1` → DEMO PASS。docs/RELEASE.md 和 docs/M4.md 记录了该次运行和剩余的安装程序阻塞项。

- CLI `batch`（DESIGN §10）：`recraft batch <project> --script <file.json> [--dry-run] [--json]` 通过日志化 Command 总线按顺序执行 JSON 编辑命令列表，在第一个失败处停止并给出其 index/type。支持的命令：`rename`、`comment`（regular/repeatable/anterior）、`prototype`、`define_struct`（对象或 `offset:name:kind` 字段字符串）和 `apply_type`（`struct` 字段避免与 `type` 判别符冲突）。符号地址会针对项目和同一脚本中较早的重命名解析（因此 dry-run 无需应用即可工作），容忍 UTF-8 BOM，`--dry-run` 只校验不写入。E2E 覆盖 dry-run + apply + 重命名验证；README 展示了该命令。

- 带 bearer token 认证的 MCP HTTP 传输（DESIGN §9.1）：`recraft serve-mcp <project> --http [--bind 127.0.0.1:8765] [--token <token>]` 每个 `POST /mcp` 请求处理一条 JSON-RPC 消息，要求 `Authorization: Bearer <token>`（否则 401），通知返回 202，坏 JSON 返回 400，其他路径/方法返回 404/405，并暴露未认证的 `GET /health` 探测；省略 token 时会生成并打印到 stderr。实现使用 std TCP、有界头部/正文和 `Connection: close`。单元测试覆盖缺失/错误 token、initialize、tools/call 和 health；CLI 冒烟验证了 200/401。docs/MCP.md 记录了该传输。

- CLI `search` 和 `callgraph`（DESIGN §10 无头缺口）：`recraft search <project> --bytes <hex>|--string <ascii> [--limit] [--json]` 扫描可读映射段（分块）并用包含命中的函数标记；`recraft callgraph <project> [--addr <filter>] [--limit] [--json]` 打印由 CALL xref 构建的函数级调用边。字节扫描移入 `Session::search_bytes`，MCP 的 `search_bytes` 工具现在委托给它。E2E 演练两个命令；README 列出示例。

- 带类型的调用参数：当被调用者有用户原型时，指针参数会把渲染的参数转换为声明类型（`int str_probe(const char* text)` 的 `str_probe((char*)rcx)`），而标量参数为可读性保持不转换。新语料探针 `call_str` 保留真实的直接调用点（并在 `str_probe` 上加 `noinline`）；语料现在有 31 个函数，仍是 100% 结构化、97.2% 覆盖率，golden 快照从干净项目重新生成（只新增 call_str 快照）。测试断言调用点的指针转换。

- 穿过序言 push 的 x86 栈参数恢复：`recover_x86` 现在跟踪累计的 `esp` 调整（push/pop/sub/add），并把观察到的 `[esp+N]` 读取映射回传入参数偏移，为同一参数插入每个观察到的偏移。在读取参数之前保存寄存器的函数现在得到真实的参数名/类型（例如 `int32 __cdecl recursive(int32 n)`、`// L2 params: n=[esp+0x8]`），而不是空签名；未知的栈指针改写保留保守回退。测试已重命名/更新，以断言一个先 push 的函数恢复两个参数。

- x86 栈调用参数：提升器记录 `push` 值（最新 push 在前）并把它们追加到 `Stmt::Call` 参数，因此当被调用者有用户原型时，cdecl/stdcall 调用点会渲染压入的参数（Win32 上的 `recursive(eax)`，x64 的第 5 个及之后参数同理）；帧调整会清除过时候选，调用在使用后清除它们。测试：x86 被调用者原型参数渲染（加上现有 x64 IR 调用捕获测试）。Docs/M2 记录了剩余的 x86 原型签名注意事项（自动栈槽恢复可能被序言 push 破坏）。

- 调用参数恢复 + 跨函数原型传播：提升器按基本块跟踪寄存器值（`reg_env`）并在 `Stmt::Call { args }` 上记录 x64 调用参数；当被调用者有用户原型时，伪代码渲染声明数量的参数（例如 `int recursive(int n)` 的 `recursive(rcx)`），而没有原型的调用保持 `name()`（golden 快照不变）。自引用参数表达式（调用前的 `dec ecx`）回退为寄存器名以保持语义正确；x86 栈参数尚未渲染。测试：IR 调用捕获、反编译器原型/参数渲染。

- 发布产物验证：`scripts/verify-package.ps1` 对照 SHA256SUMS.txt 检查 zip，断言必需的暂存文件（两个 exe、LICENSE、含 FAQ/ROADMAP/M4/DEMO 的 docs、VERSION.txt），并让**提取出的**二进制跑 info/import/analyze/decompile/`ai provider`/metrics；已接入 CI 打包作业。重建 dist（10.1 MiB，23 个条目）并在本地验证（502 个函数）。NSIS 安装程序生成仍未验证：本次会话没有交互式 UAC（`winget install NSIS.NSIS` 和静默 NSIS 安装都在提权处被取消），因此免提权的绿色 zip 仍是官方产物；docs/RELEASE.md 记录了安装程序的确切 VM 步骤。

- 复杂循环头（循环顶部的短路守卫，其头部条件无法干净地拆成留/离）现在回退为结构化的 `while (1) { <结构化函数体> }`，用 break/continue 承载出口，而不是发射 goto。语料达到 **30/30 结构化（100%）**，覆盖率 97.2%；notepad 观测 78.7%（395/502）。nested_loops golden 重新生成（无 goto）；测试断言回退保持结构化。

- switch case 内联：目标是简单叶子（基本块以 `return` 结尾或跳到 switch 汇合点）的跳转表现在渲染为 `switch (sel) { case N: { ... } }`，而不是 `case N: goto loc_X;` 加带标签的函数体；相邻分组的 case 共享一个函数体，复杂/共享的 case 函数体保留之前的标签/goto 形式。corpus `switcher_big` 现在已结构化：29/30 个函数（96.7%），覆盖率 97.2%；golden 重新生成（30 个快照）；测试断言内联的 `case` 形式且没有 goto。

- do-while 识别：头部同时也是 latch（单基本块）的循环现在渲染为 `do { ... } while (cond);`，而不是 `while (1) { ... if (cond) break; }`，而“有 latch 条件、没有头部条件”的形式由专用 do-while 路径处理。语义保持不变（`while (1)` + `break` 与 `do..while` 在这里等价，已在 golden 差异中验证）。新语料探针 `count_bits`；语料现在 30 个函数 / 93.3% 结构化 / 97.2% 覆盖率，notepad 观测 77.7%（390/502）。golden 快照重新生成（30 个）；测试断言 do-while 输出。

- 结构化器 break/continue：循环体现在会为离开到循环出口的分支发射 `break;`，为回到循环头的分支发射 `continue;`（通过循环上下文栈跟踪），跳到外层区域汇合点的跳转会落空而不是打印 `goto`（同时为循环体之外、既非 break 也非 continue 的目标保留显式 goto）。Notepad 的结构化率从 62.4% 升到 **77.3%**（388/502，高于 v1 的 70% 目标），修复 75 个函数，diff 回退集时零回归；语料保持 93.1% 结构化 / 97.1% 覆盖率。golden 快照不变；测试断言发射 `break;`。

- M4 预览材料：README 刷新（当前状态、功能对比、mermaid 架构、文档索引链接）、新增 docs/FAQ.md（十个问题，含 AI 隐私、许可证和安全）、docs/ROADMAP.md（里程碑状态 + 近期优先级）和 docs/README.md（文档索引）。scripts/check-docs.ps1 校验每个相对 Markdown 链接（18 个文件中的 52 个链接）并在 CI 中运行。还移除了误跟踪的 MSVC 链接器产物（functions.obj/.exp/.lib）并忽略这些扩展名。

- 结构化器质量：基于 `test`/`cmp` 的有符号条件跳转（`jle/jg/jl/jge` 及匹配的 `cmov`）现在从仅零标志状态建模，if/else 打包接受已位于外层区域停止集中的汇合点，因此多目标提前退出链和“守卫后循环”形态无需 `goto` 即可结构化；空分支不再发射空 `if` 块。新语料探针 `early_chain`，golden 快照重新生成（29 个函数），语料结构化率 93.1% / 覆盖率 97.1%，notepad 观测从 49.2% 升到 62.4% 结构化。测试：提升层有符号 jcc、结构层无 goto 检查。

- 持久化 AI 响应缓存 + 审计日志：`ai explain --llm` 现在查询 `%LOCALAPPDATA%\ReCraft\ai-cache.json`（键 = 模板 + provider + 模型 + prompt），重复解释无需网络 I/O 即可返回（`cached=yes`；`--no-cache` 绕过读取），新响应尽力保存。每次 provider 交互都会向 `ai-audit.jsonl` 追加一条 JSONL 记录（ts/provider/model/template/function/tokens/cached）。新增 `scripts/ai-cache-smoke.ps1` 通过 TCP 启动本地 mock Ollama，并断言未命中 → 命中 → 两条审计行（已接入 CI）；单元测试覆盖缓存往返、键组合和审计追加。

- Prompt 模板版本化 + 离线提示质量基线：`PROMPT_TEMPLATE_VERSION`（`rc-ai-2026-10-v1`）标记导出的系统模板，`eval.rs` 定义 `EVAL_VERSION`（`prompt-eval-2026-10-v1`）和五个语料案例，它们组装真实解释提示并断言其包含函数证据（API/字符串/伪代码）、保持脱敏（无原始 URL）并符合 token 预算。以 `cargo test -p recraft-ai --test prompt_eval` 和 `recraft ai eval <project> [--json]` 运行（5/5 通过；失败时非零退出）。模型答案质量评分仍是后续工作。

- HTTPS LLM 传输：`UreqTransport`（ureq + rustls）服务 `https://` 端点，而 `TcpHttpTransport` 为本地 Ollama 保留纯 `http://`；`transport_for` 按 scheme 选择，provider 现在使用它。`TransportKind`/`transport_kind` 有单元测试，一个被网络忽略的测试验证针对 crates.io 的 TLS 握手（`cargo test -p recraft-ai https_tls_handshake_to_crates_io -- --ignored`）。OpenAI 兼容云端点不再报告 "https unsupported"。

- M4 预览的演示包：`scripts/demo.ps1` 运行九步 CLI 走查（PE 头、导入/分析、函数发现、L2 伪代码、重命名+原型、结构体布局/apply_type、在真实样本上的 AI triage/propose/apply/undo、MCP stdio 握手、质量指标）并以 `DEMO PASS` 结束；`-Keep` 把项目留给 GUI。`docs/DEMO.md` 新增 CLI 脚本、GUI 走查、MCP 客户端配置、本地 Ollama 设置和演示讲稿；`docs/M4.md` 跟踪预览状态。

- x86 原型现在按偏移顺序（`[esp+N]`/`[ebp+N]`）把声明的参数映射到恢复出的栈槽，包括名称和类型，因此带 `int __stdcall stdcall_add(int a, int b)` 的 32 位函数渲染为 `int32 __stdcall stdcall_add(int32 a, int32 b)`，函数体使用 `a`/`b`。`ArgNames` 新增 `stack_types`（签名使用声明类型，默认 int64）和 `apply_prototype_x86`；新增 x86 语料测试。

- 数组下标渲染：通过指针类型参数的缩放寻址（`[rcx+rax*4]`、`[rcx+rax<<2]`、`char*` 上的字节偏移）打印为 `arg1[index]` 而不是原始指针运算；指针推断现在也会从索引访问学习元素宽度。新语料探针 `array_probe`（`const int*` 上的 `p[i]`）、新 L2 测试、golden 快照（28 个函数）、指标（89.3% 结构化，96.5% 覆盖率）和文档已更新。

- 原型声明现在会被解析并驱动反编译器签名：返回类型、`__cdecl`/`__stdcall`、参数类型（`int32`/`int64`/`char*`/`wchar_t*`/`int32*`/`int64*`/`void*`）和参数名在 x64 上替换推断值（最多四个寄存器参数），函数体使用声明的名称。未知或不支持的声明符（函数指针、数组）回退为逐字的 `// prototype:` 注释。新增 `parse_prototype`/`ProtoSig`/`ProtoParam` API 及解析器测试和覆盖/撤销反编译测试；E2E 断言被覆盖的签名。

- 用户结构体布局 + `apply_type`（DESIGN §9.1）：`recraft define-struct --name Probe --size 24 --field 0:tag:u32 --field 16:value:i32` 存储布局（schema v3：user_struct/user_struct_field/func_arg_type，幂等迁移），`recraft apply-type --addr <func> --slot 0 --type Probe` 把它绑定到参数字段，`recraft structs` 列出布局，省略 `--type` 会清除绑定。所有写入都经过 `Command::DefineStruct`/`ApplyType`（日志化、撤销/重做、持久化；未知类型被拒绝）。反编译器为绑定的参数渲染具名字段（`arg1->value`），而不是 `arg1->field_10`。AI 新增 `ProposalKind::ApplyType`（值 `"<slot>:<Type>"`）。测试：会话撤销/重做/持久化、反编译器具名字段/清除、AI 提议应用；E2E 定义/应用/撤销一个结构体。

- 用户原型（写工具，DESIGN §9.1）：`recraft prototype <project> --addr <func> --text "int foo(int)"` 通过 `Command::SetPrototype` 存储 C 声明（日志化、撤销/重做/持久化），`recraft unprototype` 删除它；结构化伪代码在生成的签名上方显示为 `// prototype: ...`。Schema v2 新增 `prototype` 表（从 v1 幂等迁移）。AI `ProposalKind::Prototype` 可以通过 `ai apply` 携带相同的写入。新测试：命令撤销/重做/持久化、反编译器渲染、AI 提议应用；E2E 设置/渲染/撤销一个原型。

- 结构体字段访问渲染（M2 类型）：当指针类型参数在常量偏移处被解引用时，伪代码为整数和浮点加载/存储打印 `arg1->field_10`（负偏移为 `field_nXX`），而不是 `*(u32*)((arg1 + 0x10))`。新语料探针 `field_probe`（`const int*` 上的 `p[4]`）演练 `[rcx+0x10]`；golden 快照、指标（27 个函数，88.9% 结构化，96.4% 覆盖率）和文档已更新。

- 打包（M3）：`scripts/package.ps1` 构建发布二进制，暂存绿色 `dist/ReCraft-<version>` 目录（exe、LICENSE/README/CHANGELOG、VERSION.txt、docs），生成 `recraft-v<version>-windows-x64.zip`，写 `SHA256SUMS.txt`，并在存在 `makensis` 时可选地从 `packaging/installer.nsi` 构建 NSIS 安装程序。CI 新增 `package` 作业，把 zip + 校验和作为 artifact 上传。文档：`docs/RELEASE.md`。

- 可注入 `Transport` trait 背后的 LLM HTTP 传输：`TcpHttpTransport` 说纯 `http://`（对 127.0.0.1:11434 的本地 Ollama 足够；`https://` 在 TLS 栈落地之前会报告明确的不支持错误），`MockTransport` 支持离线测试请求形态、认证头、错误状态和响应解析。`OllamaProvider`（POST /api/chat，stream=false）和 `OpenAiCompatibleProvider`（POST {endpoint}/v1/chat/completions，Bearer 密钥）都返回模型文本；`provider_from_settings` 保持启发式 provider，除非配置被显式启用。新增 `recraft ai explain --llm` 发送脱敏、符合预算的提示并打印答案（默认配置以明确错误拒绝）。

- LLM provider 契约 + 提示安全层（尚无网络 I/O）：`ProviderSettings` 默认离线，仅当种类、端点和 API 密钥环境变量都存在时才启用 HTTP provider；`redact` 掩码 URL/IP/电子邮件；`TokenBudget` 以约 4 字符/token 的预算强制截断；`PromptCache` 和 `AuditLog` 提供缓存/审计簿记；`build_explain_prompt`/`build_rename_prompt` 从解释（API、字符串、L2 伪代码）组装最小上下文。新 CLI：`recraft ai provider` 显示解析后的设置（从不显示密钥），`recraft ai prompt` 打印脱敏、符合预算的提示以供审查。

- 1 GiB 性能门禁：`scripts/bench-large.ps1` 构建 1 GiB 覆盖样本，测量导入/分析墙钟和峰值工作集（每 20 ms 轮询子进程，因为 PowerShell 的 Start-Process 会丢失我们需要的值），并在超出 DESIGN 阈值时失败（导入 < 60 s、分析 < 5 s、峰值 <= 512 MiB）。已接入 CI。实测（调试）：导入 4.03 s、分析 0.32 s、峰值 5.6 MiB。文档：`docs/PERF.md`。

- UI AI 提议面板：新的左侧 "AI" 标签列出重命名提议及其置信度、理由和证据链接；选中一条并按 Apply 会通过同一个日志化命令总线发送（可撤销），而任何东西都不会被隐式应用。`suggest_names` 一次性预加载字符串/xref 表，因此生成建议不再随函数数二次增长。无头 UI 测试渲染面板并端到端应用一条提议。

- M3 AI 层（离线部分 + 写协议）：`recraft ai explain` 从 API 标签、被引用的字符串和调用边构建函数摘要；`recraft ai suggest` 为 `sub_*` 函数生成带证据地址、置信度和理由的重命名提议；`recraft ai triage` 汇总 API 类别、加权可疑函数和 URL。`recraft ai apply <file> --yes` 是唯一的变更路径，走日志化/可撤销的命令总线；不带 `--yes` 会拒绝。新增 `Session::all_api_tags` 支撑聚合。文档：`docs/AI.md`、`docs/M3.md`。

- 指针参数类型：用作直接内存基址的寄存器参数按唯一访问宽度定型（1 字节为 `char*`、2 字节为 `wchar_t*`、4/8 字节为 `int32*`/`int64*`、混合为 `void*`、从未作为基址时为 `int64`）。新语料探针（`str_probe`、`wide_probe`）验证 `char*`/`wchar_t*`；L2 渲染 `int32 str_probe(char* arg1)`。Golden 快照文件名现在基于符号而不是带索引前缀，因此插入语料函数不再重命名现有文件。

- SSA-lite 参数跟踪：CFG 上的 may-write 前向数据流（入口集合为空，汇合点 = 前驱出集的并集）为第二趟提升提供种子，因此 `argN` 只对每条路径上都完好的传入值打印；汇合点处被破坏的值回退为具体寄存器视图。单元测试覆盖带一个分支参数写入的菱形上的数据流。

- 基本浮点类型：SSE 标量内存操作数（`movss/movsd` 和浮点算术）成为带类型的加载/存储（`f32`/`f64`），而不是原始 `u32`/`u64`，浮点算术使用专用的 `BinOp::FAdd/FSub/FMul/FDiv`，int<->float 转换选择带类型的源操作数。16 字节向量访问打印为 `__m128`。`float_math` 的 golden 快照已重新生成。

- L2 签名：返回类型推断（函数写入完整 `rax` 时为 `int64`，`eax` 宽度结果为 `int32`，值转发调用/尾调用为 `int64`，否则 `void`）和调用约定检测（x86 上 `ret imm16` -> `__stdcall`、`ret` -> `__cdecl`；Windows x64 fastcall 保持隐式）。新的语料 stdcall 探针在 x86 上验证两条路径。L2 输出示例：`int32 add_leaf(int64 arg1, int64 arg2)`。

- 语义提升第 1 批：`push`/`pop`（栈指针副作用和内存访问，排序使打印的表达式看到旧的 `rsp`）、`imul`（2/3 操作数）、`cmovcc`（新的 `Expr::Select` 保持惰性标志有效），以及 SSE/浮点子集（XMM 寄存器文件：`movd/movq/movdqa/movdqu/movaps/movups/movss/movsd`、`padd*/psub*/pmulld/pand/por/pxor`、`psrldq/pslldq`、标量 `addss/addsd/.../divsd`、`cvttss2si/cvtsi2ss`）。语料语义覆盖率 80.6% -> **96.2%**（v1 目标 95%）；CI 覆盖率门禁提高到 0.9。Golden 快照已重新生成。

- 质量门禁（DESIGN 7.4）：`recraft metrics` 报告指令语义覆盖率（内建回退）和结构化成功率，`--min-structured` / `--min-coverage` 把它变成会失败的门禁；`recraft golden` 写入或检查规范化 L1 快照（`0x...`/`loc_...` 被替换为占位符）。已提交的语料快照在 `tests/golden/corpus` 下；CI 运行语料门禁加 golden 往返。基线：24 个函数，结构化 87.5%，覆盖率 80.6%（与 95% v1 目标的差距已记录）。文档：`docs/QUALITY.md`。

- M2 MCP 只读服务器：`recraft serve-mcp <project>` 在 stdio 上说换行分隔的 JSON-RPC 2.0，提供 DESIGN 读工具（`list_functions`、`get_disassembly`、`get_pseudocode` L0/L1/L2、`xrefs_to/from`、`list_strings`、`list_imports`、`callgraph`、`search_bytes`、`function_summary`）和项目资源（`functions`、`strings`、`imports`、`pseudocode/{func}`）。写工具留在 M3，采用 propose/apply 协议。新增 `Session::all_xrefs` 支撑调用图。文档：`docs/MCP.md`；E2E 现在包含 MCP stdio 冒烟步骤。

- M2 switch 选择器恢复：跳转表携带被跟踪的选择器寄存器（`mov eax, ecx; ...; mov ecx, [base+rax*4+tbl]; jmp rcx` -> `switch (ecx)`），当选择器仍是完好的参数时 L2 渲染为 `argN`。Case 函数体仍是带标签的区域。测试：L1 选择器和 L2 参数选择器。

- M2 L2 Win32 x86 参数恢复：有帧指针时，`[ebp+8..]` 栈槽变成 `arg1..`；没有时，`[esp+4..]` 也可以（直到 `push`/`sub esp` 使偏移不稳定，此时启发式保持保守）。栈存储永远不会获得参数名。`int64` 仍是占位类型。测试：x86 add_leaf 参数恢复、移位栈辅助函数回退到 `var_XX` 槽位。

- M2 L2 变量恢复（初版）：参数寄存器（Windows x64 rcx/rdx/r8/r9）通过写前读取检测，在完好时渲染为 `arg1..arg4`；栈槽变成 `var_XX`；签名列出恢复出的参数。暴露为 `recraft decompile --l2` 和 UI 伪 C 标签中的 "L2 (vars)" 开关。尚无 SSA（跨基本块破坏跟踪和类型是已记录的后续工作）。

- M2 switch 结构化：CFG 现在携带其跳转表（分派地址 -> 按表顺序的目标），L1 结构化器发射 `switch (index) { case N: goto loc_...; }`，case 函数体作为带标签区域追加。选择器寄存器仍是占位符（记录在 `docs/M2.md`）。

- M2 L1 结构恢复：保守的结构化器用支配/后支配和回边循环检测把 CFG 变成 `if`/`else` 菱形、提前退出 `if` 和自然 `while` 循环；不支持的形态回退为带标签的 `goto` 区域，不丢弃基本块。暴露为 `recraft decompile --structured` 和 UI 伪 C 标签中的 "L1 (structured)" 开关。文档：`docs/M2.md`。

- M1 后台分析：UI 的 Analyze 操作在拥有自己 SQLite 连接的工作线程上运行；顶栏显示已用时间和 Cancel 按钮。取消是协作式的（`AnalysisOptions { cancel }`，在函数之间和每 1024 条指令检查），返回 `Error::Cancelled`；完成的运行通过派生数据表刷新每个面板。基准（调试构建）：`kernel32.dll` 292 ms / 79,797 条指令（含字符串）；1 GiB 覆盖样本分析 317 ms（覆盖层未映射，因此不被扫描），导入 3.88 s。

- M1 函数边界：MSVC 冷 `.pdata` 片段（热/冷拆分）在 fallthrough 连续或被条件分支指向时合并回其所属函数；x64 语料相对链接器映射的精确率从 0.857 升到 1.000。
- 函数指针表发现：可读数据段中对齐的指针若落在可执行内存中，就成为 `DataPointer` 种子，恢复从未被直接调用的分派处理器（语料召回率 0.833 -> 1.000）。`recraft eval --map` 用 MSVC `/MAP` 文件作为每个函数符号的真值，语料构建现在生成 `corpus.map`。

- M1 符号还原：MSVC C++ 名称（`?name@scope@@...`、构造函数、重载 —— 保留参数类型，丢弃返回类型）和 Rust legacy/v0 名称通过 `msvc-demangler` + `rustc-demangle`（去掉 legacy 哈希后缀）。在加载时应用于导出和导入名称，因此反汇编、符号查找和 `funcs` 显示可读名称。新增 `recraft demangle <name>` 命令和无 CRT 的 C++ 语料（含命名空间、类方法和重载）。

- M1 Go 运行时恢复：扫描可读段中的 `pcHeader` 魔数（Go 1.16 / 1.18 / 1.20+ 布局，包括移除 `textStart` 的 Go 1.26），校验候选，然后遍历 `functab` → `_func` → 以 NUL 结尾的 `funcnametab` 名称。恢复出的函数成为高优先级种子，其真实名称（`main.main`、`runtime.*`）持久化为自动名称。新 Go 语料（用 `-gcflags=all=-l` 构建）：2,997 个 pclntab 函数，发现 3,088 个，295 ms 内 147k 条指令；`recraft funcs` 现在显示函数名。

- M1 跳转表：CFG 识别绝对指针表（`jmp [index*scale+table]`）和 MSVC x64 映像相对表（`lea base,[image_base]; mov dst32,[base+index*4+disp]; add dst,base; jmp dst`），校验至少两个函数内目标。表目标成为 leader 和新的 `JumpTable` 种类边（Graph 标签中为紫色）。语料新增 `switcher_big`（23 个基本块，20 条表边）。

- M1 控制流图：`recraft-analysis::build_cfg` 把函数拆成基本块（leader = 入口、分支目标、分支后指令），用 fallthrough/taken/not-taken/jump 边连接，并跟踪外部尾调用分支；`layers()` 给出 BFS 深度布局。UI 新增 Graph 标签绘制节点和贝塞尔边（单击节点跳转），`recraft cfg --addr` 打印或发射块表为 JSON。测试覆盖分支、循环（回边）、notepad 的入口 CFG 和无头图渲染。

- M1 UI 侧面板：Segments / Functions / Strings / Imports / Xrefs 标签。Strings 有实时子串过滤（渲染前 2000 个匹配）和单击导航；Imports 显示 `module!api [category] ×refs` 并跳到第一个调用点；Xrefs 列出对当前选中行的引用并可跟随。反汇编现在显示 `sub_...` 和 IAT 注释。新的无头测试针对真实已分析 notepad 项目渲染全部五个标签。

- M1 导入命名 + API 标签：IAT 槽位在加载时命名为 `MODULE.dll!Api`，反汇编用解析出的名称注释直接内存操作数和调用目标（`jmp qword ptr [IAT] ; COMCTL32.dll!#413`），分析为未命名函数写入 `sub_<hex>` 名称（auto 种类，永不覆盖加载器/用户名称）。有序子串规则把导入归类（network/file/registry/process/thread/crypto/service/anti-analysis/ui/memory）为派生的 `api_tag` 行。新 `recraft imports` 命令列出 `notepad.exe` 的 340 个导入 API 及类别和引用计数；analyze 在 37 ms 内发射 422 个 API 标签和 442 个自动名称。

- M1 字符串 + xref：ASCII/UTF-16LE 字符串提取（分块、跨块串延续、ASCII 尾部误报守卫）和为直接调用、分支和数据引用（绝对/RIP 相对）构建 xref。作为派生数据持久化到 `string` 和 `xref` 表。新增 `recraft strings` 和 `recraft xrefs --addr` 命令；`analyze` 报告所有计数。`notepad.exe`：3,578 个字符串、618 条调用边、1,533 条分支边、1,348 个数据引用，34.6 ms。语料测试验证指向横幅字符串的 `lea rax,[rip+…]` 数据 xref。

- M1：自动函数发现（`recraft-analysis`）—— 从 `.pdata`、导出、入口点和 TLS 回调播种，然后用带尾调用检测的递归下降。精确的 `.pdata` 边界；函数作为派生数据（auto 标志）持久化，不覆盖用户定义。
- `corpus/`：无 CRT 的 MSVC 基准（14 个导出），导出 + `.pdata` 作为真值；`corpus/build.ps1` 通过 `vswhere` 定位 MSVC。
- `recraft eval` / `recraft analyze` CLI 命令，外加 UI 中的 "Analyze functions" 按钮（F5）。基线：`notepad.exe` 召回率 1.000 / 386 个 `.pdata` 边界中 386 个；`kernel32.dll` 在 2460 个可分析函数上召回率 1.000、精确率 0.955。
- `docs/M1.md`：M1 状态、指标和后续步骤。

- M0 完成：端到端验收（`scripts/e2e.ps1`）覆盖导入 → info/disasm → 重命名 + CJK 注释 → L0 反编译 → 撤销/重做 → JSON → 代码/数据标记，外加可选的 1 GiB 覆盖冒烟（导入 4.44 s）。已接入 CI。
- `docs/M0-REPORT.md`：交付的能力、实测指标、已知限制和 M1 计划。README 现在将 M0 标为完成。
- 项目设计文档 v0.1（`docs/DESIGN.md`）。
- 带计划 crate 图的 Cargo 工作区骨架。
- `recraft-common`：地址、范围、id 和共享错误类型。
- M0 实现计划（`docs/M0.md`）。
- ADR-002：egui 对原生 UI 可行（spike 通过：20 万行网格 p95 0.62 ms，500 节点 CFG 在 CPU 布局 + 细分下 p95 2.43 ms）。
- `recraft-storage`：单文件 SQLite 项目容器（schema v1）、带进度/取消和 SHA-256 的 256 KiB 分块映像导入、撤销日志（追加/撤销/重做/截断）、只查询的读取连接。1 GiB 导入实测 4.6 s；重新打开 3.1 ms。
- `recraft-format`：PE32/PE32+ 加载器 —— 头、节、导入（含延迟导入）、导出（含转发器）、TLS 回调、x64 `.pdata`、覆盖层、RVA 映射、`Loader` trait。畸形的导入/导出数据会降级为警告而不是让加载失败。冒烟：`notepad.exe`（8 个节、340 个导入、386 个 `.pdata`）和 `kernel32.dll`（1,292 个导入、1,697 个导出、1,530 个 `.pdata`）干净解析。
- `recraft-decode`：基于 iced-x86 1.21 的无状态 x86/x64 解码 —— `Insn` 包装器带 `FlowKind` 分类、直接分支目标、内存操作数检测、Intel 格式化器（为 UI 拆分助记符/操作数）和有界解码缓存。10 个单元测试；无状态吞吐实测约 46 M 条指令/s。
- `recraft-ui`：基于 **Glow** 渲染器的原生 egui 0.36 桌面外壳（wgpu/DX12 在 spike 时于 Intel 驱动上崩溃）—— 虚拟化反汇编 Grid，含地址/字节/助记符/操作数/名称/注释列，L0 伪 C 窗格带单击跳转，段/函数侧栏，goto 对话框，重命名/注释对话框，撤销/重做，键盘优先导航（G/N/;/Space/Esc/Ctrl+Z），CJK 字体回退和中英文切换。包含一个无头 egui 布局测试，在没有窗口的情况下渲染真实的 400 行网格。
- `recraft-cli`：`recraft` 命令行 —— `info`（无需项目的 PE 检查）、`import`、`disasm`、`decompile`、`funcs`、`rename`/`unname`、`comment`/`uncomment`、`mark`、`undo`、`redo`，带 `--json` 输出和符号/`entry` 地址解析。在 `notepad.exe` 上端到端验证：导入 → 入口反汇编 → 重命名/注释（CJK）→ L0 伪 C → 撤销 → 在新进程中重新打开仍显示编辑。
- `recraft-ir`：寄存器视图（al/ax/eax/rax 作为一个基础寄存器）、表达式区域、直线语句、mov/movzx/movsx/lea/算术/inc/dec/移位/cmp/test/call/jmp/jcc/ret 的 x86/x64 提升，以及惰性标志（只跟踪供给下一分支的比较）。不支持的指令变成保留原始汇编的内建指令 —— 提升从不失败。10 个单元测试。
- `recraft-decompile`：**L0 伪 C** 渲染器 —— 扁平、忠实于指令的类 C 输出，带真值/有符号位移格式、范围内分支的局部标签、范围外跳转的尾调用渲染、调用/名称解析和用于交叉高亮的逐行源码地址。在 `notepad.exe` 上 2 个集成测试。
- `recraft-core`：项目数据库之上的会话 —— 来自 PE 加载器的段映射、每字节 4 位标志（带 64 KiB 页缓存）、名称/注释/函数、页缓存映像读取（VA → 文件偏移）、线性反汇编视图，以及带日志化逆操作的命令总线（撤销/重做、重做栈失效、原子应用）。PE 元数据以 JSON 持久化在项目中，因此重新打开不会重读映像。10 个单元测试（重命名/注释/代码-数据/函数撤销重做持久化）。

### 变更

- **MCP 客户端文档以 OpenCode 为先。** `docs/MCP.md` 的客户端章节现在展示 `opencode mcp add`
  以及 `mcp.servers` 的 `local` / `remote` 配置形态（包括用于 header-token HTTP 端点的
  `oauth: false`，以及 `opencode mcp list` 的重新加载怪癖），而不是 Claude Desktop 的
  `mcpServers` 示例 —— 后者曾是唯一被记录的客户端。`docs/DEMO.md`、`docs/FAQ.md` §9
  和两个 README 现在都点名同一个客户端，因此文档中不再有任何内容教用户配置
  Claude Desktop / Cursor。

- **工具输出 schema 逐字段声明。** 每个读工具（位于 `recraft-query`）和写工具现在都声明
  真实的 `outputSchema` —— 记录在案的字段、它们的类型，以及其中哪些总会出现的字段 ——
  而不是一个光秃秃的 `{"type": "object"}`。测试让文档保持诚实：一个测试校验每个 schema，
  然后运行七个读工具和五个写工具，断言结果中既没有未记录的字段，也不缺任何必需字段。
  客户端可以直接校验或渲染 `structuredContent`。

- **stdio 上的资源订阅。** `serve-mcp`（stdio）现在声明 `resources.subscribe: true` 并实现
  `resources/subscribe` / `resources/unsubscribe`；在任意写入或 `undo` 之后，服务器会为每个
  已订阅的 URI 发送一条 `notifications/resources/updated`，由 stdio 循环冲刷。HTTP 传输
  保持无状态（每个请求一个 `Session`），因此它声明 `subscribe: false`，并以 `-32601`
  拒绝该调用，而不是假装支持。

- **CLI 与 MCP 共用一套批量脚本语法，且 MCP 可以定义结构体。** 命令解析器从 `recraft-cli`
  移到 `recraft-core::script`（`COMMAND_TYPES` + `parse_command` + 共享的
  `resolve_address`），因此 `recraft-cli batch` 和 MCP 的 `batch` 工具接受相同的
  `{"type": ..., ...}` 条目；MCP 也仍然接受 `{tool, arguments}` 对。语法新增了
  `define_function`（CLI batch 之前无法表达它），MCP 新增了 `define_struct` 工具，从而补上了
  “`apply_type` 存在，但布局只能在 GUI 里创建”的缺口。单个写工具通过同一个解析器构建命令，
  因此参数形态只存在于一处。

- **图概览的行为像 IDA：只在显示图时才存在。** 它现在是图模式窗口，而不是始终停靠的窗格：把反汇编视图切换到文本（`Space`）会让它退出布局，从而让 Functions 收回整个左列（不会留下标签条），切换回来时按用户上次选择的高度重新停靠在 Functions 下（默认占该列的 18%，类似 IDA 约 110 px 的概览）。缩略图现在适配整个 CFG —— 小图会被放大而不是被限制在 35%，后者正是“小块周围一大圈白框”外观的来源 —— 绘制普通图色方块、细线边框和按边种类区分的颜色（不再有当前块着色），用虚线矩形标出可见区域（仅当图大于窗格时），并把点击的块居中到图中，而不是跳到文本列表。Views 菜单在图模式之外不再提供该窗格，保存的布局在文本模式下也不再携带它。两个测试覆盖模式驱动的是否存在和点击路径，`docs/DEMO.md` 记录了该仅图模式窗口。

- **AGPL-3.0-only 现在是唯一许可证，贡献使用 DCO 而不是 CLA。** 商业许可轨道已移除（删除了 `LICENSE-COMMERCIAL.md` 和 `CLA.md`），因此贡献者唯一要做的就是 `git commit -s`：贡献按 AGPL-3.0-only 以 inbound = outbound 方式接受，没有版权转让，也没有再许可授权。依赖允许列表不变（GPL-2.0-only 因 AGPL 兼容性继续排除，GPL-3.0-only 则为我们自己代码将来可能的再许可留出空间）。更新了 `scripts/check-license.ps1`、`deny.toml`、`Cargo.toml`、`NOTICE.md`、两个 README、`CONTRIBUTING.md`、`docs/FAQ.md`、`docs/DESIGN.md` §11/§12/§16、`docs/M4.md`、`docs/RELEASE.md`、`docs/DEMO.md`、`docs/README.md` 和 ADR-002/ADR-010，并由 ADR-013 取代 ADR-007。
- **`F5` 像 IDA 一样反编译；分析在导入时运行。** 该键不再启动分析 pass。导入 PE 现在会自动启动后台分析 —— 工具栏按钮、`Options → Analyze functions` 和命令面板命令 `Re-run analysis` 会重新运行它 —— 与 IDA 的自动分析一致，后者也没有专用热键。`F5` 改为显示光标下函数的伪 C：它打开/聚焦反编译窗格，在光标离开上一个函数时重新反编译，并居中光标所在行。滚动请求以地址为键，并在行存在之后解析（`pseudo_scroll_to`），因此重新反编译绝不会滚动到过时的行，并且它会在渲染它的第一帧被消费。十六进制同步和 `F5` 现在共用一条 `cursor_address` 规则（活动的伪 C 行，否则用反汇编选择）。两个测试覆盖按键绑定和居中滚动，一个覆盖导入触发的分析。
- **在选择和复制方面，代码视图像文本编辑器一样工作。** 反汇编列表和反编译视图现在在渲染行上跟踪锚点/焦点对，因此拖动（或 Shift+单击）会精确选中指针下跨行的字符并把它们绘制在字形后面；普通单击把选择折叠为插入符，拖动永不导航。`Ctrl+C`（桌面集成的 `Event::Copy`）通过 `copy_focused` 复制选择 —— 没有拖动时复制当前行 —— `Ctrl+A` 选中整个视图。行文本由 `disasm_columns` / `disasm_line_text` 一次构建，因此命中测试、绘制和复制不会与渲染漂移。十六进制视图选择包含端点的字节范围（拖动、Shift+单击或光标），并可从右键菜单或 `Ctrl+C` 复制为十六进制或 ASCII 转储，其头部显示选中的字节数。停靠列表（Strings、Imports、Exports、Functions、Names、Segments）以地址为键保留选中行，并通过 `Ctrl+C` 或“Copy row”把可见列以制表符分隔复制；`xrefs to …` 对话框用 `Ctrl+C` 复制其选中行。普通标签（AI 面板、Output 日志）内部的文本选择保留 `Ctrl+C`：代码视图只在没有标签文本被选中时才认领该事件。九个新测试覆盖字符计算、分页重映射、Copy/SelectAll 组合键、三个视图各自的拖动、端到端停靠手势、列表行格式和标签选择优先级。
- **If/else 打包能在某个分支无法继续时存活**（`may_post_dominators`）。当穿过分支的一条路径以 `ret`/`int3`/noreturn 调用结束，严格后支配会报告“没有重新汇合”，于是另一分支被发射为提前退出，存活路径跑过了汇合点 —— 把第二分支的 fallthrough 变成 `goto loc_…`。结构化器现在计算第二组后支配链接，其中终止路径不施加约束，并在汇合点是干净菱形时把两个分支都打包在共享汇合点周围（每个前驱都无条件落入它、它留在外层循环内、且两个分支都尚未发射）。Notepad：`sub_140007924` 82 → 63 行，`sub_140007ab4` 113 → 96，`sub_140026438` 152 → 150，`sub_140004264`/`sub_14000468c` 各少一行（清理尾部不再在两个互斥体辅助函数中重复三次）。Go：2,998 → **3,003** 个结构化函数（+5）。crackme（160）、corpus（31）、corpus86（32）和 go（60）没有快照变化；notepad 仍是 487/502，lint 在 notepad（1,004 段文本）和 kernel32（5,134）上保持 0 发现。也能修复剩余多入口形态的更宽汇合链式处理未被采用：它让两个 crackme 函数把一个尾部块重复最多五次，并产生围绕 goto 的 `while (1)`，因此门禁保持保守。
- **字符串提取现在匹配 IDA 的 *Strings* 窗口默认值**（`analyze` 输出不再被指令字节淹没）。字面量必须以 NUL 结尾（`STRTYPE_C` / `STRTYPE_C_16`）、至少 4 个字符，并且可以包含 `ida.cfg` 的 `StrlitChars` 接受的控制字符（`\a\b\t\n\v\f\r`、`ESC`）；已解码函数内部的候选会被跳过（IDA 的 *Ignore instructions/data definitions* 默认关闭），重叠的 C/C16 候选会像 IDA 的步进扫描那样丢弃。Notepad：**3,578 → 778** 个字符串 —— 旧条目中有 1,962 个是 `D$pH` 之类的 `.text` 操作码字节，IDA 一个都不列；corpus：39 → 45（纯标点字符串不再被过滤）。`StringScanOptions` / `extract_strings_with` 保留旧的“任何不可打印字节结束一段”扫描（`nul_terminated: false`、`skip_code: false`）。七个 notepad golden 快照现在渲染 `off_…`，而它们过去显示未终止的 4 字节串（`"ETW0"`）；Go 语料降到 9,099 个字符串，因为 Go 字符串数据没有 NUL 终止符。
- **重命名了可执行文件**：桌面 GUI 现在是 `recraft.exe`（原为 `recraft-ui.exe`），命令行现在是 `recraft-cli.exe`（原为 `recraft.exe`）。只有二进制目标名变了 —— cargo 包仍是 `recraft-ui` / `recraft-cli`，因此 `cargo run -p recraft-ui` / `cargo run -p recraft-cli` 和每个工作区任务继续工作。clap 用法行现在读作 `recraft-cli`，打包脚本、NSIS 快捷方式和卸载图标、`verify-package` / `verify-installer` / demo / e2e / bench 脚本以及参考文档（`recraft <cmd>` 示例变成 `recraft-cli <cmd>`）都已相应更新。
- 反汇编文本视图现在通过伪 C 高亮器使用的同一个 `is_register` 匹配器识别整个寄存器文件（`r8b`…`r15d`、`r8w`、`sil`/`dil`/`bpl`/`spl`、`rip`/`eip`、段寄存器、`ymm`/`zmm`），而不是固定列表。
- `e7da3b6` 上的最终验证：156 个测试、全部五个快照集零不匹配（785 个函数）、四个样本 lint 干净、工作树干净。
- 阶段报告以结论（§8）收尾：什么变得更短、更像 C、更可信、更快，随之增长的护栏，以及剩下的一处已量化结构性缺口（多入口循环，下一步指向 CFG 工作而不是更多文本 pass）。
- 交接的工作列表部分现在在样本旁边携带真实库分布（kernel32 2000 个中的 111 个、user32 159、advapi32 125），因此剩余难度明显是代码形态的属性，而不是所选样本的属性。
- ntdll.dll 和 shell32.dll 在 L1 下也 lint 干净，因此五个系统库（kernel32、user32、advapi32、ntdll、shell32）通过不变量。阶段报告新增 §6.1 记录该验证及其发现的两个缺陷，教训是受控样本返回零发现并不意味着生产代码也如此。
- 对没有已分析函数覆盖的地址的结构化请求现在会以 `invalid argument: no analysed function contains 0x…` 失败，而不是打印一个标题为“L1 unavailable: run analysis first”的扁平列表（分析已经运行过 —— 只是该地址不是函数）。剩余的回退文本说明真实情况：`no function starts here: showing the flat instruction listing`。
- 签名修复后在更多生产代码上验证：user32.dll（3040 个函数）和 advapi32.dll（1861）在 L1 下都 lint 出 **0 条发现**，`metrics` 报告各自前 2000 个函数的结构化率 92.0% / 93.3%、覆盖率 99.3% / 99.5%。连同 kernel32（94.5%，覆盖率 99.1%），反编译器在真实系统库上站得住。
- **修复了在生产代码上测试发现的一个真实输出缺陷**：对 kernel32.dll 的 `lint` 报告 34 个函数体读取 `arg5`/`arg6` 而签名停在 `arg4`。栈参数 pass 只为 `sub_*` 名称加宽签名，且只在 `arg1` 已声明时这样做，因此具名函数（LCMapStringEx、CompareStringEx、CreateProcessAsUserW、…）和 `(void)` thunk 保留了与其函数体不匹配的参数列表 —— 这违反了一条输出不变量。两个条件都已去除；kernel32 的 5134 段文本现在 lint 干净，四个样本和全部 785 个快照不变。
- 在真实 kernel32.dll（862 KB）上演练了无项目路径：`info` 列出 8 个节、107 个导入模块和 1292 个导入函数，`eval` 报告相对于入口 + 代码导出 + `.pdata` 的召回率 1.000、精确率 0.942（2419 个真值、2567 个检出、148 个未匹配调用目标）。阶段工作没有干扰发现。
- 也抽样了 Go 的单跳回退（cmpbody、maps.(*Map).getWithoutKey、runtime.mapaccess2、strconv.FormatInt）：每个都向后跳到后面跟着循环体语句的标签，因此单跳切片在那个样本上也不是容易的收益。
- 在符号扩展提升之后重新测量了 Go 样本：内联汇编 1905 → **1899**，直方图为 `0:2998 1:60 2:38 3:31 4:27 … 45:1`，因此 263 个回退中有 60 个只带一个跳转 —— 这是该样本最可行动的一片。文档已更新，直方图现在记录在 `QUALITY.md` 的路线图笔记旁。
- 修正了 crackme 的内联汇编计数：`metrics` 报告 **265** 而不是 266 —— 符号扩展提升在那里比位扫描多拿掉一条语句，而文档停在了较早的数字。两个表和两处散文提及现在都说 265。
- 路线图的质量行现在在 corpus 和 Notepad 数字旁边引用 crackme（91.9%）和 Go（3261 中的 91.9%，覆盖率 98.8%），并说明快照集为 785、Notepad 已完整。
- 三份活动文档仍写着 453 个快照（交接的目录列表和关于变动的警告，以及质量页的目录摘要）；现在它们写 785，与集合一致。`PERF.md` 中记录的测量保留原始数字，因为它们描述的是一次集合还更小的时候所做的运行。
- 两个指令提升有编码器级测试：`bsf eax, ecx` 变成 `UnOp::Ctz`，`popcnt eax, ecx` 变成 `UnOp::Popcount`（两者都通过真实操作码字节检查），`cdq` 用条件为 `SignedLt` 的 `Select` 写入 `edx`。IR crate 测试 24 → 26。
- 检查了流水线的确定性：把 corpus.exe `import` + `analyze` 到一个新项目，产生的指标与已提交样本完全相同（31 个函数、391 条指令、97.4% 覆盖率、`0:31`），其 L1 文本与已提交快照匹配，因此项目的文本取决于二进制和代码，而不是测量的顺序。
- `docs/QUALITY.md` 中的命令速查列出了本阶段新增的标志：`metrics --list`、`metrics --max-functions 0` 和 `export-pseudo --only-unstructured`，与现有的 lint、gate 和快照命令并列。
- 修复了两个从来不是真正标题的标题：`NEXT.md` §1.2（goto 进入 `do` 的形态）被留成单个 `#`，§1.4（局部变量重建）被粘到 §1.3 末尾。文档现在列出全部七节，扫描也不会再发现使用单个 `#` 的编号标题或嵌在散文中的标题。
- 六个被脚本粘到段落末尾的章节标题重新独立成行（`NEXT.md` 的第 1.3、2、3 节，较早报告的 §6，以及本阶段报告的输出构成一节）。现在扫描嵌在散文中的标题只会匹配那两条故意讨论标题的 changelog 条目。
- 报告的变换表现在把它新增的两个指令提升（位扫描和符号扩展）与四个文本 pass 并列，因此 §1 列出了本阶段改动的全部内容。
- 对照最终测量校对阶段报告并修正了漂移之处：快照数量（453 到 785，并注明 Notepad 现已完整）、内联汇编列（Notepad 134、crackme 266、Go 1905）、Go 覆盖率（98.8%）和参考函数的耗时（1.1 s 到 0.56 s，文本 pass 255 ms 到 67 ms）。
- 报告和质量页再次携带套件的当前规模（154）。
- 一个测试针对完全结构化的样本（x64 语料）钉住 `--only-unstructured`：它不写任何函数文件，只写索引头，README 也展示了该标志。套件为 154 个测试。
- `export-pseudo` 新增 `--only-unstructured`，只写出仍带 fallback 跳转的函数（crackme：493 个文件中的 40 个），因此审阅困难案例不必滚过结构化的大多数。
- 阶段报告和质量页携带当前测试数（153），而不是 149/150。
- 一个测试覆盖直方图：各桶之和等于函数数，且“0 跳转”桶等于结构化计数。套件为 153 个测试。
- 检查了 crackme 中全部八个单跳转函数：每一个都向后跳到后面跟着真实语句的标签（`continue` 条件、未建模标志分支或簿记），因此它们是同样的双入口形态 —— 没有一个是“跳到函数尾部 return”的简单情形。
- `metrics` 打印一行 `goto histogram`：有多少函数携带 0、1、2 … 个 fallback 跳转。对 Notepad 它读作 `0:487 1:1 2:3 3:1 4:3 9:3 10:2 14:1 369:1`，对 crackme 读作 `0:453 1:8 2:7 … 92:1`，一眼就能看出尾部是几个大函数加上一条由一两跳函数组成的长尾 —— 与导出索引按文件给出的图景相同。
- 最后一天的提交卫生检查：83 个提交，没有空的，没有短于 15 个字符的，没有标为 wip/temp/fixup 的，且每个都带有解释改动的正文。
- 阶段报告以交付索引（§7）结尾：哪份文档回答哪个问题（报告、交接、不变量、性能及其排除方向、FAQ、changelog、代码入口点、回归网）外加四命令验证路径。
- 逐字走了一遍 `REPORT-2026-10-31.md` 中的维护者清单：fmt、clippy、工作区测试、`lint`、五项快照检查和 `metrics` 都按写法运行，并且修正了说 `sub_140004264` 在快照集之外的注释 —— 该集合现在覆盖全部 502 个函数。
- 为记录逐 crate 运行了测试套件：UI 73、decompile 152、IR 24、core 17、analysis 16（加上更小的 crate），全部通过 —— 反编译器工作没有落下任何 crate。
- 估算了局部变量这块收益的规模：2 497 次 `var_XX = …;` 写入，但只有 339 个槽位被写入不止一次，因此约 2 158 个槽位是单次写入。`docs/NEXT.md` §1.4 记录道，把它们变成 `int32 v18 = …;` 声明是产品决策而非纯技术决策 —— 它会用 `var_18` 与栈偏移之间 1:1 的映射换取更像 C 的外观 —— 因此留给人来选择，而不是在阶段末单方面改动。
- 拆分了输出构成中“其他”的 12.2%：2 352 行是 `x = f(y);` 赋值，743 行是内存存储，517 行是函数签名 —— 没有无法归类的内容。
- 实现并回退了一个寄存器复制传播 pass。规则本身是合理的 —— 在一个窗口内，`src` 既未被写入也未被破坏、`dst` 被覆盖，就把 `dst` 的读者重命名为 `src` 并删除复制 —— 但它在 502 个 Notepad 函数（29 664 行，1 508 次复制，输出不变）中触发了**零**次。这些复制是真实的数据移动：其目标通常再也不会被写入，而源会被立即复用，因此窗口在读者之前就关闭了。这确定了一点：移除它们的是局部变量重建，而不是复制传播。
- 又两个只由 CLI 演练过的行为测试：`measure(session, 0)` 证明零上限意味着“所有函数”（并且 `skipped` 统计其余部分），对空目录运行 `check_golden_limited` 证明每个函数都被报告为缺失 —— 这正是“no golden snapshots in <dir>”消息背后的情形。套件为 152 个测试。
- 测量了“看起来像汇编”这种感觉背后的寄存器到寄存器复制：29 664 行输出中有 1 508 行是普通复制，其中只有 126 行是死写入 —— 另外 1 382 行携带真实数据流。因此下一个大特性是局部变量重建，而不是文本清理；`docs/NEXT.md` §1.4 记录了这块收益、三个步骤以及为什么它需要自己的阶段。
- 测量了全部 502 个 Notepad 函数的 L2 输出构成（29 664 行）：赋值 44.0%、调用 12.4%、花括号/空白 12.4%、其他 12.2%、控制流 11.2%、返回 3.0%、头部 2.8%、标签 0.9%、`goto loc_` 0.6%、内联汇编 0.5%。非结构化残留（标签加跳转加 asm）不到文本的 2%，这让剩余工作有了参照。
- 尝试把已提交的语料快照接入 `cargo test`（这样结构化改动就不会悄悄落地），但放弃了这个想法：测试内构建的会话对 31 个函数之一渲染出的结果与拍摄快照的项目不同，因此测试会因与反编译器无关的原因失败。`recraft golden --check` 仍是运行它们的方式。
- 扫描了每个导出的 Notepad 函数的渲染缺陷（双重否定、`!!`、自比较、条件内赋值、转换的转换、负下标、`!= 0 ? -1 : 0`）：一个都没有出现。确实出现的两个模式是有意为之 —— `!= 0) ? -1 : 0` 是 `neg`/`sbb` 惯用法设置掩码的方式，`sub_X();` 是参数在另一个基本块中设置的调用（已记录的行为）。
- 仓库状态检查：455 个提交、931 个受跟踪文件、`tests/golden` 1.36 MB、工作树干净；完整验证在全部五个集合（785 个快照）、四个样本的 lint 和 150 个测试上通过。
- FAQ 对“输出有多好”的回答现在引用全部四个样本（corpus 100%、Notepad 97.0%、crackme 91.9%、Go 3261 中的 91.9% 且覆盖率 98.8%），而不只是前两个，因此 Go 二进制的真实数字在用户最先看的地方可见。
- 在位扫描和符号扩展提升之后重新跑了两次完整 L2 lint 扫描：Notepad 502/502 和 Go 3261/3261 段文本，各 0 条发现。不变量在最大样本的两个层级上都成立。
- 测量了增长到 502 个 Notepad 函数之后的快照套件：调试构建下 `golden --check` 耗时 12.7 s，主要由 576 KB 的参考函数占据；其余 501 个合计约 2 s，Go 的 60 快照检查为 0.6 s。记录在 `docs/PERF.md`。
- 主样本的快照覆盖现已完整：`tests/golden/notepad` 保存全部 502 个函数（1.1 MB；仅 45 KB 的参考函数就有 576 KB），而不是按地址的前 170 个。较早的集合停在 `sub_140004acc`，因此项目中最复杂的函数没有快照 —— 这正是 `cdq` 提升改动了三个函数却没有任何快照察觉的原因。总共 785 个快照；五个集合仍然全部匹配。
- 对前一条的更正：`cdq` 提升**没有**坏掉。比较有无该提升的 L2 导出显示三个受影响函数，唯一变化是 `__asm("cdq")` 变成 `edx = ((eax < 0) ? -1 : 0)`，其中一处并入后续掩码为 `edx = (((eax < 0) ? -1 : 0) & 3)`。先前的笔记把禁止单字符推测误解为丢弃了赋值。该提升已加入：`UnOp` 保留 `Ctz`/`Bsr`/`Popcount`，`cdq`/`cqo`/`cwd` 复用 `Expr::Select`，使 Notepad 的内联汇编从 138 降到 134。
- 尝试把 `cdq`/`cqo`/`cwd` 提升为 `edx = (eax < 0) ? -1 : 0` 并回退了：紧随其后的原始 `__asm("idiv …")` 被视为破坏所有寄存器，于是死写 pass 删除了它刚产生的赋值，打印出的程序丢失了被除数的高半部。该风险在改动之前就存在（`cdq` 内建指令无法被丢弃），而提升这些指令首先需要原始 asm 语句不再被视为无条件写入。`docs/QUALITY.md` 把它记录在 asm 审计旁边。
- 在名称包含 `/`、`.`、`(` 和 `*` 的 Go 符号上检查了 `export-pseudo`：文件输出为 `internal_abi_BoundsDecode.c` 和 `internal_abi___MapType__HashMightPanic.c`，即主干清理器让每个名称都是合法的 Windows 文件名，目录也仍然得到其索引。
- 阶段报告的验证章节写明了当前提交，并在否定结果旁边提到位扫描提升，因此收尾摘要反映了实际交付的内容。
- `docs/QUALITY.md` 审计了 Notepad 仍有的 138 条内联汇编语句：30 个 `lock` 原子操作、21 个依赖标志的 `if (…) goto …`、21 个单操作数 `imul`（64 位 `edx:eax` 乘积，用 C 写需要更多行）、12 个 `div`/`mul`，以及 44 个系统或隐式寄存器指令。该笔记记录了为什么位扫描是唯一值得提升的类别，其余则不是。
- 基线表跟随位扫描提升：crackme 的内联汇编从 277 降到 266，Go 从 1959 降到 1905，覆盖率从 98.7% 升到 98.8%。Notepad 不变（其快照中没有位扫描）。
- `bsf`、`bsr` 和 `popcnt` 被提升而不是留作文本。`UnOp` 新增 `Ctz`、`Bsr` 和 `Popcount`；`bsf` 打印 `ctz(x)`，`bsr` 打印 `(0x1f - clz(x))`（精确恒等式，因为 `bsr` 返回最高置位的索引），`popcnt` 打印 `popcnt(x)`。crackme 的 SIMD 字符串扫描少了 11 行 `__asm`（277 → 266），并把原本的 `__asm("bsf ecx, eax"); eax = ecx;` 读作 `ecx = ctz(eax);`。一个快照（`crackme/sub_140006fd0.c`）发生变化并经过审阅：正好是那两行位扫描，没有别的。
- `docs/NEXT.md` §1.3 记录了下一个有界特性：`bsf`/`bsr`/`tzcnt`/`lzcnt`/`popcnt` 仍打印为 `__asm`（例如 crackme 的 SIMD 代码中的 `__asm("bsf ecx, eax"); eax = ecx;`），对它们建模需要新的 `Expr::Call` 变体，每个 `match Expr` 位置都必须处理。该笔记列出了四个步骤以及为什么快照变动使它成为一项独立且更大的工作。
- 检查了最少演练的路径，即 L2 下的 32 位样本：`lint --level l2` 对全部 32 个函数报告 0 条发现，文本显示出预期的 32 位形态（`__cdecl`、参数写作 `[esp+0x4]`、被调用者保存的溢出为 `var_c = ebx;`，以及起始于 `[esp+0xc]` 的栈参数绑定为 `arg2`）。
- 阶段报告新增提交构成表（§5.2）：目前 116 个提交，其中 72 个是文档。文档占比高是有意为之 —— 否定结果和按指标划分的优先发现最能节省后续会话的时间。
- 两份阶段报告都去掉了过时数字：较新的一份说 196 个 Go 回退（现在是 263，并点名了运行时内部函数）和 148 个测试（现在是 150），较旧的一份也说 196（现在是 263，并注明 196 来自 2000 个函数的样本），验证标题写明了当前提交。
- 也检查了 crackme 的回退：40 个中有 15 个有一到三个跳转，第一个检查的（`sub_140002354`，单个 `goto`）与 Notepad 中相同的双入口形态 —— 标签位于 `if` 分支内部，而跳转来自兄弟 `else`，因此“先修小的”计划在这个样本上也没有变容易。
- `e6274b9` 上的完整回归：150 个测试、四个样本（notepad、crackme、corpus、corpus86）的 `lint` 干净、全部五个快照集零不匹配（453 个函数）、fmt/clippy 干净。
- `docs/QUALITY.md` 解释了为什么“只剩一个跳转”的函数也无法用文本修复：在 `sub_14000da7c`（703 行，单个 `goto`）中，目标是某个 `if` 分支内共享的 `CoTaskMemFree` 尾部，而跳转来自另一个分支深处，因此内联尾部仍会留下跳到原路径继续处的跳转。14 个小的回退和 369 跳的离群值是不同规模下的同一问题。还把 Go 行修正为 2998/3261。
- `docs/NEXT.md` 记录了 `export-pseudo` 索引显而易见的事实：Notepad 的 15 个非结构化函数中，14 个只有 1–14 个跳转，一个却有 369 个，因此两个指标彼此分离 —— 只修那 14 个小的就能把结构化率从 487/502 提到 501/502（99.8%），而跳转总数需要那个离群值。该笔记把只有一两个跳转的四个函数列为最便宜的起点。
- `export-pseudo` 还会写出 `index.tsv`（`file`、`address`、`gotos`、`lines`），因此包含数百个文件的目录可以被分诊：按 `gotos` 排序会把离结构化控制流最远的函数排在前面。导出测试会检查行数。
- 一个语料测试覆盖 `export_pseudocode`：它把整个 x64 样本导出到临时目录，断言文件数与返回值匹配、每个文件都带有 L2 头部（因此写出的是可读渲染），并在之后删除该目录。现在有 150 个测试。
- 两个 README 都在命令块中展示 `export-pseudo notepad.rcraft --dir pseudo-out`，与它所补充的 `golden --check` 示例并列（快照用于回归，导出文件用于阅读）。
- 新增 `export-pseudo <project> --dir <dir> [--limit N]`：为每个函数写一个 L2 伪 C 文件，使用与 golden 快照相同的文件名（因此 `dir/sub_140001480.c` 与 `tests/golden/.../sub_140001480.c` 对应）。语料导出 31 个文件，其内容是可读的 L2 文本，而不是快照存储的规范化 L1 形式。
- `decompile` 新增 `--out <FILE>`，把渲染后的伪 C 写入文件（同样遵循 `--addresses`），并在 stderr 打印 `wrote N lines to <path>`。坏的路径会报告 `invalid argument: cannot write <path>: …`。渲染选项移入 `DecompileOptions` 结构体，使函数保持在 clippy 的参数上限之内。
- `docs/PERF.md` 再次一致：阶段中期的“1.0 s”数字指向较新的 556 ms 测量，整样本笔记说离群值约 0.6 s，而一个被插入操作粘到上一段的 `### 反例` 标题重新独立成行。
- 在阶段末重新测量了 45 KB 函数：各阶段合计约 **556 ms**（first lift 122、alias/arg 38、seeded lift 76、emit 10、text passes 67、naming 15），低于阶段中期记录的 1.0 s，`docs/PERF.md` 携带该表。
- `fold_constant_registers` 的文档注释不再包含两段无关内容：`GetClientRect`/`RECT` 那句话移到了 `name_known_struct_fields`（它所描述的代码所在处），常量折叠的注释则说明它也会折叠进复合赋值。
- 在临时目录上从零走了一遍快照工作流：`golden --dir <new>` 写出 31 个文件，`--check` 报告 0 不匹配，而往一个快照追加一行后 `--check` 打印 `mismatched: 1`、`diff: add_leaf.c` 并以 1 退出。写入、检查和漂移检测都按文档工作。
- `decompile --json` 在行数旁边携带 `"gotos"` 和 `"structured"`，与 L2 头部打印的两个事实相同，因此调用者无需重新解析文本就能按函数离结构化控制流的距离排序。
- 修复了 `docs/NEXT.md` 中四个被辅助脚本截成单个 `*` 的粗体标记（§1 量化, 该函数内部的热点, 最纯粹的一例, 规模实测）以及 `REPORT-2026-10-31.md` 中的一个；现在扫描所有文档都找不到以孤立 `*` 开头的行。
- 交接笔记修复了两个被脚本截断的粗体标记，并补充了回退实际集中的位置（用 `metrics --list` 测量）：Go 的 263 个几乎都是运行时内部函数（cpu.processOptions 29、maps.(*Iter).Next 22、pruneTombstones 21、五个 strconv 辅助函数、五个 mapassign 变体），它们是手写汇编或锁重试代码而非普通 Go；crackme 的是两个函数（92 和 62 个跳转）；Notepad 的基本上是一个（369）。
- `recraft --help` 以简短指南开头：`.rcraft` 项目是什么、`import` → `analyze` → `decompile --l2 --addr` 序列、`--addr` 接受什么，以及 `metrics`/`lint`/`golden` 无需 GUI 就能评判输出。
- `recraft --help` 提到工作列表：`metrics` 那行现在读作“……；`--list` 会列出结构化器无法归约的函数”。
- `import` 会说出它无法读取的文件：`invalid argument: cannot read <path>: <os error>`。裸 I/O 消息没有说明哪个路径缺失，而且会被操作系统本地化。
- 又有两条面向用户的消息说明实际出了什么问题：坏的 `--addr` 值读作 `error: invalid argument: invalid address or unknown symbol: …`，而对没有任何快照的目录执行 `golden --check` 会显示 `no golden snapshots in <dir> (run without --check to write them)`，而不是把 502 个“缺失”报告为回归。
- 用户输入失败不再把自己报告为内部错误：`Error` 新增 `InvalidArgument` 变体（`invalid argument: …`），用于“nothing to decompile at 0x…”、“no functions: run `analyze` before linting”和“no texts checked: raise --limit above 0”。输错地址现在读作 `error: invalid argument: nothing to decompile at 0x0`，而不是 `error: internal error: …` —— 后者暗示工具里有 bug。
- 阶段报告中 Go 的基线行现在使用完整样本（2998/3261 结构化，91.9%，263 个回退，1446 个标签，1959 条 asm），而不是截到 2000 个函数的数字，测试数写为 149。
- `docs/PERF.md` 记录了完整测量的代价：对 Go 样本（两个层级共 3261 个函数，调试构建）运行 `metrics --max-functions 0` 约需 11–12 分钟，这就是默认上限为 2000 的原因；语料则以秒计。
- `docs/NEXT.md` 携带完整样本的 Go 数字并将其用作验收基线：剩余工作是 Notepad 中 15 个、crackme 中 40 个、Go 中 **263** 个（共 3261 个）非结构化函数，`metrics --list` 是枚举它们的方式。
- Go 基线现在是完整样本：`metrics --max-functions 0` 测量全部 **3261** 个函数并报告 **2998 个结构化（91.9%）**、263 个回退函数、98.7% 覆盖率、1446 个标签和 1959 条内联汇编语句。表中此前是 2000 个函数的样本（90.2%，196 个回退，1059 个标签，1343 条 asm），既低估了比率也低估了剩余工作。
- 基线表的 Go 行曾在 220 个函数的样本上测量；用 `import`/`analyze` 新建的项目显示 `metrics` 今天实际覆盖的范围（默认 2000 个函数上限，跳过 1261 个）：**2000 个中 1804 个结构化（90.2%）**，同样的 196 个回退、98.7% 覆盖率、1059 个标签和 1343 条内联汇编语句。表中现在这样写。
- 为 `docs/PERF.md` 测量了整个样本的吞吐：以 L2 反编译全部 502 个 Notepad 函数（每个一个进程，含启动和打开项目）耗时 10.9 s，即平均每函数 22 ms，45 KB 的离群值约 1.0 s，其余 501 个平均约 20 ms。
- 文本 pass 计时的第三个否定结果：在临时值计数器中跳过字面量 token 后 `folds` 不变（51.3 ms 对 50.5），因此开销就是逐行 token 化本身，只有合并 pass 才能改变它。已回退；`docs/PERF.md` 列出了全部三个。
- 验证了两个 README 中打印的 `import` 形式（`import <exe> -o <project>`）仍然可用：位置参数文件加 `-o` 产生了与 `--out` 变体相同的项目，因此无需修改文档。
- 在全新项目上端到端走了一遍文档中的快速开始（`import --out`、`analyze`、`decompile --addr --l2`、`lint`、`metrics --list`）：每一步都成功，语料结果为 31/31 结构化且 lint 0 发现，`--list` 报告没有非结构化函数。文档中的示例此前只通过预构建项目演练过。
- `docs/QUALITY.md` 中的基线表写明了测量时间和地点（2026-11-02，提交 `9ae2537`），因此读者能判断这些数字有多旧。
- goto 进入 `do` 函数体的形态已测量并降级：Notepad 只有 **1** 个实例（`0x0140007570` 第 88 行），而“向后跳入 `if` 分支”的形态有 **161** 个，因此 `docs/NEXT.md` 告诉后续会话先做 §1，把这个留作长尾清理，而不是为它构建通用机制。
- `docs/NEXT.md` §1.2 记录了阅读 Go 输出时发现的第二种多入口形态：`goto loc_X;` 紧挨在函数体包含 `loc_X:` 的 `do {` 之前，即进入循环中部。该笔记给出了具体的节点分裂配方（复制序言和循环体，受大小上限约束）、语义论证和验证顺序。
- 对照当前指标修正了 `docs/QUALITY.md` 中两个过时数字：crackme 的标签和内联汇编计数是 218 和 277（表中仍是 155 和 275），快照摘要写的是 go 60 而不是 40。
- 修复了五个被脚本降级为 `#` 的标题（它们本应是 `##`）：`NEXT.md` §0/§1.1/§2.1、`REPORT-2026-10-31.md` §5.1 和 `REPORT-2026-11-02.md` §5.1，最后一个现在写明了当前提交。
- 两个 README 都在命令块中展示 `metrics notepad.rcraft --list`，因此非结构化函数的工作列表可以从首页发现。
- `docs/NEXT.md` 中的参考案例再次携带其当前行号（`loc_140017355` 在第 1212 行，9857 行函数体中的 15 个向后跳转位于 7778–7874），并说明它们会随文本 pass 移动，因此读者不必寻找过时的位置。
- 在第二阶段所有文本改动之后，重新以 L1 跑了完整的 Go 扫描：**3261** 个函数，0 条发现，因此在最大样本上，嵌套 if 折叠、累加合并、窄/复合常量折叠和基本块边界修复到位后，不变量仍然成立。
- `docs/NEXT.md` 开头说明第二阶段已经完成的内容（嵌套 if 折叠、累加合并、窄/复合常量折叠、基本块边界修复、命名 pass 加速和新的 `metrics --list`），因此交接不再把它们列为未完成工作；随后是当前基线和未变的多入口章节。
- 阶段报告记录了 `cf6a8f9` 上的阶段末验证：148 个测试、四个样本 lint 干净、全部 453 个快照匹配、fmt/clippy/build 干净。
- `docs/PERF.md` 中第二个测得的否定结果：在 `strip_condition_parentheses` 中跳过逐行文本克隆后 `ifs` 组不变（52 ms，在 47–50 的噪声范围内），因此该开销就是四次完整扫描本身。已回退，结论是只有把相邻文本 pass 合并成一次遍历才能改变它。
- 性能工作的一项测得否定结果记录在 `docs/PERF.md`：把两次 `if` 合并内部的整体花括号重算替换为局部数组编辑，输出逐字节相同但没有改变 `ifs` 组（47.1 → 47.2 ms），因此那段时间花在三个条件 pass 中构建新字符串上。该改动因没有收益而被回退，而不是保留。
- `docs/PERF.md` 携带本轮工作之后 45 KB 函数的完整阶段分解（cfg 0、first lift 125、alias/arg 37、seeded lift 77、emit 10、text passes 约 260 按组划分、post passes 约 34、naming 15、墙钟约 1.0 s），以及三个从“每个候选项一次”改为“函数体一次”的 pass，并点名了下一个热点。
- `RECRAFT_PROFILE=1` 现在拆分此前缺失的阶段，因此性能剖析覆盖整个流水线，而不是停在文本 pass。对于 45 KB 函数：`cfg` 0 ms、`first lift` 125、`alias/arg analysis` 37、`seeded lift` 78、`emit` 10、**`text passes` 258**、**`post passes` 117**、**`naming` 48**。最后两个此前不可见，也是剩余文本工作所在。
- `docs/NEXT.md` 指向 `recraft metrics <project> --list` 作为结构工作的第一步，并给出它今天打印的数字（Notepad：15 个函数，最大的 369 个跳转、9868 行；crackme：40 个，最大 92）。
- JSON metrics 报告在 `"unstructured"` 下携带同一列表（起始地址、名称、goto 计数、行数），因此仪表盘无需解析文本就能绘制非结构化函数体。
- `recraft metrics --list` 打印结构化器无法归约的函数，最差的在前（`369 goto  9868 lines  0x140013154  sub_140013154`），这是多入口循环工作的工作列表，也是 `N goto` 头部的逐函数版本。Notepad：15 个函数，crackme：40 个，最大的是 `sub_140006fd0`，有 92 个跳转。
- `docs/REPORT-2026-11-02.md` 是第二次实现类源码输出阶段的报告：四个文本变换和正确性修复各自依赖的安全规则、五个样本的当前基线、完整的双层 lint 扫描、148 测试 / 453 快照的回归网、四个剩余限制和复现命令。
- 不变量的全量扫描证据：`recraft lint .tmp/go.rcraft --level l2` 覆盖全部 **3261** 个 Go 函数并报告 0 条发现，因此六条输出不变量现在在两个层级上对每个样本都成立（Notepad、crackme 和两个语料此前已在 L1 + L2 扫描过，Go 更早时在 L1）。
- 累加合并有一个针对基本块边界的防御性测试：两条相邻语句中若第一条属于嵌套基本块，则不得合并，因为它产生的值不是第二条累加的那个。
- **修复了常量折叠中的一个正确性 bug**：单次使用常量会被推入该寄存器之后的第一次提及，即使那次提及位于循环内部、值是循环携带的。`eax = 0; do { edx += eax; eax += 2; } while (…)` 打印了 `edx += 0` 并丢掉了 `eax = 0;`。现在折叠要求两行在同一基本块内，且复合赋值（`eax += 2`）算作读取而非覆盖，因此仍然活跃的常量会保留。常量折叠和累加合并之后又各运行两轮迭代，找回的行数多于更严格规则放弃的（notepad L2 29 651 → 29 667：更长是因为错误的死代码删除没有了）。基本块深度已预计算，因此 45 KB 函数仍只需约 255 ms 的文本 pass 而不是 1.7 s。更新 8 个快照；一个回归测试钉住循环携带的情形。
- 累加会合并进供给它的赋值：`rax = &var_38; rax += 8;` 变成 `rax = (&var_38 + 8);`，加载和位测试同理（`rcx = *(u64*)(rax); rcx |= 1;` → `rcx = (*(u64*)(rax) | 1);`）。守卫条件：同一目标、常量、值无副作用，且没有收窄转换（寄存器加法是 64 位）。Notepad：L2 29 921 → **29 651** 行；更新 67 个快照。
- 单次使用常量也会折叠进复合赋值：`rdx = 0x140000000; rcx += rdx;` 读作 `rcx += 0x140000000;`。只有 `+=`、`-=`、`&=`、`|=`、`^=`、`*=`、`<<=`、`>>=`、`/=`、`%=` 的右侧是折叠点，且操作数必须等于该寄存器，因此不会被截断。Notepad：L2 29 938 → **29 921** 行；更新 8 个快照。
- 折叠进比较的常量现在也能匹配寄存器的 16 位和 8 位名称：`eax = 0x5a4d; if (word_140000000 != ax)` 曾变成带原始槽位的三条语句，现在读作 `if (word_140000000 != 0x5a4d)`。常量会被截断到比较宽度，放不下时拒绝折叠，因此宽值绝不会被悄悄重写。Notepad：L2 29 959 → **29 938** 行；更新 7 个快照，全是这个折叠。
- 嵌套 `if` 折叠有一个不动点栅栏：一个测试读取每个 `tests/golden/corpus` 快照，对其运行该 pass 并断言文本不变，因此该 pass 被证实在真实输出上已经饱和，任何将来留下可折叠嵌套的编辑都会让测试失败。
- 嵌套的单分支 `if` 折叠成一个条件：只要两个连续测试共享汇合点，结构化器就会发射 `if (a) { if (b) { … } }`，但源码只有一个条件。新 pass 写出 `if (a && b)` 并把函数体提升一级，仅当内层 `if` 就是整个外层函数体且任一侧都没有 `else`；带顶层 `||` 的一侧会加括号，两个完全相同的条件会合并为一。Notepad：L2 30 102 → **29 959** 行，合并 68 个条件；goto/标签计数不变。刷新 25 个 golden 快照（合并 + 重新缩进）。
- 第三个也是决定性的否定结果：Notepad 中没有任何向后 `goto` 的源和目标在同一基本块内，即每个剩余跳转至少跨越一对 `{}`。因此对文本层而言该形态已被穷尽，只有 CFG 工作能移除其余部分；`docs/NEXT.md` 这样写了，因此不再为它尝试新的文本 pass。
- 第二个否定结果记录在案：Notepad 的 452 个 `goto` 没有一个以所在循环的头部为目标（伪装的 `continue`），只有 4 个以循环之后的语句为目标（`break`），因此把 goto 改写成 `continue`/`break` 几乎不会带来收益 —— 剩余的跳转是需要结构工作的跨层级跳转。
- 测量结果记录在 `docs/NEXT.md`，以免后续会话实现一个毫无收益的 pass：任何样本中都**没有**落空式 `goto`（目标就是下一行的跳转），而确实存在的退化跳转已由 `fold_jumps_over_braces` 处理。
- L2 头部计数与 `metrics` 做了交叉核对：Notepad 有 15 个带 `N goto` 头部的函数，metrics 报告 15 个回退函数；crackme 是 40 和 40。因此该头部可以直接用作非结构化函数体的列表。
- L2 头部报告函数剩余的跳转：`// L2 structure recovery (40 blocks, 2 goto)`，而不只是基本块计数，因此读者一眼就能看到哪些函数体无法归约为结构化控制流。附加的计数在文本 pass 之后计算，没有跳转的函数保持普通头部。
- 文档一致性检查：`NEXT.md` 仍写着 223 个快照，`QUALITY.md` 仍把 crackme 的集合描述为 60 个函数，且 `QUALITY.md`/`REPORT` 中的示例命令仍使用旧的 `--limit` 值。现在都与已提交集合一致（notepad 170、crackme 160、corpus 31、corpus86 32、go 60 = 453）。
- `docs/NEXT.md` 说明了多入口修复的预期规模：Notepad 的 452 行 `goto loc_` 中，161 个是工作针对的“向后跳入 `if` 分支”，因此折叠它们并让 `tidy_labels` 丢弃标签会把计数降到约 **291**（−36%），标签数从 267 降到约 106。
- 报告的验证章节写明了本阶段的收尾提交（`c009fb4`）：五个集合共 453 个快照函数零不匹配，Notepad、crackme 和两个语料 lint 干净。
- `tests/golden/notepad` 覆盖 170 个函数而不是 40 个，从而纳入 `sub_140004264` —— 多入口循环形态的 Notepad 参考 —— 因此两个参考函数现在都有快照和 lint 保护。Golden 总数 **453** 个函数（notepad 170、crackme 160、corpus 31、corpus86 32、go 60）。
- `tests/golden/crackme` 覆盖 160 个函数而不是 60 个，从而纳入 `sub_140006fd0` —— 其跳转表分支在第 47 轮之前引用了 180 个未定义标签的函数 —— 因此该修复现在既有快照也有 lint 规则。Golden 总数 **323** 个函数（notepad 40、crackme 160、corpus 31、corpus86 32、go 60）。
- `docs/NEXT.md` 记录了第 47 轮修复的回归覆盖：`sub_140006fd0`（带 180 个未定义标签的 `case N: goto loc_X` 形态）是 crackme 的第 152 个函数，因此它被 lint 覆盖但在 60 个快照样本之外；把 `--limit` 提到 160 就能为它建快照。
- `docs/NEXT.md` 新增多入口工作的实现地图：循环在哪里决定（`compute_loops` → `loop_blocks`）、在哪里发射（`emit_loop`、`emit_do_while`、约第 2800 行的 `while (1)` 回退），以及为什么额外入口会让 `loop_blocks` 不完整；该笔记把守卫提升放在 `compute_loops` 之后、发射之前，因此现有循环发射器会再次产生 `while`。
- `docs/NEXT.md` 携带该形态的最小复现：`loc_140017355` 是一行 `if` 分支的第一条语句，从下方很远处的 15 个位置跳回，因此跳转落在一个早已闭合的分支内部 —— 该区域不平衡，任何文本 pass 都无法折叠它；这是守卫提升工作的最小案例。
- `docs/NEXT.md` 进一步收窄目标：在 `sub_140013154` 中，三个最热的向后标签（`loc_14001de8b` 16 个跳转、`loc_140017355` 15 个、`loc_140015c81` 8 个）占其 147 个向后跳转中的 39 个，且每个都只被向后跳转引用，因此只折叠这三个就能把该函数的 `goto` 计数减少约 40%。
- `docs/NEXT.md` 量化了多入口形态：Notepad 回退函数的 199 个向后跳转中，**161** 个以 `if`/`else` 分支内部的标签为目标（其中 117 个在同一个 45 KB 函数中），因此守卫提升工作有一个集中的地方先证明自己。
- `docs/NEXT.md` 记录了交接：剩下的唯一结构性缺口（多入口循环）、其两个参考函数及各自拥有的保护、从最小到最大的三条实现路径（守卫提升 → 节点分裂 → 保留 `goto`）、以 `metrics`/`lint`/golden 表述的验收标准、次要候选以及两件不要做的事。
- Go 快照集覆盖 60 个函数而不是 40 个，其中包括 `internal/abi.Name.ReadVarint` —— 第 41 轮标签放置修复的参考案例 —— 因此该形态现在除 lint 之外还有快照保护。Golden：notepad 40、crackme 60、corpus 31、corpus86 32、go 60（**223** 个函数）。
- `docs/REPORT-2026-10-31.md` 新增维护者速查：改动反编译器之后要运行的准确命令序列（fmt、clippy、测试、lint、五项 golden 检查、metrics）、阅读结果的顺序（lint → golden 差异 → metrics）以及剩余工作所在（多入口循环，附带两个参考函数）。
- 验证了 `recraft-ui` 仍能针对当前反编译器和 CLI API 构建（`cargo build -p recraft-ui --target-dir target/ui-check`，使用单独的目标目录，因为运行中的实例会锁住默认目录）。
- JSON lint 报告会标明它检查的层级（`"level":"L1 + L2"`），与文本摘要一致。
- `recraft lint` 按稳定顺序（按地址）打印其发现，因此被截断的报告总是显示相同的首批问题。`--limit 0` 守卫和这个排序都位于 lint 函数中（两者此前都是被一次过宽的文本替换插到 golden 检查旁边的；守卫在那里是重复的，而不是被移除）。
- 报告的验证说明写明 **141** 个反编译器测试（其中三个为 x64 和 x86 语料断言输出不变量），而不是撰写时的 138。
- 32 位语料在 `cargo test` 下也是 lint 干净的（`the_32bit_corpus_passes_every_output_invariant`）；测试辅助函数现在接收架构（`corpus_for("x64" | "x86")`），因此两种 ABI 由同一断言覆盖。
- 在 `cargo test` 中断言语料是 lint 干净的（`the_corpus_passes_every_output_invariant`）：没有发现、受影响函数为零，且恰好检查 `2 × functions` 段文本，因此 CI 无需运行 CLI 就能抓住不变量回归。
- JSON lint 报告携带 `"affected"`（至少有一条发现的函数），与文本模式一致。
- `recraft lint` 会报告失败的波及范围（`findings: 180 in 7 functions`），因此影响许多函数的回归可以与单个异常案例区分开。
- `recraft lint --limit 0` 会报错（`no texts checked: raise --limit above 0`），而不是报告一次什么也没检查的干净运行；这与未分析项目的情形使用同一守卫。
- 检查器本身也有覆盖：一个单元测试为每条不变量喂入一段文本，并断言每条规则都会触发（跳转没有标签、标签重复、花括号不平衡、空条件、未声明参数、签名上方出现标签），外加一个必须不报告任何问题的干净函数。
- `recraft lint` 会拒绝没有已分析函数的项目（`no functions: run `analyze` before linting the output`），而不是报告零发现 —— 后者看起来像通过。
- `docs/QUALITY.md` 的 golden 快照章节在一处列出五个集合及其函数数量以及 **203** 个函数的总数，因此在描述更新工作流的地方也说明了回归面。
- `recraft lint --limit N` 会照实写在摘要里（`functions: 3261 (checked first 300)`），此前它与已检查文本数放在一起读起来自相矛盾。
- `recraft lint --json` 还携带每条规则的计数（`"rules":{"goto without label":0,…}`），因此仪表盘可以分别绘制每条不变量，而不只是总数。
- 已提交的快照集记录在 `docs/QUALITY.md`：五个样本共 **203** 个函数（notepad 40、crackme 60、corpus 31、corpus86 32、go 40），全部通过 `--check`，与 `recraft lint` 和 `recraft metrics` 并列为三层回归。
- `tests/golden/notepad/` 新增前 40 个 notepad.exe 快照（`--limit 40`）：用于每次测量的主样本现在也有自己的已提交文本快照。Golden 总数：notepad 40、crackme 60、corpus x64 31、corpus86 32、go 40（**203**）。
- 句柄命名也能看到按地址传递的输出参数：`RegOpenKeyExW(…, &hKey)` 和 `WindowsCreateStringReference(…, &hstringHeader, &hstring)` 过去保留 `var_…`，因为匹配器要求裸槽位名。WinRT 这一对是表中的新条目（`HSTRING_HEADER*` 是第 3 个参数，`HSTRING*` 是第 4 个）。Notepad：92 处具名句柄使用，影响 13 个函数；L2 行数不变。
- 两个 README 都在 `metrics` 和 `golden` 旁边列出 `recraft lint <project> --level l1`，因此三条验证命令（不变量、快照、指标）可以从快速开始中找到。
- `tests/golden/crackme/` 向已提交语料新增前 60 个 crackme.exe 快照（`--limit 60`），因此暴露出缺失 switch 分支标签的样本现在也有快照覆盖。Golden 总数：corpus x64 31、corpus86 32、crackme 60、go 40。
- `docs/REPORT-2026-10-31.md` 总结了本阶段工作：notepad 的前后数字（L2 行数 34 259 → 30 102、`goto` 1 264 → 452、具名全局 0 → 2 069、字段访问 137 → 777、45 KB 函数 1.6 s → 1.2 s）、五个样本的基线、每个主题改了什么、剩余限制（多入口循环是唯一剩下的结构性缺口）以及可复现每个数字的命令。
- 整个 Go 样本现在都通过输出不变量：`recraft lint .tmp/go.rcraft --level l1` 检查全部 **3261** 个函数并报告 **0** 条发现（每个 `goto` 都有标签、标签只定义一次、字符串之外花括号平衡、没有空条件、签名声明了它们使用的内容、签名上方没有标签）。连同 Notepad（502 个函数，L1+L2）、crackme（493）和两个语料（63），这覆盖了工作区中的每个样本。
- `recraft lint --level l1|l2` 只检查一个层级而不是两个，使大型项目的工作量减半（3261 个函数的 Go 样本在两个层级下无法在合理时间内完成；对其前 300 个函数执行 `--level l1` 约需 14 s）。摘要行会说明所检查的层级。
- `recraft lint` 从头部行的**最后一个** `(...)` 取签名的参数列表，因此名称本身带括号的 Go 或 C++ 符号（`int64 internal/abi.(*RegArgs).IntRegArgAddr(int64 arg1, …)`）会被正确解析，而不会报告 `arg4 > 0`。前 300 个 Go 函数（600 段文本）lint 干净；此前报告的 33 条“未声明参数”发现就是这个解析假象。
- **修复**：打印为 `case 0: goto loc_140007014;` 的跳转表分支保留了 `goto`，但目标标签从未被发射，于是输出引用了未定义的标签 —— crackme.exe 中有 180 个这样的跳转（notepad 不受影响，因为它的 switch 形态不同）。跳转目标解析器此前只识别行首或条件之后的 `goto loc_…`，不识别 `case` 标签之后的；现在它会在语句中的任何位置找到该关键字。`recraft lint` 再次报告 crackme 干净（493 个函数，986 段文本，0 条发现），Notepad 不变。
- `recraft lint --json` 把报告打印为一个 JSON 对象（`{"functions":…,"checked":…,"findings":…,"items":[…]}`）供 CI 和 UI 使用；退出码与文本模式相同。
- `recraft lint <project> [--limit N]` 对每个函数和两个层级检查输出不变量，违规时以非零退出码结束：每个 `goto` 都有标签、标签只定义一次、字符串字面量之外花括号平衡、没有 `if ()`/`while ()`、签名声明了函数体使用的每个 `argN`、签名上方没有 `loc_` 标签。它会报告摘要以及首批问题。Corpus：62 段文本，没有发现；Notepad（全部 502 个函数，1004 段文本）也没有报告。
- `merge_identical_if_arms` 在扫描候选基本块时会重新计算每一行的花括号，使该 pass 在深层嵌套代码上变成平方复杂度。现在逐行净花括号计数只计算一次（并在罕见的合并后重算）—— 运算不变，因此输出逐字节相同：45 KB 函数的文本 pass 从约 262 ms 降到约 225 ms。
- 又两条输出不变量现在是对语料的单元测试：**函数体使用的每个参数都在签名中声明**（缺口会打印 `arg5` 而没有 `arg4`）以及 **签名行上方不发射任何 `loc_` 标签**。两者都针对 L2 检查（标签规则也针对 L1），因此第 35 轮的栈参数加宽和第 41 轮的标签放置不会悄悄回归。
- Golden 快照支持 Go 样本：`recraft golden` 过去在它上面失败（`os error 123`），因为 `internal/abi.(*RegArgs).IntRegArgAddr` 之类的 Go 名称以及非常长的名称不适合作为文件名。名称会被清理并截断到 80 个字符（去重 pass 在冲突时仍会追加地址），`recraft golden --limit N` 只写入/检查前 N 个函数，因此 3 261 个函数的项目也能得到可管理的样本。`tests/golden/go/` 保存前 40 个 Go 快照并通过 `--check`；语料和 32 位快照不变。
- 跳回函数首地址的跳转过去会把标签放在函数上方的头部注释上（`loc_140001ba0:` 打印在 `// L2 structure recovery` 之前），这也隐藏了该跳转所属的循环。现在标签放在函数体的第一条语句上，位于头部和附着在该地址的任何注释之后，注释行永远不会获得标签。在 Go 样本上这恢复了 24 个函数：goto 回退函数 220 → **196**，标签 1165 → **1059**（覆盖率 98.7% 不变）。Notepad、crackme 和两个语料不变。
- `fold_sibling_tail_gotos` 过去为每个候选跳转扫描整个函数来查找目标标签；现在它一次构建标签索引，并在拼接移动尾部时平移它（拼接很少见）。45 KB 函数的文本 pass 从约 313 ms 降到约 265 ms，全部 502 个函数的输出逐字节相同。
- 只接收过一个参数的栈槽就是该参数，而不是局部变量：被 `var_40 = arg4;` 恰好写入一次且从未取过地址的槽位，其读取会被重命名为 `arg4` 并移除溢出（`qword_140034e40 = arg4;` 而不是 `var_40 = arg1; … = var_40;`）。Notepad：88 个这样的溢出中有 72 个在 53 个函数中被折叠，L2 行数 30 174 → 30 102。被写入多次、或地址被传给调用的槽位保留其名称。
- L2 头部显示序言保留的栈帧：`// L2 structure recovery (11 blocks, frame 0x48)` —— Notepad 的 502 个函数中有 101 个拥有常量大小、无 push 的序言并会报告它，因此读者无需遍历序言就能看到函数保留了多少局部状态。两个栈参数也被推断为指针类型（`int64* arg5`、`void* arg7`）。
- x64 栈参数按其值在伪 C 中被访问的方式推断类型：当槽位被复制到寄存器、且该寄存器被用作内存基址时，参数变成 `char*`/`wchar_t*`/`int32*`/`int64*`（宽度不一致时是 `void*`），而不是 `int64`。Notepad 的栈参数都是普通值，因此那里的输出不变；一个单元测试覆盖通过 64 位指针写入的槽位。
- 32 位栈参数按其使用方式推断类型：当加载的值成为另一次内存访问的基址时，参数是指针，访问宽度决定 `char*`/`wchar_t*`/`int32*`（不一致时是 `void*`）。x86 语料现在读作 `str_probe(char* arg1)`、`wide_probe(wchar_t* arg1)`、`array_probe(int32* arg1, int32 arg2)` 和 `field_probe(int32* arg1)`，与 x64 路径从寄存器宽度推断的一致。
- 序言移动 `esp` 之后重新读取参数的 x86 函数过去会得到两个同名参数（`nested_loops(int32 arg1, int32 arg1)`）；现在参数按名称去重，得到与 x64 一样的 `nested_loops(int32 arg1)`。
- 32 位栈参数按其槽位宽度推断类型：自身类型未知的 x86 参数是 `int32` 而不是 `int64`，因此整个 x86 语料中 `int32 __cdecl add_leaf(int32 arg1, int32 arg2)` 取代了 `(int64 arg1, int64 arg2)`（32 个签名）。用户原型仍然覆盖它。32 位语料也获得了已提交快照（`tests/golden/corpus86/`，32 个文件），因此 x86 路径与 x64 路径拥有相同的回归覆盖；`recraft golden <project> --dir tests/golden/corpus86 --check` 通过。
- 读取调用者栈参数的 x64 函数会把它们显示为参数：当序言是 `sub rsp, const`（没有 push）时，对 `frame + 0x28` 及之后的读取是 `arg5`、`arg6`、…，而不是 `var_` 槽位，签名会加宽以覆盖它们，参数注释写明 `arg5+=stack`。Notepad：9 个函数，例如一个过去转发 `var_80, var_88, var_90` 的尾调用现在转发 `arg5, arg6, arg7`。函数写入的槽位仍是槽位（它们是函数自己的传出参数），序言 push 寄存器的函数保持原样，因为它们的帧偏移不是相对于扫描找到的值。
- 文本 pass 在最大函数上不再比需要的更慢：`hide_callee_saves` 过去为每个保存槽位扫描每一行（平方复杂度），`propagate_argument_copies` 为每次参数复制重写每一行。前者现在一趟构建槽位 → 行索引，后者跳过未提及该寄存器的行。45 KB 窗口过程每次反编译从 **约 1.6 s 降到约 1.3 s**，其中文本 pass 从约 800 ms 降到约 310 ms；发出的文本逐字节相同。`RECRAFT_PROFILE=1 recraft decompile …` 会打印各阶段耗时（lift、analysis、emit、文本 pass）供后续工作使用。
- `recraft metrics` 从同一趟 L1 报告另外两个计数：`labels`（结构化器不得不发射的 `loc_` 定义）和 `inline asm statements`（伪 C 中残留的 `__asm("…")` 行）。Notepad：502 个函数中 267 个标签和 138 条内联汇编；语料报告 0 和 9。这两个数字此前都是在本笔记中手工统计的，因此 `docs/QUALITY.md` 中的数字现在可以用一条命令复现。
- 为一次比较而加载到寄存器中的常量会移入比较：`eax = 0x111; … if (arg2 > eax)` 读作 `if (arg2 > 0x111)`，并且当该寄存器在任何其他读取之前被重写时（因此值不可能活跃）赋值会被丢弃。Notepad：35 个函数，L2 行数 30 208 → 30 174；窗口过程的消息分派现在读作 `if (arg2 > 0x111) { if (ebp > 0x233) {`，安全 cookie 检查读作 `if (rax == 0x2b992ddfa232)`。两个已知常量之间的比较保留其寄存器形式 —— 求值它会打印一个常量条件。
- 句柄表覆盖更多 API：注册表函数把它们的键槽位命名为 `hKey`（`RegOpenKeyExW`/`RegQueryValueExW`/`RegSetValueExW`/`RegEnumKeyExW` 现在与 `RegCloseKey` 匹配），`DestroyIcon`/`DestroyCursor` 给出 `hIcon`/`hCursor`，`UnhookWindowsHookEx` 给出 `hHook`，窗口 API（`ShowWindow`、`SendMessageW`、`PostMessageW`、`SetWindowPos`、`InvalidateRect`、`RedrawWindow`、`SetWindowTextW`、`GetWindowLongPtrW`）把第一个参数命名为 `hwnd`。Notepad：137 处具名句柄使用（从 120 增加），例如 `RegCreateKeyExW(…, &hKey, …)` 和 `SendMessageW(hwnd, …)`。
- 地址常量现在会命名它们指向的东西：对数据段内的地址，`rdx = 0x14002b210;` 读作 `rdx = off_14002b210;`；等于函数起始地址的常量变成该函数（`return sub_140001c64(sub_140027590);`）；与字符串字面量匹配的常量变成该字符串。与这些都匹配不上的数字保持原样（`rbx = 0x2b992ddfa232;`）。Notepad：168 个函数中 358 个 `off_` 名称和 5 个函数指针，裸地址赋值 511 → 337（其余是映像基址/代码地址和普通常量）。Golden 规范化器也会掩码 `off_` 的数字，且 `dispatch.c` 已重新生成。
- 全局数据读作具名 word，而不是转换表达式：`*(u64*)(0x140036648)` 变成 `qword_140036648`，`*(u32*)(0x140034460)` 变成 `dword_140034460`，`u8`/`u16`/`f32`/`f64`/`__m128` 给出 `byte_`/`word_`/`flt_`/`dbl_`/`xmmword_`（宽度仍然可见，而像 `*(u32*)(0x140034460 + 4)` 这样的计算地址保留其转换）。Notepad：189 个函数中 **1725** 处全局读取，不再残留绝对地址转换；`qword_140036648 = rax;` 读起来就像它实际是的那次全局存储。Golden 规范化器掩码数字（`qword_ADDR`），因此快照保持地址无关；三个语料快照已重新生成（`dword_ADDR`、`flt_ADDR`）。
- 类型推断留作 `void*` 的指针参数在偏移明确时会显示字段访问：`*(u32*)((arg1 + 0x10))` 读作 `arg1->field_10`，裸 `*(u8*)(arg1)` 读作 `arg1->field_0`。该 pass 只重写所有访问都使用同一宽度的偏移 —— 字段名无法说明它指哪个宽度，因此同时被读作 `u32` 和 `u8` 的 `arg2 + 0x18` 保留其转换。Notepad：`argN->field_XX` 访问 266 → **777**（+511，97 个函数），原始 `*(…*)((argN + …))` 转换 1418 → **18**，行数不变，仍为 30 208。
- 接收到句柄 API 的槽位会得到句柄名：`CloseHandle(var_88)` 读作 `CloseHandle(hObject)`，`RegCloseKey(var_38)` 读作 `RegCloseKey(hKey)`，`ReleaseMutex` 为 `hMutex`，`SetEvent`/`ResetEvent` 为 `hEvent`，`DestroyWindow` 为 `hwnd`，`WindowsDeleteString` 为 `hstring`，`FreeLibrary` 为 `hModule`，`FindClose` 为 `hFind`，`DestroyMenu` 为 `hMenu`，`ImageList_Destroy` 为 `himl`，`ReleaseDC` 的第二个参数为 `hdc`。被多个 API 接收的槽位取最有说明性的种类（互斥体句柄即使后来被关闭也保持 `hMutex`）。Notepad：9 个函数中共 120 处具名句柄使用。只用 `CoTaskMemFree` 释放的槽位保留其编号：通用的 `pBuffer` 并没有明显更好。
- 被 API 填充的结构体会得到真实名称：`GetClientRect(hwnd, &var_30)` 把该槽位变成 `rect`，于是代码读作 `rect = 0; GetClientRect(hwnd, &rect); edx = rect.bottom; edx -= rect.top;` 而不是 `var_30`/`var_34`/`var_38`/`var_3c`。`MSG` 槽位变成 `msg`，`POINT` 槽位变成 `pt`；函数中已被使用的名称会加数字后缀（`rect2`）。Notepad：8 个函数、54 处具名访问。这是朝向变量命名的一小步，由同一张已知结构表驱动。
- 什么都不做的语句会被丢弃：`rax = rax;`（以及槽位自赋值）、函数体为空的 `if`，以及没有任何跳转指向、且只引入右花括号的标签。Notepad：3 处，L2 行数 30 212 → **30 208**。
- 分析 pass 不再格式化它丢弃的语句。`lift_block` 新增 `render` 标志：第一趟只需要基本块的效应（写入、调用、栈使用、参数复制），因此它为 Notepad 最大的函数跳过了约 37 000 次语句渲染。发出的文本逐字节相同；本机墙钟变化在运行间噪声范围内（45 KB 函数两种方式都是 1.6 s），因此应把它视为减少的工作量，而不是测得的加速。
- 按名称传入参数寄存器的直接调用现在会显示实际放入的值。`edx = 0; sub_1400029dc(&var_20, rdx, …)` 虽然值已知却仍打印寄存器；现在读作 `sub_1400029dc(&var_20, 0, …)`。只替换普通值（常量、栈槽、`&var_XX` 地址）—— 加载或算术表达式会在参数列表中被求值两次，因此它们保留寄存器名。Notepad：77 个函数中的 175 个调用参数得到改善，行数不变。条件表达式在相邻临时值折叠中保持为具名临时值（`r9 = ((al != 0) ? rax : 0);` 比内联更好读），`RECT` API 列表扩展到其他矩形产生者/消费者（`SetRect`、`InflateRect`、`OffsetRect`、`CopyRect`、`InvalidateRect`、`PtInRect`、`GetUpdateRect`、`AdjustWindowRectEx`）。
- 从不返回值的函数现在会如实声明：`int64 sub_140001440(void) { … return; }` 变成了 `void sub_140001440(void)`，因为每个出口都是裸 `return;`。以调用结尾的函数体保留整数类型 —— `f(); ret` 在源码中可能是 `return f();`，此时寄存器宽度签名才是诚实的读法。Notepad：29 个签名得到修正，L2 文本其余不变；兄弟元数填充（见下文）也学会了读取 `ApiName(…)` 调用点，尽管 notepad 剩余的空调用都是签名表中的零参数 API。
- 参数在另一个基本块中设置的直接调用会打印其元数。这样的调用不记录参数，因此即使同一函数中对同一目标的兄弟调用显示三个参数（`sub_X(a, b, c)`），它也打印 `sub_X()`；现在最宽的兄弟提供计数，调用点打印相应数量的寄存器占位符。参数超过四个的调用保持原样（栈参数的位置无法仅凭寄存器名确定）。Notepad：空直接调用 426 → **291**，影响 49 个函数，行数不变。`POINT` 也加入已知结构表（`GetCursorPos`/`ClientToScreen`/`ScreenToClient` → `var_XX.x`/`.y`）；notepad 没有这样的调用，因此这部分没有 notepad 增量，但覆盖了确实有这类调用的 GUI 二进制。
- 只写一次、且被紧邻的下一条语句读取一次的寄存器临时值会内联进去，就像源码级局部变量那样：`eax = *(u64*)(rbx); sub_140001000(eax);` 变成 `sub_140001000(*(u64*)(rbx));`，`ecx = var_a0; var_50 = ecx;` 变成 `var_50 = var_a0;`。只有寄存器临时值会被折叠 —— 栈槽是变量，因此其地址（`&var_30`）绝不会被替换成存入其中的值，而带顶层运算符的表达式在代入时会加上括号。Notepad：L2 行数 30 256 → **30 212**，影响 41 个函数，`goto` 计数和覆盖率不变；一个 corpus golden（`array_probe`）因新文本而更新。
- 知名导入现在会打印正确数量的参数。反编译器此前没有 IAT 调用的参数计数，因此在较早基本块中设置寄存器的调用会打印 `CoTaskMemFree()`，而带遗留寄存器的零参数 API 会打印一个幽灵参数（`GetCurrentThreadId((ecx + 1))`）。一张涵盖 66 个常见 Win32 API 的参数计数表限定了列表：调用恰好打印那么多槽位，未解析的用 ABI 寄存器名填充，零参数 API 完全不打印参数。Notepad：`CoTaskMemFree(rcx)`、`WaitForSingleObject(rcx, 0)`、`CloseHandle(*(u64*)((rcx + 8)))`、`GetCurrentThreadId()`（假参数消失）；空参数 API 调用 305 → **约 256**，行数和 goto 不变（30 256 / 452）。未知导入保留旧的寄存器设置规则。
- `MSG` 加入已知结构表中的 `RECT`。`GetMessageW`/`PeekMessageW`/`TranslateMessage`/`DispatchMessageW` 接收 `MSG*`，因此后续成员访问会打印 `var_40.hwnd`、`var_40.message`、`var_40.wParam`、`var_40.lParam`、`var_40.time`、`var_40.pt.x`/`.pt.y`，而不是裸槽位（`if (var_40.message == 0x50)`、`return var_40.wParam;`）。重写器现在是幂等的 —— 同一函数中的第二次消息循环调用不再给已经是成员路径的名称再加前缀（`var_40.hwnd.wParam`）。Notepad：1 个函数中的 3 个消息字段，共 30 处结构体字段访问，行数和 goto 不变。
- 由 `GetClientRect`/`GetWindowRect` 填充的 `RECT` 现在有字段名。这两个 API 接收 `RECT*`，因此传入 `&var_XX` 处的四个 32 位槽位就是该结构体的字段；后续读取和存储会打印 `var_30.left`、`var_30.top`、`var_30.right`、`var_30.bottom`，而不是 `var_30`、`var_34`、`var_38`、`var_3c`。地址（`&var_30`）和任何指针基址（`(var_3c + 8)`）保留普通名称，调用前的宽清零（`var_30 = xmm0;`）也是如此，因此只有真正的成员访问会被重命名。Notepad：6 个函数中的 27 处字段访问（`edi = var_30.bottom; edi -= var_30.top;`），行数不变。这是结构感知命名的第一步；`POINT`、`MSG` 等同族可以沿用同一张表。
- 更长的向前跳过区域会折叠为 `if` 块。`fold_short_forward_gotos` 过去只处理最多 24 行的守卫，因此大型分派/清理菱形仍保留为 `if (c) goto L; … goto M; loc_L:`；现在上限是 64（该 pass 仍会拒绝括号不平衡或绑定 `break`/`continue` 的区域）。Notepad：`goto loc_` 475 → **452**（45 KB 函数 391 → 369，例如一个 `GetProcAddress` 循环体现在位于 `if (r15d != 0) { … }` 内，而不是跳转之后），L2 行数 30 254 → 30 256（+2 个花括号），结构化函数和覆盖率不变。
- 携带的调用参数现在能跨基本块存活，即使它们命名了寄存器。跨块值分析过去只保留不含寄存器的值（常量、字符串、栈槽），因此当句柄来自 `rcx = *(u64*)(rbx)` 时 `CloseHandle()` 仍为空。现在每个携带的值都会记住它读取的寄存器，只有当其中一个在定义与调用之间被覆盖时数据流才会丢弃它 —— 而这正是打印文本会过时的时候。Notepad：空 Win32 API 调用 337 → **305**（例如 `CloseHandle(*(u64*)(arg1))`、`ReleaseSemaphore(arg1, 1, &var_48)`、`SetThreadpoolTimer(*(u64*)(rsi), &var_30, 0, 0x124f8)`），空 `sub_` 调用 437 → 426，L2 行数不变，仍为 30 254。
- 尾调用保留其参数。以 `rcx = 0x140027590; return sub_140001c64();` 结尾的函数过去会丢弃该值，因为结构化尾调用发射器只打印目标；现在跳转紧邻上方的设置行会按 ABI 顺序被消费（受被调用者恢复出的元数限制，因此未知被调用者只取 `rcx`），并打印为调用的参数。Notepad：空尾调用 74 → **63**，48 个尾调用现在显示参数（`return sub_140023d8c(*(u64*)(0x140036648))`），L2 行数 30 265 → 30 254。
- 结构化器的无条件循环打印为 `while (1) { ... }`，而不是 `do { ... } while (1);`。两者都是无限循环，但 `while` 形式才是编译器发出的、也是源码读起来的样子；该重写还会访问嵌套循环（此前被转换的循环内部的 `do` 会被跳过）。Notepad：`do { } while (1);` 12 → 0，`while (1)` 循环 48 → 60，行数和 `goto` 计数不变（30 265 / 475）。`matching_brace` 现在忽略字符串字面量中的花括号，因此包含 `{`/`}` 的格式字符串不会再让文本 pass 看不到循环体。
- 没人读取的帧指针设置不再打印。`rbp = &var_100;` 只是为了让槽位名能解析到入口帧；当该寄存器再也不会被命名时，这一行就是噪声，因此最后一趟会丢弃它（任何后续读取都会保留它）。Notepad：7 个函数中的 7 行（`sub_14000e950` 和其他 WndProc 辅助函数直接以第一条真实语句开始），L2 行数 30 272 → **30 265**；单元测试覆盖丢弃和保留两种情况。
- 伪 C 头部会标明产生它的层级：反编译视图过去即使显示 L2 签名视图也声称“L1 structure recovery”。第一条注释现在是 `// L1 …` 或 `// L2 …`，取决于入口点；L1 文本和每个 golden 快照不变。本次会话新增的分析也获得了单元测试：参数别名数据流（只有当所有路径一致时复制才保持，某一路分支中的覆盖会清除它）以及基于寄存器的帧恢复的栈布局（`lea r11, [rsp+0x50]; mov rsp, r11`）。
- 通过寄存器恢复帧指针的尾声不再丢失栈位移。像 `lea r11, [rsp+0x50]; mov rsp, r11; pop r14; pop rdi; pop rbp` 这样的尾部过去会让整个基本块的 `rsp` 布局未知，于是函数中每个槽位都回退到原始偏移（`var_n48`）；现在已知持有栈地址的寄存器会解析该更新，打印器通过同一个值恢复位移。Notepad：`var_nXXXX` 名称 47 → **18**，两个单基本块函数变化（`sub_14001f420` 现在读作 `var_20`/`var_24` 而不是 `var_n48`/`var_n44`），L2 行数不变，仍为 30 272，`goto loc_` 不变，仍为 475。
- 通过保存的 `rsp` 别名建立的帧保留其槽位名。像 `mov rax, rsp; push ...; lea rbp, [rax-0x148]` 这样的序言过去无法解析：从栈地址命名寄存器只理解 `rsp` 本身，因此 `[rbp-0x30]` 打印为 `var_n30`，而对同一槽位的其他访问却使用规范化名称。现在打印器会从任何已知持有地址的寄存器解析地址（`rsp - 8`、`rax - 0x148`），因此帧基址会沿别名传播。Notepad：`var_nXXXX` 名称 115 → **47**，10 个函数变化，L2 行数 30 275 → 30 272，`goto loc_` 不变，仍为 475。
- 以 `sub rsp, rax` 分配帧的函数中，栈槽保留其帧基址。MSVC 把帧大小加载到 `eax`，调用 `__chkstk`（它保留 `rax`）再减去 `rax`；提升器把 `rax` 视为被调用破坏，于是整个函数的栈位移变成未知，每个槽位名回退到原始偏移（`var_n1008` 而不是 `var_c0`）。现在该常量会跨越探测调用被携带，因此 `sub rsp, <size>` 是普通步骤：notepad `var_nXXXX` 名称从 **2090 降到 115**（45 KB 函数 1891 → 1，其帧读作 `rbp = &var_100`），六个函数变化，L2 行数 30 277 → 30 275，`goto loc_` 不变，仍为 475。
- 参数设置被基本块边界拆开的调用现在会打印其参数。打印器的参数寄存器值以与别名相同的 must 分析携带（前驱交集，被调用清除），因此后续基本块中的调用会打印较早基本块加载的内容。只有不命名活跃寄存器的值会被携带 —— 常量、字符串字面量和栈槽 —— 因为像 `*(u64*)((rcx + 8))` 这样存储的字符串会在另一个基本块中读取寄存器的*新*值。Notepad：空 Win32 API 调用 433 → **337**（41 个函数），例如 `ReleaseSRWLockExclusive()` 现在打印其锁参数，L2 行数 30 276 → 30 277，`goto loc_` 不变，仍为 475。
- 指针类型参数现在会在每个基本块中拼写字段访问，而不只是在复制它们的那个基本块。`r14 = arg4` 后跟 `mov ecx, [r14+0xC]` 过去会打印 `*(u32*)((r14 + 0xc))`，因为别名只在单个基本块内跟踪；现在打印器会得到在 CFG 上计算的 must 别名映射（`entry_alias_map`：只有当所有路径一致时寄存器才把参数带到某个基本块 —— 前驱映射的交集，再应用基本块自身的复制）。只有当访问宽度等于指针元素大小时才使用字段拼写，因此通过 `void*` 的 `u8` 加载保留其显式转换。Notepad：121 个函数中的 `argN->field_XX` 访问 137 → **266**（`sub_14000e950` 现在读作 `arg4->field_c` / `field_4` / `field_8` 而不是原始转换），L2 行数 30 272 → 30 276，`goto loc_` 不变，仍为 475，corpus golden 31/31。
- 不再打印 MSVC 的 `__chkstk` 栈探测。大于一页的帧通过把大小加载到 `eax` 并调用探测辅助函数来分配，后者在真正序言之前触碰保护页；它没有源码级效应，因此该调用及其大小加载会被丢弃。辅助函数按形态识别（`sub rsp, imm` 之后六条指令内出现 `mov [rsp], r10`/`mov [rsp+8], r11`），而不是按地址，且调用点必须是裸的无参调用。Notepad：六个拥有 4 KB+ 帧的函数（`sub_140013154`、`sub_140007570`、…）失去其 `eax = 0x10c8; sub_140027440();` 开头（notepad L2 行数 30 291 → 30 272）。
- 函数不再因为一个内层循环离开其父循环就被整体发射为扁平标签。只有出问题的顶层循环会以带标签的基本块发射（其函数体使用原始 `loc_`/`goto`，保留每条边），函数其余部分保持 `while`/`if` 结构。验证器函数仍得到它们需要的标签，但周围代码不再失去形态：45 KB 的 notepad 函数从 687 行 `goto loc_` 降到 **402**（10070 → 10080 行），notepad 总数从 771 降到 **475**。Go 样本增加一个结构化函数（1779 → 1780/2000）；corpus golden 和结构化率升至 487/502（97.0%）和 453/493（91.9%）。
- 反编译 notepad 最大的函数快了 **8.5 倍**：后处理 pass 过去会为每个候选重新扫描整个函数（单次使用临时值折叠的 token 计数、标签整理 pass 的标签引用计数）。现在 token 频率只收集一次并在折叠发生时维护，跳转目标单趟收集，IAT 名称映射每个函数构建一次而不是每个基本块一次。那个 45 KB / 1 620 基本块的函数从 12.2 s 降到 **1.4 s**（调试构建），同时 502 个 notepad 函数的 L2 输出逐字节相同 —— 这纯粹是重构。
- 带标签的回退文本现在会从它自己的跳转中恢复结构。区域以跳回它自己结尾的标签会变成 `do { ... } while (cond);`（无条件回边变成 `while (1)`），而被跳过区域较短（≤ 24 行）且花括号平衡的向前 `if (cond) goto L;` 会变成 `if (!cond) { ... }`。两种重写都不移动语句：标签仍然有效（C 标签是函数作用域），跳入该区域的跳转仍能工作，使用 `break`/`continue` 的循环区域保留其跳转，因此与外围循环的绑定不会改变。更长的跳过保留跳转 —— 包裹数百行会让函数其余部分缩进到屏幕之外。Notepad 的 `goto loc_` 行从 1 235 降到 **771**（242 个恢复的 `do` 循环）；结构化率从 486 升至 487/502（97.0%），crackme 案例从 450 升至 452/493（91.7%），Go 样本从 1 767 升至 1 779/2 000（88.9%）。
- 序言建立的帧指针（`rbp = rsp - N`）现在为每个基本块命名 `[rbp + disp]`。一个从不重新加载 rbp 的基本块过去会用原始 rbp 偏移命名同一个槽位，而序言用 `rsp` 命名它，于是一个位置出现两个名字（序言中的 `var_18`、分支中的 `var_n10` —— 以及 L1 中原始的 `*(u64*)((rbp + 0x200))`）。`lift_block` 现在报告它看到的栈地址寄存器，入口基本块提供帧指针，每个基本块都从该基址开始：58 个 notepad 函数为每个槽位得到一致的名字（`var_48`/`var_40` 而不是 `var_18`/`var_n10`），L1 中也不再残留原始帧表达式。Corpus golden 不变。
- 不再打印 MSVC `/GS` 栈 cookie 的簿记。序言把 `__security_cookie ^ rsp` 存入一个帧槽，每条返回路径都会把它折回并调用 `__security_check_cookie`；两者都是编译器生成的，而把该调用打印为函数结果（`return sub_1400019f0(rcx);`）掩盖了实际返回的值。检查点现在读作 `return 0;`（真实值从 `eax` 折叠而来），并且一旦没有检查读取该槽位，设置就会被丢弃。识别基于形态 —— 一个与 `rsp` 混合的全局值存入某个槽位 —— 因此不假定 API 名称，没有这种设置的函数保留其调用。Notepad：44 个函数中的 126 个检查点，出现次数从 155 降到 2（剩下两个是检查器本身和一次未解析的调用），帧 cookie 的加载/折叠从 74 降到 7。L1 输出（它不会填充寄存器重建集合）把检查的加载保留为原始栈表达式（`*(u64*)((rbp + 0x200))`），而不是 `var_XX` 名称；匹配器接受两种形式。在此之前，L1 丢弃了设置却保留了检查，留下对一个没人写入的槽位的比较 —— L1 现在与 L2 一样隐藏这些检查（剩余 2 处）。
- 被基本块边界与其参数设置分开的调用现在会打印其参数。基本块在跳转目标处结束，因此 `mov rcx, [global]` 和它供给的 `call` 可能落在不同基本块中（并且 call 所在基本块会用全新的值跟踪发射）；参数从调用上方的直线段恢复，遇到标签、花括号和会破坏参数寄存器的调用就停止。只有已知非空元数的被调用者才会从整段填充；其他任何直接的 `sub_...` 调用仅当其第一个参数写在紧邻上方一行时才填充，因此游离的寄存器值不会变成无参调用的参数。Corpus `recursive` 读作 `recursive(ecx)` 而不是 `recursive()`；notepad 增加了 99 个调用（来自上方 `ecx = eax;` 行的 `sub_14000b320(eax)`，…）。这也会填充作为共享尾部复制的调用，因为每个副本都根据其自身前面的行重写。
- 跳到某标签、而该标签的基本块就是返回路径的 `goto` 会折叠为 `return <value>;`：一旦目标的 `eax = 0; return;` 就是整个基本块（包括被多个出口共享的标签），`if (eax == 0) goto loc_140013a2d;` 就读作 `if (eax == 0) return 0;`。没有跳转引用该标签时它就会消失；notepad 中有 29 个跳转被折叠。
- 交叉引用现在是 IDA 的 `xrefs to …` **对话框**，而不是停靠标签页：`X`（仅此一个 —— `F7` 已退役）打开一个以所选项命名的窗口，带有 `Direction`（`Up`/`Down`）、`Type`（`c` 调用、`j` 跳转、`o` 偏移、`r` 读、`w` 写）、`Address`（`main:loc_1400010b9`）和 `Text` 列。每行显示另一端的指令，并把被引用的字符串作为 `lea rdx, [0x…]  ; 'flag{…}'` 附在后面。`Down` 行是新加的：选中一条指令也会列出它引用了什么。`OK`/Enter 跟随选中行，`Cancel`/Esc 关闭，`Search` 过滤各列，`Help` 显示类型字母图例；方向键移动光标，页脚报告 `Line N of M`。
- 不再打印帧别名行：`mov rbp, rsp`（或异常帧中的 `mov r11, rsp`）只是建立用于命名帧相对槽位的别名，而该别名从不会在伪 C 中被读回。簿记仍会运行，因此 `var_nXX`/`frame_XX` 命名不变；notepad 中减少 96 行。
- `while (1)` 循环体顶部与循环无关的设置会被提升到循环之上，使哨兵测试变成真正的条件：`while (1) { rax = &var_30; if (rcx <= rax) break; ... }` 现在读作 `rax = &var_30; while (rcx > rax) { ... }`。只有当循环体中没有任何调用、汇编或对该寄存器的写入时，该设置才会被移动。
- 整个函数中再也没被提到的寄存器写入会被丢弃（桩函数中的 `rbp = &var_b1;`、内联调用留下的设置寄存器）。所有拼写都计入；`rax`/`eax` 会保留，因为 L1 的裸 `return;` 仍会返回它；参数寄存器（`rcx`/`rdx`/`r8`/`r9`）会保留，因为调用只打印它恢复出的参数；包含汇编的函数会被跳过，因为 `rep movsb` 之类的文本会在不命名寄存器的情况下读取它们。notepad 中约 40 行。
- 只供给一次存储的临时值会折叠进去：`ecx = (uint32)(*(u16*)(rax)); *(u32*)((r8 + 8)) = ecx;` 变成 `*(u32*)((r8 + 8)) = (uint32)(*(u16*)(rax));`，命名栈槽同样如此（`ecx = 0; var_8 = ecx;` 变成 `var_8 = 0;`）。如果该寄存器之后仍然活跃，复制会保留，调用/分支也算作读取。
- 只为返回值构造的临时值会折叠掉：`ebx = 0x80004002; return ebx;` 读作 `return 0x80004002;`，`rdx = (c ? a : b); return rdx;` 读作 `return (c ? a : b);`（notepad 中 29 处）。
- 编译后的 `bool` 返回（`al = 1; return eax;`）会折叠为 `return 1;`：当常量紧跟 `eax` 返回时，它会被移到返回行上。非常量字节值保留显式写入，以便宽度仍然可见。notepad 中 33 个这样的返回，剩下 6 个读取字节寄存器的返回。
- 仅有序号的导入（COMCTL32 `#345`、shcore-path `#170`、…）现在也能解析出调用名：`(**(u64*)(0x14002a420))()` 读作 `ord_170()`。Notepad 中经 IAT 的未解析指针调用从 10 降到 2。
- 同一段直线代码中下一次提及是对其进行完整 32/64 位重定义的寄存器写入会被丢弃：`rax = var_b8; r9d = 0; r8d = 0; rax = var_b0;` 会去掉第一次加载。调用、分支、标签和汇编会停止扫描，读取该寄存器的重定义不算数，8/16 位写入（`al = 0`）会让更宽的值继续存活。Corpus `start` 快照失去一次死溢出和一次死参数复制；notepad 列表减少 76 行。
- 条件分支不会修改标志，因此在其前驱分支之后以另一次测试开头的基本块，会在自己的表达式区域中重新提升前驱的纯 `cmp`/`test`/`bt`，并打印真正的条件，而不是 `__asm("if (je ... /* unmodelled flags */) goto ...")`。`lock cmpxchg` 也会记录它执行的比较，因此 compare-and-swap 之后的宝试读作 `if (arg1->field_2c != eax) continue;`。Notepad 的未建模分支从 44 降到 21，内建指令计数从 172 降到 170；一个大函数现在需要为共享尾部使用 `goto`（结构化 487 → 486）。
- `sbb x, x` 按其本质（进位掩码）提升为 `x = cond ? -1 : 0`，而不是汇编，其常见消费者会折叠成一条语句：`x = -(x)` 变成布尔值 `x = (cond ? 1 : 0)`，`x &= y` 变成条件掩码 `x = (cond ? y : 0)`。进位标志从正确的产生者推导：`cmp`/`sub` 给出无符号借位，`neg` 给出“操作数非零”（`neg` 之后的 `jb`/`jae` 现在读作 `!= 0`/`== 0`），`bt` 给出被测试的位。Notepad 的 54 行 `sbb` 汇编消失，内建指令计数从 219 降到 172（覆盖率 99.5%）。
- 参数超过四个的调用现在会打印通过栈传递的参数：对传出参数区（`[rsp+0x20]`、`[rsp+0x28]`、…）的存储会一直收集到该调用，并按槽位顺序追加，同时解析复制为其携带的值。一次格式字符串调用读作 `sub_1400083f8(rbx, 0x25, "%08lX-%04X-...", var_2f, (uint32)(var_33), ...)`，`RtlVirtualUnwind` 显示全部八个参数；notepad 中有 149 个这样的调用。只有已经显示其四个寄存器参数的调用才会被扩展，因此未知元数绝不会把栈槽放到错误位置。
- 槽位名称现在也会跟随基本块内部的栈指针变化：位移在打印每条语句之前更新，因此基本块较早处的 `push`/`sub rsp` 不会再让后续访问按栈指针未移动来命名（notepad 的 unwind 辅助函数中 `&var_98` → `&var_50`）。
- 标量浮点参数和返回值会被推断类型：在被写入之前读取的 `xmm0`..`xmm3` 成为 `f32`/`f64` 参数（内存操作数的宽度区分 `float` 与 `double`），而把结果留在 `xmm0` 且不写 `eax` 的函数声明为 `f32`/`f64`。Corpus `float_math` 现在读作 `int32 float_math(f32 arg1)`，其中 `arg1=xmm0`，而不是无参签名。没有原型时，整数/浮点参数的混合顺序无法恢复，因此那些 `xmmN` 寄存器保持未命名 —— 用户原型除外，它会把浮点参数映射到 `xmm0`..`xmm3`，并在签名中保持其声明顺序（`float scale(float factor, int count)`）。
- 只用于把值送入存储的寄存器会折叠进去：`rcx = &var_58; var_30 = rcx;` 变成 `var_30 = &var_58;`。只有当该寄存器在同一段直线代码中稍后被重定义时才会移除复制，因此调用或分支仍可能读取的值会保留其往返。Notepad 列表减少 220 行。
- 栈槽名称现在是每个函数一套空间：每个基本块的 `rsp` 位移都会沿控制流图传播，槽位相对帧底部命名，因此序言、函数体和每个尾声对 `var_XX` 的含义达成一致。持有栈地址的寄存器（保存的 `rsp` 副本、重建的 `rbp`）携带相同的基址，因此命名同一槽位的 `[rbp-0x30]` 和 `[rsp+0x18]` 打印相同的名称；入口保存、出口恢复的寄存器即使两次访问位于不同栈深度也能被识别。以两个不同位移到达的基本块（`alloca`、动态调整）保留之前的名称。
- 匹配的 `push`/`pop` 对被视为簿记并从列表中省略：入口保存、离开时恢复的寄存器不再显示为 `var_n8 = rbx;` / `rbx = ...;`。配对按寄存器以 LIFO 顺序匹配，且仅当每个 push 都有 pop 时才行，因此供给调用栈参数的 `push` 仍可见。单独一条序言溢出、其恢复不在列表中的也会被隐藏；在第一个分支或调用之后，同一存储可能成为栈参数，因此会保留。
- 被捕获的参数值在其设置与调用之间的栈存储之后仍然存活：对 `var_XX` 槽位的存储无法改变通过指针参数到达的内存，因此不再阻止设置移除。Notepad 的参数设置行从 4060 降到 3949。
- 取局部变量地址现在会如实读作如此：`rax = (rsp + 0x30);` 和 `rcx = (rsp + 0x68);` 渲染为 `rax = &var_30;` / `rcx = &var_68;`，因此传给输出参数的指针可见为 `RtlLookupFunctionEntry(rdi, &var_50, 0)`。帧设置仍保留 `frame = rsp`。Notepad 列表转换了 348 个这样的赋值。
- 裸帧槽访问会像其他局部变量一样命名：`*(u64*)(rsp)` 和 `*(u64*)(rbp)` 渲染为 `var_0`（`push`/`[rsp]` 槽），而不是无类型解引用。Notepad 列表减少 56 个这样的解引用。
- `bts`/`btr`/`btc` 会被提升，而不是保留为汇编：`bts ebx, 0x1c` 渲染为 `ebx |= 0x10000000;`，`btr edx, 0x1c` 渲染为 `edx &= 0xefffffff;`（常量位索引被折叠进掩码，旧位标志的定义仍可供后续 `jb`/`jae` 使用）。`cdqe` 渲染为 `rax = (int64)(eax);`。Notepad 列表减少 29 行汇编（内建指令 244 → 219）。
- 参数恢复不再把清零惯用法算作输入：函数顶部的 `xor r8d, r8d` 过去看起来像读取 `r8` 并凭空造出一个参数，而指针证据会被记到恰好作为内存基址的寄存器上，即使它已被重新赋值（`bytes_op` 把它的 `int len` 定型为 `int32*`）。现在读取会忽略自抵消形式，指针类型会沿地址回溯到产生它的参数，穿过寄存器复制（`r10 = arg1`、`rdx = r10`）。在 notepad 上，四参数签名从 104 降到 62（其中大多数带有一个幽灵 `r8`/`r9`），corpus `bytes_op` 现在读作 `int32 bytes_op(void* arg1, int32 arg2)`，与其双参数源码一致。
- 标量参数按其被函数体读取的宽度定型：只通过其 32 位半部读取的参数现在渲染为 `int32` 而不是 `int64`（基于内存的指针推断仍然优先），因此 `count_bits(int32 arg1)` 和 `array_probe(int32* arg1, int32 arg2)` 与源码一致。Notepad 的 502 个签名中至少有 166 个获得一个 `int32` 参数。
- 反编译视图不再有 `L1 (structured)` / `L2 (vars)` 复选框。它总是渲染带恢复签名、参数名和栈槽的结构化伪 C（原先的 L2 输出），对于已分析函数之外的地址回退到原始列表。被移除开关的两个语言键也删除了。
- 单行条件语句去掉了生成器多余的括号：`if ((eax == 0)) break;` 现在读作 `if (eax == 0) break;`，`continue`、`return ...` 和 `goto loc_...` 同理（`if`/`while`/`do-while` 块头早已如此）。Notepad 列表减少 1065 对这样的括号。
- 被调用消费的参数寄存器设置不再打印两次。直接调用已经渲染被调用者的参数，间接导入调用（`call [IAT]`）现在会打印最后写入 `rcx`/`rdx`/`r8`/`r9` 的值，因此当这些副本的输入在调用前没有变化（且没有存储使内存读取失效）时，前面的复制会被丢弃；`xor edx, edx` 之类的自抵消形式算作常量。在 notepad 上列表减少 2525 行设置（6585 → 4060），例如 `GetProcessHeap(); HeapFree(rax, 0, rbx);` 而不是两次额外的寄存器复制。
- 唯一语句是嵌套 `if` 的 `else` 分支现在渲染为 `} else if (cond) {`，分支体左移一层，因此 else-if 阶梯读起来像源码，而不是每层分支多一级嵌套。Notepad 列表合并了 126 个这样的分支。
- 唯一出口是一条 `if (condition) break;` 的 `while (1)` 循环现在把该测试显示为循环条件：在函数体顶部它变成 `while (!condition)`，在末尾则变成 `do { ... } while (!condition)`。在 notepad 上，83 个无条件循环中有 47 个变成真正的条件，例如一次表扫描读作 `while (rbx < rdi) { ... }`。函数体中的 `continue` 会让循环保持不变，因为它会在测试之前到达 latch。
- 签名现在会跟随通过直接调用转发的参数。调用被视为消费被调用者的参数寄存器（被调用者自身的签名通过最多四层包装深度优先恢复、带记忆化，每次反编译只做一次函数范围查询），因此 corpus `call_str` 渲染为 `int32 call_str(int64 arg1)` 并带 `str_probe(arg1)`，而不是 `void` + `str_probe(rcx)`；notepad 中 `sub_1400030e8` 之类的包装器会显示它们透传的四个寄存器。未建模指令会报告它们写入的寄存器，因此 `cpuid` 和 `sbb edx, edx` 不再凭空造出参数：在 notepad 上，502 个签名中有 40 个变化 —— 33 个获得它们转发的寄存器，2 个去掉这样的幽灵参数，其余得到更精确的指针类型。
- 跳转表函数不再两次显示编译器的分派：恢复出的 `switch` 之前的表地址加载和结尾的 `__asm("jmp rcx")` 会被丢弃（这些语句被 `switch` 本身替换，每个 case 的函数体现在是折叠后的 `return <value>;`）。只有当该基本块仅由以间接跳转结尾的机器寄存器运算组成时，清理才会触发，因此与它共享基本块的调用或内存存储仍可见。
- 反编译出的伪 C 现在返回代码计算出的值，而不是把它留在寄存器里：`eax = f(); return;` 渲染为 `return f();`，`eax += x; return;` 渲染为 `return (eax + x);`，而产生者不是相邻语句的有值返回路径会命名返回寄存器（`return eax;` / `return rax;`），而不是裸 `return;`。紧挨返回上方写入的单次使用临时值会内联进去，因此 `eax = a; eax += b; return;` 变成 `return (a + b);`。void 函数保留裸 `return;`，寻址缩放遗留的乘以一（`(arg2 * 1)`）会被丢弃。在 corpus 上，`add_leaf` 现在读作 `return (arg1 + arg2);`。
- 在反编译视图中，双击被调用者的名称现在会进入该函数（其伪 C 替换列表，`Back` 返回调用者），而不是跳到指针下的反汇编行。被调用者名称按反编译器自己的命名规则解析（存储名称或 `sub_<hex>`），被调用者工具提示和手形光标提示可点击，而对其他任何东西（局部变量、关键字、当前函数自身的名称、行号栏）的双击仍然跳到反汇编。重命名函数现在会立即刷新 Functions 标签，因此新名称无需重新分析即可解析。
- 反编译视图现在在固定的左侧栏中显示 IDA 风格行号，而不是地址前缀；代码水平滚动时该栏保持不动。单击某行会选中该行并同步反汇编光标；列表在单击时不再重新锚定/重新渲染，过去会一次高亮多行的地址组现在只高亮被单击的行（或当选择来自其他视图时高亮其第一行）。悬停某行会显示其地址。

- `e7da3b6` 上的最终验证：156 个测试、全部五个快照集零不匹配（785 个函数）、四个样本 lint 干净、工作树干净。
- 序言移动 `esp` 之后重新读取参数的 x86 函数过去会得到两个同名参数（`nested_loops(int32 arg1, int32 arg1)`）；现在参数按名称去重，得到与 x64 一样的 `nested_loops(int32 arg1)`。
- 分析 pass 不再格式化它丢弃的语句。`lift_block` 新增 `render` 标志：第一趟只需要基本块的效应（写入、调用、栈使用、参数复制），因此它为 Notepad 最大的函数跳过了约 37 000 次语句渲染。发出的文本逐字节相同；本机墙钟变化在运行间噪声范围内（45 KB 函数两种方式都是 1.6 s），因此应把它视为减少的工作量，而不是测得的加速。
- 反编译出的伪代码现在读起来更接近源码：`x = (x + n);` 变成 `x += n;`，`r = *(T*)(a); r -= k; *(T*)(a) = r;` 折叠为 `*(T*)(a) -= k;`，`if (!(x == 0))` 变成 `if (x != 0)`，整个条件外多余的包装被去掉（`if ((x == 0))` 变成 `if (x == 0)`），只写一次读一次的临时值内联进其读者。取反辅助函数过去使用会互相抵消的链式字符串替换（`==` → `!=` → `==`），因此相等分支保留了多余的 `!`；现在它只翻转一个比较，并让 `&&`/`||` 条件保留显式 `!` 包装。
- L1 结构化恢复更多形态，且绝不发射指向缺失标签的 `goto`。共享尾部现在最多 48 行，并可包含内部标签（标签不会被复制），连续相同的终止分支合并为 `if (a || b) { … }`（编译器的短路 `||` 降级），当另一条路径跳到循环头时循环头会获得标签。仍有活跃向前 `goto` 命名的标签在死尾清理时会被保留，因此没有跳转悬空。在 notepad 上这测得 **487/502 结构化（97.0%）**，覆盖率不变为 99.4%；corpus x64 保持 31/31 和 97.4% 覆盖率，golden 快照因新的语句形态而重新生成。
- 经导入表的间接调用会渲染为导入的 API，而不是指针表达式：`(**(u64*)(0x...))()` 变成 `CreateFileW(...)`。具名导入的 IAT 槽位映射到函数名（清理为 C 标识符），调用前写入 `rcx`/`rdx`/`r8`/`r9` 的参数按 ABI 顺序渲染（栈槽地址用 `&var_XX`）。`jmp [IAT]` 尾调用渲染为 `return Api();`。在 notepad 上 1667 个间接调用中有 1657 个被命名，大多数携带其捕获的参数。
- 传递局部变量地址的调用参数现在打印为 `&var_XX`，而不是原始的 `(rbp - 0x30)` 指针表达式，直接和间接调用都是如此；参数寄存器的全宽复制（`rsi = arg1`）会让后续读取保留参数名。
- 当溢出槽位只被保存和恢复时，被调用者保存簿记会被移除：一次保存可以配对多次恢复，而从未被恢复的保存算作死。在 notepad 上伪代码缩减约 800 行，大多数函数现在以第一条真实语句开始，而不是 `var_8 = rbx; ...`。
- 重建自己帧指针的函数（`rbp = rsp - 0x27`）现在也会把正的 `[rbp+off]` 访问命名为局部变量，并且在同一直线段中稍后被覆盖且无读取的栈存储会被移除。
- 栈槽命名把 `frame = rsp` 别名当作 `rsp` 处理，因此通过它进行的溢出变成 `var_XX`，并像其他被调用者保存对一样被隐藏。
- 小的符号扩展常量打印为负字面量（`or rax, -1` 而不是 `or rax, 0xffffffffffffffff`，`0xfffffffffffffff8` 掩码用 `and r9, -8`），且 `x |= -1` 折叠为 `x = -1`。
- 直接通向 `return` 的被调用者保存恢复会作为调用约定簿记被丢弃，序言中复制进被调用者保存寄存器的参数（`rsi = arg1;`）在该寄存器再未被赋值时，会在之后每次读取时替换为参数名。在 notepad 上这移除了约 2,250 行尾声噪声，并让整个 L2 输出中的 `*(u16*)(rsi)` 变成 `*(u16*)(arg1)`。
- 目标基本块发射时没有标签的 `goto` 现在会在语句地址处具体化该标签，因此在结构化器把单前驱基本块内联发射的样本（Go 和 kernel32）上，生成的伪 C 不再引用缺失的标签。
- 越过已分析函数末尾的控制转移是进入下一个函数的尾跳转，因此它渲染为 `return sub_xxx();`，而不是跳到外部基本块的 `goto`。这消除了 kernel32（现在 2567 个函数中 0 个引用缺失标签）和 Go 上剩余的悬空跳转。
- L2 签名会填补已恢复的 x64 参数寄存器之间的空隙：读取 `rcx` 和 `r8` 的函数声明为 `arg1, arg2, arg3`，而不是悄悄省略未使用的 `rdx` 参数，因此参数列表与调用约定一致。
- 死尾清理现在会把嵌套的 `if { … }` 区域整体移除，而不是在其右花括号处停止 —— 后者过去会留下孤立的 `}`（3 个 kernel32 和 11 个 Go 函数）。区域中若有仍被外部跳转命名的标签则保留，因此该修复不会重新引入悬空跳转。kernel32 现在 2567 个函数中 0 个有括号不平衡或缺失标签。
- 移除了离线/启发式 AI 模式。`ProviderKind::Heuristic` 和 `HeuristicProvider` 已删除；GUI 的 AI 操作（explain/suggest/query/polish）和相应 CLI 命令现在在配置 LLM provider 之前拒绝运行。provider 设置面板只提供 DeepSeek、OpenCode Go 和自定义 OpenAI 兼容端点，未配置的流式调用会返回明确的“no LLM provider configured”错误。
- AI 润色现在会流式处理 OpenAI 兼容的 SSE 响应：HTTPS provider 发送 `stream: true`，解析器处理 `delta.content`（包括跨 TCP 读取拆分的块和跨读取拆分的 UTF-8 字符），AI 结果窗格实时追加文本并自动滚动。本地纯 HTTP 传输和不支持 SSE 的 provider 回退为单个最终块。其他 provider 遵循配置的完成预算；OpenCode Go 润色禁用思考（`{"thinking":{"type":"disabled"}}`）并把完成限制在 8k token，因为推理路径把首个答案延迟了 57–111 秒。仅推理的 OpenCode 流现在会产生可行动的 error（推理字符数 + 非推理模型建议），而不是通用的空流失败。重复的润色请求按端点/模型/预算缓存；AI 头部会显示已用秒数和调用运行时的首 token 等待状态。
- 反编译出的调用点现在显示推断出的参数，而不是裸 `sub_xxx()`：被调用者序言按目标扫描一次（跟随导入 `jmp` thunk），调用捕获的参数设置写入被省略，没有可恢复签名的调用回退到调用者的计算槽位。连续重复写入、`int3`/`nop` 填充、相邻的 `goto`/标签对和未被引用的标签会从 L1/L2 输出中移除；未使用的被调用者保存溢出对也会隐藏。`call_sites_show_arguments_with_a_callee_prototype`、`tidy_labels_removes_adjacent_gotos_and_orphans` 和被调用者保存测试覆盖新行为。
- 工具栏的 Text/Graph 对现在是一个目标模式切换：在图模式下它提供 文本/Text，在文本模式下它提供 图形/Graph。Space 快捷键和现有视图行为不变。这在保持模式切换可发现的同时去掉了重复的分段控件。

### 修复

- **中英混排的文字现在落在同一条基线上。** egui 的字体回退按字形换字体，并把回退字体的
  行盒在整行内垂直居中，而不是对齐基线；Consolas 的 ascent（0.74 em）远小于微软雅黑
  （1.06 em），所以等宽文本（输出窗口 11 px、反汇编/十六进制 12.5 px、状态栏）里的中文
  一直比英文低约 3 px。等宽族的中文回退现在单独使用一份带
  `FontTweak { y_offset_factor: -0.25 }` 的字体（与比例族共享同一份字体数据，不重复占内存），
  这样中文和英文的基线在 11–13 px 以及列表缩放的各个档位上都对齐；新增
  `cjk_monospace_baseline_matches_latin` 作为护栏。底部状态栏也从混排的 12.5 px 地址 +
  13 px 函数名/提示改成整条统一等宽，不再出现基线相差 3 px 的“两行字”。

- **未加载项目时，反汇编标签页（承载欢迎页及其打开/导入提示）现在始终位于最前面。**
  欢迎页位于反汇编窗格内部，因此从 `layout.json` 恢复的文档标签页 —— 或被 `Ctrl+W`
  留在前面的标签页 —— 可能把它藏在空的“反编译/十六进制”标签页后面，导致窗口内唯一
  打开项目的途径消失。当应用不带路径启动、关闭项目以及打开失败时，该标签页会被置于前台；
  用户之后选择的标签页保持不变，已打开的项目永远不会被覆盖。未加载任何内容时，反编译窗格
  也会显示“请先打开项目”，而不是“该地址没有反编译结果”。

- **带两个出口的循环不再丢失“搜索耗尽”边。** 当循环体在 latch 处结束时，循环实际上在
  latch 的 fallthrough 处继续，但发射器把头部的提前退出当成了延续，于是 fallthrough 边
  消失了：在 `sub_14000647c` 中，`while (1) { … }` 搜索循环的耗尽路径落入了“已找到”的代码，
  而不是释放路径（死循环 / 丢失代码）。发射器现在优先采用 latch 的出口，并把提前退出的
  直线链**复制**进由 `copy_linear_chain` 生成的分支（`if (cond) { …; break; }`，节点分裂），
  因此两个出口都保持结构化 —— 没有 `goto`，循环之后也没有标签。当这条链无法安全复制时
  （条件子树、共享基本块、超过 8 个基本块），保留旧形态不变，因此结构化率与之前完全相同
  （crackme 460/495，notepad 494/506）；4 个快照迁移到新形态（`sub_14000647c`、
  `sub_140020260`、`sub_1400040e8`、`sub_14000627c`）。

- **残留基本块不再从所属区域掉进接下来打印的任意内容中。** 当循环体或区域在主线遍历之后
  发射一个基本块时，该基本块的 fallthrough 目标往往不再是下一条语句；发射器却什么也没打印，
  于是控制流悄悄落入不相关的代码（缺失分支：`sub_140006d38` 丢失了它的 `if (eax != -1)`
  守卫，`sub_140005d70` 和 Go 的 `internal/abi.NewName` 丢失了一次跳转）。残留列表现在会
  传入真正跟随其后的基本块，并在它不是该基本块时把 fallthrough 显式写成 `goto loc_…`。
  结构化率不变（crackme 460/495，notepad 494/506）；三个快照补上了它们缺失的跳转。

- **共享尾部复制不再改写条件跳转。** `split_shared_tail_into_branches` 的多引用版本丢失了
  “仅限无条件跳转”的过滤，于是 `if (c) goto L;` 被算作一次引用，`L` 处的尾部被复制覆盖了它 ——
  丢掉了条件。在 crackme `sub_14000391c` 上，这把
  `if ((*(u8*)((rbx + 0xc)) & 1) != 0) goto loc_…;` 变成了一条无条件的 `eax = 1;`。现在引用被
  跟踪两次：`references` 保存每一次跳转（只有它们全部被折叠时标签才能被丢弃），
  `unconditional` 保存可以被复制的普通 `goto loc_…;` 行。crackme 的结构化率从 462 降到 459，
  notepad 从 495 降到 493：那五个函数是被错误的折叠“改进”的，现在恢复了它们真实的形态。
  十二个快照重新获得了它们的条件跳转；五个项目的 lint 均为 0 发现，789 个 golden 快照通过，
  220 个单元测试通过。

- **通过其出口离开循环的 `while (1)` 循环体现在会写 `break`。** 当函数体的线性链到达循环的
  出口基本块时，发射器直接返回而不写任何内容，因此生成的 C 在右花括号处又循环回去 ——
  源码中的退出变成了又一次迭代。现在当 fallthrough 是以 `while (1)` 发射的循环的出口时，
  `emit_linear` 会写出 `break;`；其他目标保持旧形态，因此不会出现新的 `goto`。六个快照发生
  变化，每个都补上了缺失的 `break`：crackme `sub_140002688`，notepad
  `sub_1400052e0`/`sub_1400081ac`/`sub_1400101d8` 和 Go 的 `internal/abi.NewName` 各增加一行，
  crackme `sub_140001ce0` 恢复了真实的 if/else 形态（+24/−17 —— 它之前是个无限循环）。
  结构化率和 goto 计数不变（crackme 455/495，notepad 487/506），五个项目的 lint 均为 0 发现，
  789 个 golden 快照通过。


- **用户原型中的 `...` 不再被解析为参数。** 给可变参数辅助函数命名
  （`int printf(const char* format, ...)`）会添加第二个默认类型的参数，于是每个调用点都多出
  一个源码从不传入的参数（`printf((char*)"Enter flag: ", v3);`，其中 `v3` 是调用点遗留的
  `rdx`）。这个幽灵参数还把 `rdx` 拉进值流，并把验证循环的字节更新展开成一长串掩码表达式
  （`v3 = (((...) & 0xffffffffffffff00) | ...)`）。现在会跳过该标记，原型只保留一个参数，
  调用读作 `printf((char*)"Enter flag: ");`，循环回到 `v13 = …; v13 ^= …; v13 += …;`。
  演示脚本的原型表也对照挑战的真实源码（`src/crackme.c`）做了修正：`0x140001170` 是
  `printf`（不是 `puts`），`0x140004cc4` 是 `puts`（不是 `printf`），`0x140006fd0` 是
  `strpbrk`（不是 `strchr`）。新增一个单元测试；五个项目的 lint 均为 0 发现，789 个 golden
  快照不变。

- **折叠后的常量不再到达定义并不支配的路径。** `fold_constant_registers` 可以把寄存器的最终常量折叠进嵌套基本块（映像基址就是这样到达循环体的），但它在定义所在基本块闭合时没有停止：在 `if (rax == 0) { eax = 1; return; }` 之后，存储 `*(u8*)(rax) = 0;` 变成了 `*(u8*)(1) = 0;`，notepad 的 `sub_140010798` 因为一次常量返回丢掉了一整个 `if`。现在扫描在深度低于定义时、或在其层级出现 `return`/`goto`/`break`/`continue`/`} while` 时结束，因此折叠只发生在定义支配使用的地方。二十六个 notepad 和十个 crackme 快照恢复为正确的寄存器读取（`*(u64*)(0)` → `*(u64*)(rax)`、`eax = 0x…` → `eax = ebx`）；结构化率和 goto 计数不变。常量折叠之后的第二次死写清扫还会移除它们留下的参数设置（`ecx = 0; call(0)` → `call(0)`），一个单元测试覆盖该支配规则。
- **条件语句的 fallthrough 边不再被假定为列表中的下一个基本块。** 扁平回退按地址顺序发射循环体，因此当某基本块的 `not_taken` 后继不是接下来那个基本块时，发射的文本就落进了错误的代码：`crackme main`（`sub_140001010`）在循环的耗尽路径上打印了“Correct! Nice reversing.”，而那里本应是提示。`emit_raw_flow_next` 接收接下来将要发射的基本块，只要它与 fallthrough 后继不一致就把该边显式写成 `goto loc_…`（扁平区域传入前瞻；残留发射器保留隐式 fallthrough）。`fold_return_tail_labels` 得到匹配的守卫：返回尾部在其第一个 `return;` 处停止，也在另一个跳转命名的内部标签处停止，而不是把下一个基本块的代码复制进每个跳转点。两者结合后，crackme `main` 从 **17 个 goto 降到 1 个**，变成一个内含 32 字节验证循环的 `do { … } while (tries < 3)` 循环；crackme 标签 218 → 201，notepad 结构化 489 → 487（两个扁平回退函数现在各带一个诚实的 `goto`），标签 269 → 268，corpus/corpus86/go 不变。`lint` 在五个项目上均为 0 发现，受影响的 golden 快照已刷新（检查 789 个），反编译器新增两个针对尾部守卫的单元测试。
- **`call qword ptr [IAT]` 不再结束函数扫描。** 分析遍历过去把间接调用当作死胡同，于是每个函数都在第一个导入 API 处停止，其后的基本块、调用和 API 标签都缺失：notepad 只保留 448 个 API 标签，命名证据常常是每个函数一个 API。现在间接调用会落到下一条指令（它会返回），这使 notepad 升到 **1 637 个标签**，并发现 **四个函数** 此前不可达（502 → 506；结构化 489/506，96.6%）。刷新了六个 golden 快照：两个变化的快照获得此前被丢弃的调用参数（`sub_140013154` 的栈偏移整体移 8），corpus86 `indirect.c` 补上缺失的 `return;`，notepad 的四个新增是新函数。`lint` 在 notepad/crackme/corpus/corpus86 上保持 0 发现；golden 检查在 506 + 32 + 160 + 31 个快照上干净。
- **重命名现在会拒绝会破坏列表的名称。** `Command::Rename` 过去接受任意文本：重复符号让两个地址无法区分，C 关键字让伪代码无法解析 —— 可从 `rename`、`ai apply`（提议 JSON）和模型回复触发。该命令现在校验 C 标识符（ASCII，≤ 64 字节），拒绝保留字（C11/C17 以及语料可能发射的 C++ 拼写），并拒绝另一个地址拥有的名称；重新应用地址自身的名称仍是空操作。一个测试覆盖空、非法、关键字、重复和自重命名情形。
 `scroll_to_rect(Align::Center)` 会居中两个轴，而目标是整条已分配的行 —— 比窗格更宽，因为视图至少保留 1100 pt 的代码 —— 因此重新居中光标行也会把列表（及其行号）推到右边，而通过单击标签打开视图时却不会。现在滚动目标横跨可见宽度，因此行在垂直方向居中而水平偏移不受影响。一个测试渲染相同的几何（360 pt 视口中的 1100 pt 行）并断言垂直居中且 `offset.x == 0`。
- **MCP 的 `filter` 现在在 `limit` 之前运行。** `list_functions` 和 `list_strings` 过去在扫描时应用行数上限、之后再过滤，因此默认 500 行窗口之外的匹配会返回 `count: 0`：在 Go 样本上 `filter: "main"` 什么也报告不出来，notepad 的 `filter: "%"` 在 67 个字符串中只返回 3 个。两个工具现在过滤整张表、只限制返回行数，并报告 `total`/`truncated`，因此不会悄悄丢弃内容；`xrefs_to` 和 `xrefs_from` 获得同样的 `limit`/`total`/`truncated` 报告。三个测试覆盖晚匹配、截断和 xref 情形。
- **`get_pseudocode` 会限制其响应。** 一个 L2 函数（notepad `sub_140013154`）渲染出 9 855 行 / 1.5 MB，超出大多数 MCP 客户端接受的单次工具结果。该工具现在默认 `maxLines: 2000` / `maxBytes: 256 KiB`（可提高到 100 000 行 / 8 MiB），并报告 `truncated`、`total_lines` 和 `hint`，而不是撑爆客户端。
- **MCP `initialize` 协商协议版本并携带 `instructions`。** 当客户端的 `protocolVersion` 是 `2024-11-05` 或 `2025-06-18` 时会回显它（否则回退），并返回一个说明只读契约和截断约定的 instructions 字符串。
- **MCP HTTP 令牌检查是常数时间的**（对 `Bearer` 头使用 `bytes_equal_ct`），而不是会短路的 `!=`，因此无法从响应时间推断令牌错误；`callgraph` 现在在节点列表（而不只是边列表）被截断时也报告 `truncated: true`。`docs/MCP.md` 记录了新字段、上限和版本协商。
- **在反汇编视图中滚动字符串窗口会跟随地址空间。** 以存储字符串为锚点的窗口把它显示为数据行，但翻页却经过 `Session::rewind`/`advance`，它们按指令边界行走：从字面量向上滚一格会落在它*之前*的填充里（notepad 的 `.rdata` 字面量之间由会被解码为 `add [rax], al` 的零字节分隔），于是下一次刷新把字面量当作指令解码 —— 也就是报告中的“双击一个字符串、向上滚，字符串消失了，整个视图变成反汇编”。改为对字面量列表翻页保留了数据，但会停在第一个字面量处，于是视图再也无法向上滚动（第二个报告）。现在列表是对地址空间的一次行走：可执行段中是指令，存储字面量是 `db '…',0`，它们之间的字节是原始 `db` 行，并跨节边界延续。`ReCraftApp::data_rows` 渲染这次行走，`step_listing` 为滚轮翻页（`extend_disasm_window`）、PageUp/PageDown 和窗口边缘的方向键服务；`string_row_at` 是二分查找，因此逐行查找保持廉价。原始行保留名称和 `; DATA XREF` 注释，未映射的起点仍显示空视图。回归测试 `scrolling_a_data_window_follows_the_address_space`（notepad：向上一行是渲染为数据的填充，滚轮走到节首并跨入 `.text`）、`scrolling_up_from_the_first_byte_of_a_section_crosses_into_the_previous_one`（corpus：其第一个字面量就在节首）和 `headless_wheel_scrolls_a_string_window_up`（端到端复现所报告的手势）钉住该行为。
- 用户设置的原型现在对参数列表具有权威性。函数体在其之后读取的栈槽保留 `var_…` 名称，并在
  `// stack args (not in the prototype): var_b0 (incoming #5, int64*)` 注释中报告，而不是重写签名：`int entry_proto(void)` 过去会变成 `int32 entry_proto(int64 arg1, …, int64* arg5)`，而带具名参数的原型（`int f(int a, int b)`）后面会跟着重复的 `arg1…arg5` 条目。推断的签名（无原型）仍会加宽到最高的栈参数，因此反编译基线不变（notepad lint 在 1004 段文本上 0 发现，golden 502/502，结构化 487/502）。
- `scripts/verify-package.ps1` 过去查找旧的 `// L1 structure recovery` 横幅，因此一旦反编译器打印 `// L2 structure recovery`，打包二进制检查就失败了。它现在断言当前标题，`PACKAGE PASS`（zip 哈希、条目、import/analyze/decompile/AI/metrics）在重命名后的 `recraft-cli.exe` 上重新变绿。
- 在反编译视图中双击字符串字面量现在会打开它命名的字符串数据（反汇编中的 `db '…',0`），而不是加载它的指令。伪 C 渲染器记录每个带引号字面量及其引用的地址，因此视图可以跟随字面量本身；出现在多个地址的相同文本由被单击指令的 xref 消歧。数据视图还能处理指向更长存储串内部的字面量（编译器字符串池：`"O====…"` 串中的 `"===="`）以及低于字符串提取下限的短字面量（`"\r\n"`、`")"`），这些回退到反编译器记录的带引号字面量。
- 单行分支中条件周围的多余括号也会被去掉：`if ((al == 0)) return;` 现在读作 `if (al == 0) return;`。该 pass 按结构匹配右括号，而不是假定 `{` 终止符，真正需要的一对（`if ((a) && (b))`）则保持原样。
- 只是裸寄存器的调用参数不再变成栈槽的地址：一旦 `rbp` 被写入，`helper(rbp)` 过去会打印 `helper(&var_0)`，因为裸 `[rbp]` 访问和指针值被同等对待。现在只有真正的栈槽地址才采用 `&var_XX` 形式。
- 供给调用的栈存储（第 5 个及之后的参数位于栈上）不再被当作死的被调用者保存溢出或死局部变量丢弃：`RtlVirtualUnwind` 的传出 `var_20`/`var_28`/`var_30` 槽位会保留，保存的寄存器只有在没有调用能读取其槽位时才被视为已死。通过保存的 `rsp` 副本到达的槽位命名为 `frame_XX` 而不是 `var_XX`，因此不再与当前帧的局部变量冲突。
- 当调用破坏了保存 `rsp` 的别名寄存器时，该别名会退役，因此后续 `test rax, rax` 读作 `if (rax != 0)`，而不是错误的 `if (frame != 0)`。
- 当后续语句仍读取其寄存器时，加载-修改-存储折叠不再丢弃加载的值，最明显的是返回新值的计数器更新：corpus `corpus_counter_bump` 过去渲染为 `*(u32*)(...) += 1; return;`，丢失了返回的计数器，现在保留加载并以 `return eax;` 结尾。
- 为携带语句的循环头发射的 `while (1)` 循环（结构化器在测试无法放在 `do-while` latch 上时的回退）过去在*留在*循环中的条件上中断，而不是其否定，因此打印出的循环在机器继续时离开、在机器离开时继续。回退现在对 fallthrough 条件取反：notepad 的 `cmp rdx, r9; je exit` 读作 `if (rdx == r9) break;`，而不是 `if (rdx != r9) break;`。
- `do { ... } while (...)` 条件不再保留生成器多余的括号：`while ((ecx != 0));` 渲染为 `while (ecx != 0);`，与 `if`/`while` 已有的清理相同。
- 算术指令之后的分支现在测试该指令产生的值。`sub ecx, 1; jne` 过去打印 `ecx != 1`（写入前的操作数，读取时会再减一次），而 `shr ecx, 1; jne` 复用了更早 `add` 的标志，因此循环条件可能比较错误的寄存器，打印出的循环也不必像机器那样终止。提升器现在为 `add`/`sub`/`and`/`or`/`xor`/`inc`/`dec`/`shl`/`shr`/`sar`/`neg` 记录目标寄存器作为标志来源：`sub ecx, 1; jne` 现在读作 `ecx != 0`，`shr ecx, 1; jne` 读作 `ecx != 0`，corpus `count_bits` 以 `while (ecx != 0)` 结尾，与其源码一致。算术之后的无符号条件（需要 CF，无法针对结果表达）回退到现有的“未建模标志”汇编，而不是打印错误的条件。
- 反汇编视图现在可以向上滚过其已加载窗口。窗格渲染固定的 400 条指令窗口，因此其滚动区只能在该窗口内移动：在上边缘滚轮什么也不做，更早的代码无法到达。当滚轮在顶/底边缘继续转动时，视图现在会向上/向下加载另一页（50 条指令），并按新增行的像素高度补偿滚动偏移，因此屏幕上的文本保持不动。用 `PageUp`/`PageDown`/`ArrowUp`/`ArrowDown` 翻页过去按 `rows * first row length` 字节步进，会落在变长指令中间并解码出垃圾（还可能走到映像之下并把视图清空）；现在步进通过新的 `Session::rewind`/`Session::advance` 使用真实指令边界（从最近的分析函数起点开始扫描，包含 `int3` 填充），保持光标行并在段首停止。`G`、follow、后退/前进和十六进制标签同步现在会重置滚动位置，因此跳转到的地址确实可见。
- 在 egui 运行第二趟布局（`request_discard`）的帧上，键盘快捷键和翻页不再触发两次：过去一次按键会把反汇编滚动两页。
- 内置 DeepSeek provider 预设现在提供在线模型 id `deepseek-flash` 和 `deepseek-v4-pro`。此前的 `deepseek-chat` / `deepseek-reasoner` id 已于 2026-07-24 退役且不再解析，因此预设默认到了一个 API 拒绝的模型。仍指向退役 id 的已保存设置在加载时迁移到 `deepseek-flash`，文档列出了新 id。
- AI 润色（以及其他 AI 操作）不再在 DeepSeek 官方端点上等待约一分钟才拿到第一个内容 token：请求现在要求直接回答（`{"thinking":{"type":"disabled"}}`）。DeepSeek V4 默认以高 effort 启用思考，其推理过程延迟了输出。`PromptRequest` 携带该选择（默认直接回答，`with_thinking()` 可重新开启）；流式和非流式请求构建器都遵循它，prompt/缓存键也包含它，因此快速回答与推理回答不会冲突。
- 切换 AI provider 现在会一起更新端点、模型和 API 密钥字段：退出的密钥按 provider 记住，匹配的密钥被恢复，未知 provider 以禁用状态和空密钥字段开始。密钥环境变量名重置为默认值，prompt 缓存失效，原始 Custom 端点编辑使用同一份密钥记忆。选择已经激活的 provider 不会改动当前密钥。
- provider 状态现在会点名所选预设（`deepseek`、`opencode-go`、`openai-compatible`、`ollama`），而不是打印共享传输名 `openai`。
- 反编译器结构化现在会复制以填充/外部尾调用结尾的终止基本块，而不是留下跳到未发射标签的跳转；丢弃无条件 `return` 之后的直线代码；并折叠到仅返回标签的跳转。有多个前驱的共享基本块在首次发射时获得标签，因此生成的 `goto` 不会指向缺失的标签。当较早的分支消费了后支配汇合点时，短的无环分支现在会被复制，而不是回退为原始跳转；跳到最后一个带标签的短尾部的跳转会复制该尾部加上函数尾声。尾部扫描现在把 `} else {` 识别为右花括号，跳到任何平衡尾部以 `return`/尾调用结尾的标签也会复制该尾部。最后一个标签的清理尾部现在可以是大型平衡的 `if`/赋值区域，而不只是直线语句。`return` 之后的死代码还会丢弃孤儿标签，忽略只出现在死尾部内部的引用，并暴露其余不可达尾部。一个兄弟分支末尾跳到其兄弟的短直线尾部的跳转会被复制并移除；分支末尾的条件跳转变成 `if (cond) { tail }`。复制的尾部现在按每个花括号恢复一级缩进，而不是把嵌套函数体压到单列。在 notepad 语料上，这把结构化函数从 432/502（86.1%）提升到 484/502（96.4%），同时保持指令覆盖率 99.4%；为死尾部、孤儿标签、兄弟尾部、平衡 if 和最后尾部清理补充回归测试。

- Windows 桌面二进制现在链接 GUI 子系统，因此启动 `recraft-ui.exe` 不再闪现/保留黑色控制台窗口。单元测试保留控制台，以便其输出仍可见。
- 折叠 AI 侧栏会让活动文档保持不变。助手只存在于侧栏，因此不会添加或聚焦任何停靠标签；回归测试：`collapsing_ai_sidebar_keeps_the_active_document_tab`。
- 十六进制视图现在遵循 IDA 的转储布局：16 位地址不带 `0x`、紧凑的两位字节（大写）、没有逐字节偏移头，ASCII 尾部紧跟在字节之后。字模式保持相同的紧凑网格。
- 反编译视图文本现在使用活动调色板的前景色，而不是硬编码的 `LIGHT_GRAY` —— 后者在白色亮色主题上几乎不可见。代码文本和绘制回退都遵循 `pal.text`；已用亮色主题截图验证。
- 当多个失败检查共享一个出口基本块时，L2 结构化不再丢弃循环体的其余部分。共享出口会发射为带标签的跳转，而不是在第一个检查处内联。对于内层出口离开父循环的嵌套循环（crackme 的 `for` 式验证循环），函数现在回退为带标签的扁平输出，而不是把成功路径放在提示之后；一个回归测试覆盖 `nested_loop_leaves_parent`。
- 润色现在允许模型在调用形态唯一确定时使用规范的 C 运行时名称（`fgets`、`strcmp`、`strlen`、`strpbrk`、`printf`/`puts`、…），为被引用的表发射自包含的 `extern` 声明，并从 provider 回复中去掉 Markdown 代码围栏。此前即使参数让 CRT 函数显而易见，它也会保留每个 `sub_XXX` 调用，因此润色后的代码看起来不那么像源码。
- 润色现在把**权威反汇编**（地址、字节、助记符和分支目标）与 L2 一起喂给模型，并指示模型不要信任伪代码的块顺序。这修复了把 32 轮检查提升到输入循环之外、或把提前退出与成功路径合并的输出。在 crackme 上重新运行后，验证现在保持在 `while` 内部，`wrong:` 在循环内，提示在循环之后。
- OpenCode Go / 推理模型的润色现在端到端可用：提示使用 **L2**（局部变量、字符串、调用名）并带明确的源码重建指令，润色预算默认 **65536** token（推理在答案之前消耗大部分；`deepseek-v4-pro` 需要完整的 64k），空完成现在会报告可行动的“raise RECRAFT_AI_MAX_TOKENS”错误。HTTPS 错误会包含响应体（前 800 字符），而不只是状态码。已用真实 OpenCode Go 密钥和 `deepseek-v4.1-flash` 验证；生成的源码记录在 `docs/cases/crackme_win_x64.md`（4b 节）。
- AI 面板现在可垂直滚动，把请求状态和润色结果显示在 provider 设置**上方**，并在后台润色作业运行时请求重绘。此前缓慢/较大的响应可能被裁到面板下方，或只在无关的鼠标/键盘事件之后才出现（报告为“Polish 什么也不做”）。新增 `ai_job_result_updates_status_and_explanation`。

### 移除

- **NSIS 安装程序**（`packaging/installer.nsi`、`dist/recraft-<version>-setup.exe`、
  `scripts/verify-installer.ps1`、CI 的 `installer` 作业以及 `-NoInstaller` /
  `-RequireInstaller` 开关）。发布两个安装程序意味着同一个产品有两套来源、两套校验脚本、
  两个 CI 作业和两段文档；MSI 覆盖已安装场景（带 `.rcraft` 关联、App Paths 和标准 MSI
  卸载/升级），便携 zip 覆盖免安装场景。随它一起消失的唯一能力是每用户、无需提权的安装，
  因为 Windows Installer 会在管理员账户上把它们提升为每机器安装。
