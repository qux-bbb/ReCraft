# 质量门禁与 golden 快照（DESIGN §7.4）

反编译质量用两个可自动测量的指标 + 一组文本快照守住；数据来自已分析项目中的
`func` 表与 L1 发射器。

## 指标定义

| 指标 | 定义 | v1 目标 |
|---|---|---|
| **语义覆盖率** | 1 − 包含 `intrinsic`（`__asm("...")`）的指令数 / 总指令数 | ≥ 95% |
| **结构化成功率** | L1 输出中**不含** `goto loc_*` 回退的函数 / 总函数数 | ≥ 70%（M2 退出线 ≥ 60%） |
| goto 回退函数数 | 结构化失败数量 | 越少越好 |

说明：

- 语义覆盖率按"指令是否完全建模"计；`push/pop/imul/cmov/indirect jmp` 等目前落入
  `intrinsic`，所以真实语料上覆盖率是**下界**，随 §7.2 语义表扩充而上升。
- 结构化成功率只看控制流发射：`goto` 回退意味着结构器无法证明 if/loop/switch 形状
  （不可约 CFG、复杂循环头、case 体标签等）。
- 两个指标都在 **corpus x64** 上跑，作为 CI 门禁；真实样本（notepad/kernel32）用于
  本地观察，不设阈值（ground truth 不完整）。

## 命令

```powershell
recraft metrics <project> [--json]
recraft metrics <project> --min-structured 0.6 --min-coverage 0.75   # 门禁
recraft golden <project> --dir tests/golden/corpus                    # 写入快照
recraft golden <project> --dir tests/golden/corpus --check           # 对比快照
```

`metrics` 与 `golden` 都要求先 `import` + `analyze`。

除了指标与快照，每次改动还会把真实样本的**每个函数**在三种模式下各反编译一遍
（`decompile --addr <f>`、`--structured`、`--l2`），确认没有 panic/内部错误，并检查
花括号是否配平、每个 `goto loc_...` 是否有对应标签。检查花括号时必须先去掉字符串字面量
（Go 样本里有 `"{"`、`frame={sp:`、`{index out of range` 这类常量，否则会误报）。

实测记录（2026-10-30）：notepad（MSVC，502 个函数）与 corpus-go（3261 个函数，Go
runtime/编译器产物）在三条路径上均为 **0 报错**；字符串字面量剔除后花括号全部配平，
`goto` 全部有标签。Go 样本同时是覆盖率观察点：结构化 88.9%（1779/2000 可达函数）、
指令覆盖率 98.7%。

## 当前基线（2026-10-31，三个样本）

```
functions:              31
structured:             31 (100%)
instructions:           391
intrinsic instructions: 10 (coverage 97.4%)
goto fallback functions: 0
labels:                 0
inline asm statements:  9
```

| 样本 | 函数 | 结构化 | 覆盖率 | `loc_` 标签 | `__asm` |
|---|---|---|---|---|---|
| notepad.exe（502） | 502 | 487（97.0%） | 99.5% | 267 | 138 |
| crackme.exe（493） | 493 | 453（91.9%） | 98.5% | 155 | 275 |
| corpus x64（31） | 31 | 31（100%） | 97.4% | 0 | 9 |
| corpus x86（32） | 32 | 32（100%） | 97.6% | 0 | 8 |
| go.exe（1481，受限取样） | 220 | 196 结构化（89.1%） | 98.7% | 1059 | 1343 |

`recraft metrics <project>` 一次打印上述全部计数（含 `labels` 与 `inline asm statements`）。
语义覆盖率达到 95% 目标：`push/pop`（栈语义）、`imul`、`cmov`、SSE 子集
（`movd/movdqa`、`paddd/paddq`、`psrldq/pslldq`、`punpck*` 以外的 pack 运算、
`xorps`、`addss/mulss/divss`、`cvttss2si` 等）已进入 IR。剩余 13 条 intrinsic 是
**洗牌/解包类 SIMD**（`pshufd`、`punpcklbw/lwd`）、1 条 16 字节 `movdqa` 内存形、
**间接跳转**（`jmp rcx/rax/r9`）和 **SSE 比较后的未建模标志分支**——语义不可用标量
表达式直译，保留 `__asm` 是设计允许的降级路径。

