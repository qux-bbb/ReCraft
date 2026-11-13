<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# DecBench 对照与提交计划（P 线）

[DecBench](https://decbench.com/) 是 Noelo Lab（UGA）维护的反编译器基准：770 个二进制、
96k 函数、16 个反编译器，按三个「完美率」指标排名——CFG 图编辑距离（GED）、类型正确率、
重新编译后的字节级一致率，外加三者的并集（Union）。它只看反编译**文本**，因此任何
反编译器（包括闭源/私有）都能参赛；外部提交入口在
[Noelo-Lab/decbench](https://github.com/Noelo-Lab/decbench) 的 "compete externally" 一节。

## 1. ReCraft 与三个指标的对应关系

| 指标 | DecBench 的度量 | ReCraft 现状 | 差距 |
|---|---|---|---|
| GED | 源码与反编译输出的 CFG 图同构度 | 有 CFG 与结构化（`recraft-analysis::build_cfg` + L2 结构恢复），但输出是 IDA 风格伪 C，不是可编译 C | 需要「可被 Joern 解析的 C」输出模式 |
| 类型 | DWARF 真值 vs 恢复的变量/参数类型 | L2 有 `ArgType`/结构体推断，覆盖率未按此口径测过 | 需要按 ABI 位置与栈偏移对齐变量 |
| 字节一致 | 用原工具链重编译后与 `.text` 比对 | 无「可编译」目标（DESIGN 明确不保证伪代码可编译） | 需要 `--emit-c` 模式 + 编译修复 |
| Union | 三者至少一个完美 | 未知 | — |

结论：**现在提交只会得到一个接近 0 的分数**，因为 ReCraft 的伪 C 不是为"可编译"设计的
（`undefined8`、`*(u64*)(x)` 这类写法 Joern 也未必能解析）。这个数字对项目没有解释力，
反而会给外部留下"不能反编译"的错误印象。

## 2. 什么时候提交

满足以下两条后再提交：

1. 有一个 `--emit-c` 输出模式：类型用标准 C 名字、指针强制转换写成 `*(uint64_t*)`、
   未知类型注入 `typedef`，能被 `gcc -std=c11` 解析（不要求语义完全正确）；
2. 在 `sample-set`（12 个项目的小样本）上跑通本地管线，且 GED 的完美率 > 0（说明至少
   有一小部分函数的控制流与源码同构）。

## 3. 本地跑法（现在就能做）

1. 拉取数据集（HuggingFace `noelo-lab/decbench-dataset`）与 decbench 仓库；
2. 对每个项目的二进制：`recraft-cli import <bin> -o <proj>` → `analyze <proj>` →
   对函数表逐个 `decompile --addr` 导出 `.c`；
3. 把导出目录按 decbench 的 `decompiled/<project>/<binary>/<function>.c` 结构摆放，
   套用它的评分脚本（Joern + DWARF 真值）跑 GED/类型/字节一致；
4. 记录每项指标的 Union，写进 `docs/QUALITY.md` 的对照表。

## 4. 决策记录

- 2026-11-14：**暂不提交**。先把 0.3.0 的 ARM64 与 0.4.0 的 UI 收口；
  `--emit-c` 模式与 decbench 本地管线作为 0.4.0 之后的可选工作项，届时再决定是否公开提交。
  如果用户希望现在就跑一次"摸底"（不公开），按第 3 节的步骤在本地执行即可，不影响发版。
