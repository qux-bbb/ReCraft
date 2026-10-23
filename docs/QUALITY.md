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
functions:              24
structured:             21 (87.5%)
instructions:           346
intrinsic instructions: 13 (coverage 96.2%)
goto fallback functions: 3
```

语义覆盖率达到 95% 目标：`push/pop`（栈语义）、`imul`、`cmov`、SSE 子集
（`movd/movdqa`、`paddd/paddq`、`psrldq/pslldq`、`punpck*` 以外的 pack 运算、
`xorps`、`addss/mulss/divss`、`cvttss2si` 等）已进入 IR。剩余 13 条 intrinsic 是
**洗牌/解包类 SIMD**（`pshufd`、`punpcklbw/lwd`）、1 条 16 字节 `movdqa` 内存形、
**间接跳转**（`jmp rcx/rax/r9`）和 **SSE 比较后的未建模标志分支**——语义不可用标量
表达式直译，保留 `__asm` 是设计允许的降级路径。

真实样本 notepad：443 函数、覆盖率 90.8%、结构化 49.2%（体积小、CRT 紧凑函数多，
回退分支多）；不作为门禁，仅作为观察指标。

## golden 快照

- 目录：`tests/golden/corpus/`，每个函数一个 `.c` 文件（按函数起点排序编号）。
- 内容：**归一化后的 L1 文本**——`0x...` 地址与 `loc_...` 标签替换为 `0xADDR` /
  `loc_ADDR`，因此重建/换基址不会误报；语句与结构形状仍参与 diff。
- 更新流程：改动反编译器后运行 `recraft golden <corpus-project> --dir tests/golden/corpus`，
  **人工 review `git diff tests/golden/corpus`**，确认变化符合预期后提交。
- CI 策略：windows-latest 的 MSVC 版本与本地可能不同，直接对比仓库快照会抖动；
  CI 先写入临时目录再 `--check` 做**确定性/归一化回环**校验。仓库快照用于本地与
  review 时的真实回归。M3 引入固定 toolchain 容器后改为直接对比提交的快照。