真实样本 notepad（重新分析后 502 个函数、**37522 条可达指令**）：覆盖率 **99.5%**
（170 intrinsic）、结构化 **97.0%**（487/502；依次经历 443 函数旧分析状态 49.2% →
62.4%（早退链）→ 77.3%（break/continue）→ 77.7%（do-while）→ 78.7%（复杂循环头兜底）
→ 81.1%（CFG 可达性裁剪）→ 81.5%（共享尾声块复制）→ 84.7%（共享尾部链复制）→
86.1%（`bt`/SF 标志分支与 `setcc` 建模）→ 96.4%（共享/末尾尾部复制与死代码清理）→
97.0%（共享尾部可达 48 行、短路 `||` 合臂与循环头标签）→ 96.8%（跨块标志传播后多出一个
`goto` 回退，换掉了 18 处 `__asm("if (... unmodelled flags) goto ...")`）→ 97.0%
（从扁平回退文本里恢复循环与守卫），已超过 v1 的 70% 目标）；
不作为门禁，仅作为观察指标。剩余 intrinsic 集中在原子指令（`lock xadd`、`lock cmpxchg` 的写入部分）、
`int3` 陷阱与 1 操作数 `imul`。

**会话整体收益（2026-10-31，对比会话开始时的基线二进制，同一 notepad 项目）**：
L2 行数 34 259 → **30 174**（−11.6%）、`goto loc_` 行 1 264 → **452**、`while` 循环
**278** 个（其中 `while (1)` 59）、`do` 循环 **208** 个、`__asm(...)` 行 240 →
**138**、调用实参 setup 行 6 722 → **4 040**、intrinsic 指令 239 → **170**（覆盖率
99.5%）、结构化 487 → **487/502**。
`/GS` 安全 cookie 的 126 处检查及对应序言 setup 已从伪代码中移除（`return
sub_…(cookie)` 变成真实返回值），跨块调用实参补回 99 处（如 `recursive(ecx)`），
`__chkstk` 栈探测（`eax = size; call`）在 6 个函数里移除、29 处跳转到返回标签的
`goto` 折成 `return`。整函数扁平回退改为**只扁平出问题的那个
循环**：10070 行 / 687 处 `goto` 降到 10080 行 / **402** 处，函数其余部分照常结构化
（Go 样本 1779 → **1780/2000**）。指针实参的字段访问 `argN->field_XX` 137 → **266**（121 个函数）；跨块调用实参补回 128 处（空 API 调用 433 → **305**）；`sub rsp, rax` 的帧大小恢复后槽名归一，`var_nXXXX` 2090 → **18**（大函数 1891 → 1）。

**指标口径修正（2026-10-24）**：此前 `measure` 按 `[function.start, end)` 线性扫描，把函数
范围内的对齐填充（`int3`）、跳表数据和不可达空隙也算作指令，导致 notepad 覆盖率被低估、
结构化率被拉低（旧值 108142 指令 / 84.9%）。CFG 构建现在裁剪不可达块（`int3` 从 8547 →
33）并把重叠函数区间夹到下一个函数起点，`measure` 只统计 CFG 内实际可达的指令；其后
又补了 `movups/movdqu/movdqa` 内存形式、`ror/rol`、间接 call / IAT 尾调用、共享尾部链
复制与 `bt`/SF/`setcc` 标志语义。因此 99.5% / 96.8% 是可复现的真实口径。

## 伪代码后处理（L1/L2 共用）

结构化发射之后，`structuring.rs` 依次执行一组保守的文本级折叠；每一步都必须保持
**每个 `goto` 都有同名标签**、**花括号平衡**、**条件非空**这三条不变量（corpus 上由
`generated_gotos_always_have_a_label` 与 `generated_output_has_balanced_braces` 断言）：

