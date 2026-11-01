# 性能门禁（1 GiB，DESIGN §3.3 / §14 M3）

## 测量方式

`scripts/bench-large.ps1`：

1. 把 `notepad.exe`（或 `-PEFile`）复制成临时样本，并在文件尾追加 `-OverlayMB`（默认
   1024）MiB 零数据，得到一个 **1 GiB 样本**（overlay 不在任何映射段内，PE 结构不变）。
2. 用 `recraft import` 测量导入墙钟时间，`recraft analyze` 测量分析墙钟时间。
3. 直接使用 `System.Diagnostics.Process` 启动子进程，每 20 ms 采样 `WorkingSet64` 得到
   **峰值工作集**（PowerShell 的 `Start-Process` 无 `-Wait` 时丢失 ExitCode、带 `-Wait`
   时丢失 PeakWorkingSet64，所以脚本自己实现）。
4. 超出阈值即非 0 退出；`-Json` 输出机器可读结果。

默认阈值（DESIGN §3.3）：

| 指标 | 阈值 |
|---|---|
| 导入 | < 60 s |
| 分析（交互预算） | < 5 s |
| 分析进程峰值工作集 | ≤ 512 MiB |

## 当前结果（2026-10-24，debug 构建，本机）

```
[bench] sample: 1,024.3 MiB
[bench] import: 4.03 s (limit 60.0 s)
[bench] analyze: 0.32 s (limit 5.0 s), peak 5.6 MiB (limit 512 MiB)
[bench] PASS
```

CI 在 `windows-latest` 上跑同一脚本（1 GiB overlay），作为性能回归门禁。

## 反编译视图（单函数，2026-10-30）

`decompile --l2` 的耗时由函数规模决定。会话里最大的 notepad 函数
（`sub_140013154`，45 KB、1 620 个基本块、约 1 万行伪代码）在 debug 构建下从
**12.2 s 降到 1.3 s**：单次使用临时量折叠原先对每个候选重扫全函数统计 token 次数，
标签整理对每个标签重扫全函数统计引用，IAT 名称表按块重建；现在 token 频次只统计一次
并在折叠时维护，跳转目标一次收集，IAT 表每个函数建一次，前两者都是线性。

2026-10-31 又做了一轮：`hide_callee_saves` 对每个被保存的槽重扫全函数（平方级）改为
一次建立「槽 → 行号」索引，`propagate_argument_copies` 不再对每个实参拷贝重写全部行。
文本 pass 合计从约 800 ms 降到约 310 ms，整函数约 1.3 s；`merge_identical_if_arms` 的配对大括号改为一次预计算、`fold_sibling_tail_gotos` 的标签查找改为一次索引（约 265 ms）、`merge_identical_if_arms` 的逐行大括号计数改为一次预计算（约 225 ms）；`RECRAFT_PROFILE=1 recraft
decompile ...` 会打印各阶段耗时（lift / 分析 / 发射 / 文本 pass）。两次优化都保证 502 个
notepad 函数的输出逐字节未变。

## 解释与限制

- **overlay 不参与分析**：1 GiB 样本的分析成本由**映射段中的代码量**决定（notepad 约
  1.3 万条指令），因此 0.32 s 不代表"1 GiB 全是代码"的极端情况。真正的代码密集型大
  二进制（如合并后的系统 DLL）会随指令数增长，`MAX_TOTAL_INSTRUCTIONS = 5,000,000`
  上限提供保护。
- 导入时间是**流式**的（256 KiB 分块写镜像），内存不随文件大小线性增长；峰值 5.6 MiB
  是 CLI 进程（无 UI、SQLite 页缓存较小）的实测值。
- debug 构建：release 构建的导入/分析会更快；门禁阈值按 debug 设定，留有余量。
- 内存统计是 20 ms 采样的下界，峰值可能略低；如需精确峰值可换 ETW/Job Object
  `PeakProcessMemoryUsed`，后续按需引入。
- CI 的磁盘/时间成本约 10–20 s（写 1 GiB + 读回），可接受。

## 反编译视图剖析更新（2026-11-02）

`RECRAFT_PROFILE=1 recraft decompile <project> --addr 0x140013154 --l2` 现在覆盖整条流水线。
notepad 的 45 KB 函数（`sub_140013154`，1620 基本块、约 9.9 千行 L2）在本机 debug 构建下：

| 阶段 | 耗时 |
|---|---|
| `cfg` | 0 ms |
| `first lift` | 125 ms |
| `alias/arg analysis` | 37 ms |
| `seeded lift` | 77 ms |
| `emit` | 10 ms |
| `text passes` | ~260 ms（`tails` 40、`folds` 50、`ifs` 47、`spills` 24、`returns` 18、`gotos` 11、其余 68） |
| `post passes`（命名） | ~34 ms（`struct fields` 9、`struct inst` 8、`void fields` 7、`globals` 7、`constants` 3） |
| `naming` | 15 ms |

整函数墙钟约 **1.0 s**（进程启动 + 打开工程仅 30 ms）。本轮把三个"按候选反复扫整篇"的命名 pass
改成"每趟只扫一次 + 预建索引"（`name_address_constants` 45→3 ms、两个结构体匹配 26/25→9/8 ms、
`name_handle_slots` 47→15 ms），输出**逐字节不变**（453 个 golden 快照全部一致）。

剩余热点：`folds`（50 ms）与 `ifs`（47 ms）两组文本 pass、`first lift`（125 ms）。优化方式同上，
验收标准仍是 golden 逐字节一致。