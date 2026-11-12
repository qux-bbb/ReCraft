# DecBench 对照（sample-set）

[DecBench](https://decbench.com) 是 Noelo Lab 维护的反编译器基准：同一批真实二进制（ELF/PE、
多种架构与优化级别），把每个函数的反编译结果按三个指标打分，所有分数都是「完美函数」的
百分比。

| 指标 | 衡量什么 | 对 ReCraft 的含义 |
| --- | --- | --- |
| **GED**（结构） | 控制流图与原始源码的图编辑距离 | 我们的强项：结构化率与 CFG 恢复是主线 |
| **Types**（类型） | 与 DWARF 真值的类型翻转次数 | 弱项：目前没有 DWARF 类型，只有 ABI 宽度 |
| **Bytematch**（可重编译） | 修好可编译性问题后重编译、比对汇编 | 基本为 0：输出是伪 C（`var_n8`、`int32`），不保证可编译 |

数据集与评测工具：`https://huggingface.co/datasets/noelo-lab/decbench-dataset`，
外部评测套件是 `kits/decbench-evalkit-sample-set.zip`（250 个函数 / 223 个二进制）。

## 本地跑一遍

```bash
# 1. 取套件（约 18 MB）
curl -L https://huggingface.co/datasets/noelo-lab/decbench-dataset/resolve/main/kits/decbench-evalkit-sample-set.zip -o kit.zip
unzip kit.zip && cd decbench-evalkit-sample-set

# 2. 用 ReCraft 生成 submissions（每个二进制一个 .c + results.json）
recraft-cli decbench . --out results

# 3. 套件自带的校验/打包脚本
python3 package.py
```

`recraft-cli decbench <kit> -o <dir>` 做三件事：按 `functions.json` 的 `public` 列表逐二进制
`import → analyze → persist`（含字符串表），对列出的地址渲染 L2 伪 C，然后写
`results/<二进制名>.c` 与 `results/results.json`（`decompiler.name = "recraft"`）。地址沿用
ELF/PE 头里的虚拟地址，和套件要求一致。缺文件、加载失败或加载器明确拒绝的二进制会打印一行
`skip …`；没有任何地址能渲染的二进制只计入最后的 `skipped` 数。

## 2026-11-12 实测（sample-set）

| 格式 | 套件内（二进制 / 目标函数） | 产出 | 说明 |
| --- | --- | --- | --- |
| ELF64 x86-64 | 158 / 158 | **158 / 158** | 100%；11-09 时只有 144，缺的 14 个目标现在都是函数入口 |
| PE32 i386 | 8 / 8 | **8 / 8** | 100%；5 个只靠序言可见的目标由序言回退发现（见下） |
| ELF32 **ARM**（`EM_ARM`） | 57 / 84 | 0 / 0 | 需要 ARM32 lifter；这一行**不是 i386**（见下） |
| 合计 | 223 / 250 | **166 / 166** | 66.4% 的目标函数（不做 ARM 时的理论上限），debug 构建约 2–3 分钟 |

产物 832 KB（166 个 `.c`），`recraft-cli decbench` 报告
`wrote 166 binaries / 166 functions (57 skipped)`。

### PE32 序言回退（同日补测）

`bin_150/158/198/213/222.exe` 的 5 个目标地址此前不在 `funcs` 里，原因不是它们「不是函数」，
而是 PE32 没有 `.pdata`、这些函数又不在导出表里，也不被任何直接 `call` 或数据指针引用：
`bin_150` 的 `0x4033b0` 是作为立即数传给线程创建 API 的线程例程，`bin_198` 的 `0x4013d1`
更像链接器保留下来的、没有任何引用的代码。现在 `recraft-analysis` 在递归下降收敛后增加一次
序言回退：扫描仍未覆盖的可执行区间，只接受「`ret`/`jmp` 或对齐填充之后」的 `push` 被调用者
保存寄存器、无帧 `sub esp, imm`、`lea esp, [esp-…]`、热补丁 `mov edi, edi`，并用一次试探性
CFG 遍历确认它确实解出一段指令（能到达 `ret`，或长到不像数据巧合）。回退只在镜像没有精确
函数表时启用（PE32、剥离符号的 ELF），因此 PE 789 张 golden 零 diff；这 5 个地址现在都是
`funcs` 里的 `sub_…`（来源 `prologue`）。

**口径更正**：套件里 57 个 32 位 ELF 全是 ARM（`e_machine = 0x28`），没有 i386。此前把这一行
记成「ELF32 i386 57」是错的：ELF32 i386 支持（`f1ce973`）并不给 DecBench 加分，那 84 个目标
（250 的 33.6%）要等 ARM32/ARM 支持；不做 ARM，理论上限就是 166/250（66.4%）。

**本轮顺带修掉的导出中断**：SysV 调用第 5/6 个寄存器实参（`r8`/`r9`）在 `call_argument()` 里
索引长度只有 4 的 `arg_values`，越界 panic（`bin_088.elf` 触发，导出写出 64 个 `.c` 后中止）。
读路径改成 `.get(slot)`，与只跟踪前四个槽的 `arg_slot()` 保持一致；PE 789 张 golden 零 diff。

比 11-09 慢（约 65 秒 → 2–3 分钟；本机静置一轮 138 秒，与其它构建并发时 194 秒）的原因是新增
的 14 个二进制体量大：`bin_034.elf` 一个就有 347 个函数、9 135 条指令，每个二进制都要完整跑
一遍 `analyze → L2`。序言回退只在没有精确函数表的镜像上多花时间（PE32 各自几十毫秒），对
ELF64 主体没有影响。

这组数字只说明「能不能跑通并产出合规提交」，不是最终分数：真正的 GED/类型分数由
DecBench 侧用 Joern + 原始源码计算。

## 已知差距（按对分数的帮助排序）

1. **ARM32**：84/250 个目标函数落在 57 个 `EM_ARM` 二进制里。0.3.0 的 ARM64 lifter 覆盖不了
   它们，需要 ARM32（或同时实现 AArch32 状态）；这是剩余分数里最大的一块。
2. **类型**：没有 DWARF 类型就没有类型分数；`.debug_line` 行号已支持（`recraft-cli lines`），
   但 `.debug_info` 的变量/参数类型还没解析。
3. **可编译性**：伪 C 不追求可编译，这项分数不会有起色；除非引入 `--ansi-c` 输出模式。

PE32 入口识别已不再是差距：序言回退让 8/8 个 PE32 目标都进入 `funcs`（见上）。它只补「没有
精确函数表」的镜像，所以对已经由 `.pdata`/`.symtab`/`pclntab` 覆盖的样本是零影响。

## 与仓库门禁的关系

DecBench 不进 CI：套件 18 MB、需要联网，且分数由外部计算。仓库里的护栏仍是
`lint` + 919 份 golden + `metrics`；本文档的数字在需要时用上面的命令复现。

历史对照：11-09 首跑 147/250（ELF64 144、PE32 3），当时把剩余的 57 个二进制误记为 i386；
11-12 白天 161/250（ELF64 158、PE32 3），当晚 PE32 补到 8/8，合计 166/250。