| 处理 | 作用 |
|---|---|
| 标签/尾部复制 | 共享尾部、末尾标签、兄弟分支尾部复制为 `if`/直写；最多 48 行、允许内部标签 |
| 死代码裁剪 | `return` 之后按花括号深度整段删除；被外部跳转引用的标签保留 |
| 短路合臂 | 相邻且体相同的终止 `if` 合并为 `if (a \|\| b)` |
| 表达式收拢 | `x = (x + n)` → `x += n`；`r = *(T*)(a); r -= k; *(T*)(a) = r;` → `*(T*)(a) -= k;` |
| 条件清理 | `if (!(x == 0))` → `if (x != 0)`；整条件外层多余括号去掉 |
| 参数传播 | 序言里 `rsi = arg1` 且此后不再赋值时，所有读取改用 `arg1` 并删掉复制 |
| 栈槽/保留寄存器 | 只用于保存/恢复的 callee-saved 槽隐藏；`frame = rsp` 别名按 `rsp` 命名；死栈写删除；序言建立的 `rbp` 帧基址在后续所有块沿用，同一地址不会出现两个名字（`var_18` vs `var_n10`），L1 也不再输出裸 `*(u64*)((rbp + 0x200))` |
| 调用美化 | IAT 槽解析成 API 名，`rcx/rdx/r8/r9` 渲染为实参，栈地址写成 `&var_XX`；第 5 个及以后的栈实参从调用前的 `[rsp+0x20]`… 写入按槽序追加（只在四个寄存器实参都可见时追加） |
| 句柄命名 | 句柄 API 收到的槽按其类型命名：`hObject`/`hKey`/`hMutex`/`hEvent`/`hModule`/`hFind`/`hMenu`/`himl`/`hstring`/`hwnd`/`hdc`/`hIcon`/`hCursor`/`hHook`；同一槽被多个 API 收到时取最具体的一种 |`n| 栈上参数 | x64 无 push 的 `sub rsp, const` 序言下，读 `frame + 0x28` 及以上的槽是调用方压栈的实参，写成 `arg5`/`arg6`…，签名补全并在参数注释里标 `arg5+=stack` |`n| 地址常量 | 指向数据的常量写成 `off_<addr>`，等于函数入口的常量写成该函数名（回调指针），匹配字符串字面量的写成字符串；其余保持原样 |`n| 全局数据 | 绝对地址的解引用写成带宽度前缀的符号：`*(u64*)(0x140036648)` → `qword_140036648`，`u8/u16/u32/f32/f64/__m128` → `byte_/word_/dword_/flt_/dbl_/xmmword_`；计算地址保留强制转换 |`n| void* 字段 | 类型推断为 `void*` 的实参，若某偏移只有一种访问宽度，则 `*(u32*)((arg1 + 0x10))` 写成 `arg1->field_10`；同一偏移出现两种宽度则保留强制转换 |`n| 结构实例命名 | 已知 API 填充的结构槽改用具名变量：`&var_30` → `&rect`（`MSG` → `msg`、`POINT` → `pt`），已占用时加数字后缀 |`n| 已知结构 | 表驱动：`GetClientRect`/`GetWindowRect` 的 `&var_XX` 参数定型 `RECT`（`left/top/right/bottom`），`GetMessageW`/`PeekMessageW`/`TranslateMessage`/`DispatchMessageW` 定型 `MSG`，`GetCursorPos`/`ClientToScreen`/`ScreenToClient` 定型 `POINT`（`hwnd/message/wParam/lParam/time/pt`）；只改后续成员访问，取地址、整块清零、指针基址保持裸槽名，重复出现不会二次加前缀 |
| 返回类型 | 函数所有出口都是裸 `return;` 时签名写 `void`（结尾是调用语句的保持整型返回，源可能是 `return f();`） |`n| 实参溢出槽 | 只被一个实参写一次的槽等同于该实参：读取改名为 `argN` 并删除溢出语句；被多次写入或取地址的槽保留 |`n| 常量比较 | 为一次比较而载入寄存器的常量搬进条件（`if (arg2 > 0x111)`），寄存器在该次使用后被重写则删除赋值；两个常量相比时保留寄存器形式 |`n| 空语句清理 | `x = x;`、空体 `if (...) { }`、无人跳转且只引出 `}` 的标签一律删除 |`n| 直调实参 | 实参是参数寄存器本身时（`edx = 0; f(a, rdx)`）改用该寄存器当时的值；只有常量/栈槽/`&var` 这类可重复求值的值才替换，内存读取或算术表达式保留寄存器名 |`n| 跨块直调 | 实参在别的块里建立的直调记录不到实参：用同函数内同目标最宽的兄弟调用给出个数，打印对应个数的寄存器占位（超过 4 个的不处理，栈上实参位置无法只凭寄存器名确定） |`n| 尾跳 | 目标越过函数末尾的跳转渲染为 `return sub_xxx(实参);`：紧邻跳转上方的 `rcx/rdx/r8/r9` 赋值按 ABI 顺序取作实参，数量以 callee 已恢复的实参个数为上限（未知时只取 `rcx`）；摘不到的仍写 `return sub_xxx();` |
| 返回值 | `eax = expr; return;` → `return expr;`；`eax += x; return;` → `return (eax + x);`；只计算到寄存器、没有相邻赋值的值返回路径写成 `return eax;`/`return rax;`（`void` 保持裸 `return;`）；紧邻的单次使用临时量并入返回表达式（`eax = a; eax += b;` → `return (a + b);`）；比例 1 的乘除（`*(arg2 * 1)`）去掉 |
| 跳表派发 | `switch` 前编译器生成的派发语句（表加载、`__asm("jmp rcx")`）被 `switch` 取代；仅当这段语句全是寄存器运算且以间接跳转结尾时删除，含调用/内存写入的块保持原样 |
| 循环收敛 | 唯一出口是 `if (cond) break;` 的 `while (1)`：测试在循环体开头时写回 `while (!cond)`，在末尾时改写为 `do { ... } while (!cond)`；体内有 `continue` 时保持原样（`continue` 会先到达闩锁，改条件会跳过原语义） |
| else-if 合并 | `} else { if (cond) { ... } }` 在 else 分支只有这一条语句时折叠为 `} else if (cond) { ... }`，分支体整体左移一级 |
| else 展平 | then 分支以 `return`/`break`/`continue`/`goto` 结尾时去掉冗余 `else`，else 分支体左移一级（早退风格） |
| 栈槽地址 | `lea` 得到的栈槽地址写成 `&var_XX`；裸 `[rsp]`/`[rbp]` 槽写成 `var_0` |
| 单行条件 | 单行 `if (cond) break/continue/return/goto;` 去掉生成器加的多余括号 |
| 死寄存器写 | 直线段内下一次出现即被同基址 32/64 位完整重写时删除该写（调用/分支/标签/汇编中止扫描，重写语句自身读该寄存器则保留），并删除全函数再无提及的寄存器写（`rax` 与实参寄存器除外） |
| 临时折叠 | 只喂一条存储或一条返回的临时量并入目标语句：`*(u32*)(a) = ecx` ← `ecx = ...`、`var_8 = ecx` ← `ecx = 0`、`return ebx` ← `ebx = ...`；寄存器在之后仍被读取时保留拷贝 |
| 进位掩码 | `sbb x, x` 提升为 `x = cond ? -1 : 0`，其消费者折成 `x = (cond ? 1 : 0)` 或 `x = (cond ? y : 0)` |
| 标志跨块 | 条件跳转后的下一个测试复用前驱块里纯 `cmp`/`test`/`bt`（在本块表达式域重建），`cmpxchg` 也记录它执行的比较 |
| 边界 setup | `while (1)` 体首的不变量 setup（栈槽地址/常量）在体内无调用、汇编、重写时提到循环之前，使其 `break` 收敛成真实循环条件 |
| 指针字段 | 指针型实参的字段访问写成 `argN->field_XX`：块内的复制（`r14 = arg4`）以及跨块仍然保持的参数副本（CFG 上的 must-alias 交集）；仅当访问宽度等于指针元素宽度时使用，否则保留显式转型以保住位宽 |
| 帧别名 | `mov rbp, rsp` 只提供槽命名别名，不打印 `frame = rsp;`（簿记照常运行，`var_nXX`/`frame_XX` 命名不变）；末尾再删掉无人再引用的 `X = &var_N;` 序言 |
| 安全 cookie | MSVC `/GS`：序言里全局值与 `rsp` 混合后存入槽位的 setup、以及每个返回路径上 `槽位 ^ rsp` 的检查调用都移除（按形状识别，不假设 API 名）；检查后的 `return` 露出真实返回值，setup 在无人读取时一并删除 |
| 栈探测 | MSVC `__chkstk`：`eax = 帧大小; call __chkstk;` 整体移除（callee 形状：`sub rsp, imm` 后 6 条内出现 `mov [rsp], r10/r11`），帧建立前只触碰保护页，无源码语义 |
| 跨块实参 | 调用与寄存器实参 setup 被基本块边界分开（跳转目标成为 leader）时，从调用上方直线段恢复 `rcx/rdx/r8/r9`（遇标签、花括号、调用即停；栈尾复制出的每个副本按各自的上下文恢复）；只有已知非空实参个数的 callee 用整段恢复，其余直接 `sub_*` 调用要求 rcx 写在紧邻上一行；x86 不做（实参在栈上）。IAT 调用的实参个数取自常见 Win32 签名表（66 个 API）：已知个数时按个数打印，未解析到的槽用 ABI 寄存器名占位，0 参数 API 不打印实参；表里没有的仍按寄存器建立规则决定。IAT 调用另有跨块传播：实参寄存器里的值按 CFG must 分析带到后续块，值随带它读取的寄存器集合，任一寄存器在中途被重写即失效 |
| 平凡返回标签 | 块体只是返回值的标签：`goto loc_X;` 折成 `return <value>;`、`if (cond) goto loc_X;` 折成 `if (cond) return <value>;`，标签无引用后消失 |
| 短跳转守卫 | 前向 `if (cond) goto L;` 且跳过的区域 ≤64 行、花括号自平衡、不含绑定的 `break`/`continue` 时折成 `if (!cond) { ... }`；更长的跳转保留（把几百行缩进进去反而不可读） |
| 回边成环 | 区域末尾跳回自身标签的写成 `do { ... } while (cond);`（无条件回边的 `do { } while (1);` 再规范成 `while (1) { }`）；区域含 `break`/`continue` 时保留跳转，避免绑定到新引入的循环 |
| 局部扁平 | 内层循环的出口跳出最外层循环体时，只把**最外层那个循环**整体按标签发射（每个块 `loc_`、每条边 `goto`），函数其余部分照常结构化；此前是整函数扁平回退 |

