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
ELF/PE 头里的虚拟地址，和套件要求一致。缺文件、加载失败或地址没被识别成函数都会跳过并
计入统计。

## 2026-11-09 实测（sample-set）

| 格式 | 套件内 | 产出 | 说明 |
| --- | --- | --- | --- |
| ELF64 x86-64 | 158 | **144** | 其中 14 个的目标地址没被识别成函数 |
| PE32 i386 | 8 | **3** | 另外 5 个同样是入口识别问题 |
| ELF32 i386 | 57 | 待重跑 | 加载器 2026-11-12 起支持 ELF32（`SHT_REL` 导入、i386 PLT、栈参数），本表数字为支持前所测 |
| 合计 | 223 | **147** | 147/250 个目标函数，约 65 秒，`package.py` 校验通过 |

产物体积 166 KB，`package.py` 报告
`packaged 147 .c file(s)`、`binaries covered 147/223, functions 147/250`。

这组数字只说明「能不能跑通并产出合规提交」，不是最终分数：真正的 GED/类型分数由
DecBench 侧用 Joern + 原始源码计算。

## 已知差距（按对分数的帮助排序）

1. **ELF32 重跑**：57/223 个目标准入条件已满足（加载器支持 ELF32），但表中成绩仍是支持前
   的数字，需要在同一套件上重跑才能给出正式结论。
2. **函数入口识别**：19 个目标地址没被当成函数（ELF64 14 个、PE32 5 个），多半是 thunk /
   IFUNC / 无 `.eh_frame` 的叶函数，值得单独立项看。
3. **类型**：没有 DWARF 类型就没有类型分数；`.debug_line` 行号已支持（`recraft-cli lines`），
   但 `.debug_info` 的变量/参数类型还没解析。
4. **可编译性**：伪 C 不追求可编译，这项分数不会有起色；除非引入 `--ansi-c` 输出模式。

## 与仓库门禁的关系

DecBench 不进 CI：套件 18 MB、需要联网，且分数由外部计算。仓库里的护栏仍是
`lint` + 849 份 golden + `metrics`；本文档的数字在需要时用上面的命令复现。
