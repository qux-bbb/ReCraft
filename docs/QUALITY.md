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

## 当前基线（2026-10-24，MSVC VS2022，corpus x64）

```
functions:              31
structured:             31 (100%)
instructions:           391
intrinsic instructions: 10 (coverage 97.4%)
goto fallback functions: 0
```

语义覆盖率达到 95% 目标：`push/pop`（栈语义）、`imul`、`cmov`、SSE 子集
（`movd/movdqa`、`paddd/paddq`、`psrldq/pslldq`、`punpck*` 以外的 pack 运算、
`xorps`、`addss/mulss/divss`、`cvttss2si` 等）已进入 IR。剩余 13 条 intrinsic 是
**洗牌/解包类 SIMD**（`pshufd`、`punpcklbw/lwd`）、1 条 16 字节 `movdqa` 内存形、
**间接跳转**（`jmp rcx/rax/r9`）和 **SSE 比较后的未建模标志分支**——语义不可用标量
表达式直译，保留 `__asm` 是设计允许的降级路径。

真实样本 notepad（重新分析后 502 个函数、**37522 条可达指令**）：覆盖率 **99.4%**
（239 intrinsic）、结构化 **86.1%**（432/502；依次经历 443 函数旧分析状态 49.2% →
62.4%（早退链）→ 77.3%（break/continue）→ 77.7%（do-while）→ 78.7%（复杂循环头兜底）
→ 81.1%（CFG 可达性裁剪）→ 81.5%（共享尾声块复制）→ 84.7%（共享尾部链复制）→
86.1%（`bt`/SF 标志分支与 `setcc` 建模），已超过 v1 的 70% 目标）；不作为门禁，
仅作为观察指标。剩余 intrinsic 集中在原子指令（`lock cmpxchg/xadd`、`sbb`）、
`int3` 陷阱与 1 操作数 `imul`。

**指标口径修正（2026-10-24）**：此前 `measure` 按 `[function.start, end)` 线性扫描，把函数
范围内的对齐填充（`int3`）、跳表数据和不可达空隙也算作指令，导致 notepad 覆盖率被低估、
结构化率被拉低（旧值 108142 指令 / 84.9%）。CFG 构建现在裁剪不可达块（`int3` 从 8547 →
33）并把重叠函数区间夹到下一个函数起点，`measure` 只统计 CFG 内实际可达的指令；其后
又补了 `movups/movdqu/movdqa` 内存形式、`ror/rol`、间接 call / IAT 尾调用、共享尾部链
复制与 `bt`/SF/`setcc` 标志语义。因此 99.4% / 96.0% 是可复现的真实口径。

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

- 目录：`tests/golden/corpus/`，每个函数一个 `.c` 文件，**按符号名命名**（插入新函数不会
  重排已有文件；重名函数追加地址后缀）。
- 内容：**归一化后的 L1 文本**——`0x...` 地址与 `loc_...` 标签替换为 `0xADDR` /
  `loc_ADDR`，因此重建/换基址不会误报；语句与结构形状仍参与 diff。
- 更新流程：改动反编译器后运行 `recraft golden <corpus-project> --dir tests/golden/corpus`，
  **人工 review `git diff tests/golden/corpus`**，确认变化符合预期后提交。
- CI 策略：windows-latest 的 MSVC 版本与本地可能不同，直接对比仓库快照会抖动；
  CI 先写入临时目录再 `--check` 做**确定性/归一化回环**校验。仓库快照用于本地与
  review 时的真实回归。M3 引入固定 toolchain 容器后改为直接对比提交的快照。