这些处理是**纯文本**的，因此只做能由文本证明安全的变换（例如参数传播要求复制位于
任何分支之前的直线序言、寄存器为 callee-saved 且全函数只写一次）。不满足条件的
情况保留原始 `goto`——这是设计允许的降级路径，而不是质量缺陷。

## 畸形输入回归（DESIGN §11 的离线 fuzz 替代）

`crates/recraft-format/tests/malformed.rs` 用确定性 xorshift PRNG 覆盖三类输入，要求
加载与所有公开访问器**不 panic**（加载失败可以，崩溃不行）：

| 用例 | 规模 |
|---|---|
| 随机字节 blob | 500 例（长度 1–4096，含完整随机头） |
| 真实 notepad.exe 随机截断 | 400 例 + DOS/NT 头边界定点截断 |
| 头部/节表字节突变 | 400 例（DOS/NT 头与节表区，1–4 个 bit flip） |

加载成功的畸形镜像还会遍历 `segment_containing`/`rva_to_file_offset`/节权限/导入/导出/
TLS/pdata 访问器。CI 的 `cargo test --workspace` 会自动执行（约 0.06 s）。真实
`cargo-fuzz` 需要 nightly，当前环境未启用，作为发布前的可选补充。

## golden 快照

- 目录：`tests/golden/corpus/`（x64 语料，31 个函数）与 `tests/golden/corpus86/`（32 位
  语料，32 个函数），每个函数一个 `.c` 文件，**按符号名命名**（插入新函数不会
  重排已有文件；重名函数追加地址后缀）。32 位快照的 L1 文本按设计保留原始形态
  （`eax = var_4;`，对应 x64 的 `eax = (rcx + rdx);`），参数名与 ABI 类型属于 L2。

- 内容：**归一化后的 L1 文本**——`0x...` 地址与 `loc_...` 标签替换为 `0xADDR` /
  `loc_ADDR`，因此重建/换基址不会误报；语句与结构形状仍参与 diff。
- 更新流程：改动反编译器后运行 `recraft golden <corpus-project> --dir tests/golden/corpus`，
  **人工 review `git diff tests/golden/corpus`**，确认变化符合预期后提交。
- CI 策略：windows-latest 的 MSVC 版本与本地可能不同，直接对比仓库快照会抖动；
  CI 先写入临时目录再 `--check` 做**确定性/归一化回环**校验。仓库快照用于本地与
  review 时的真实回归。M3 引入固定 toolchain 容器后改为直接对比提交的快照。
