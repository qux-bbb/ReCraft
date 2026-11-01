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

整函数墙钟约 **1.0 s**（进程启动 + 打开工程仅 30 ms；阶段末已降到约 **0.56 s**，见下文「阶段二收尾复测」）。本轮把三个"按候选反复扫整篇"的命名 pass
改成"每趟只扫一次 + 预建索引"（`name_address_constants` 45→3 ms、两个结构体匹配 26/25→9/8 ms、
`name_handle_slots` 47→15 ms），输出**逐字节不变**（453 个 golden 快照全部一致）。

剩余热点：`folds`（50 ms）与 `ifs`（47 ms）两组文本 pass、`first lift`（125 ms）。优化方式同上，
验收标准仍是 golden 逐字节一致。
## 阶段二收尾复测（2026-11-02，提交 f540dea）

同一函数 `sub_140013154` 的分段耗时（debug 构建）：

| 段 | ms |
|---|---|
| first lift | 122.4 |
| alias/arg analysis | 37.8 |
| seeded lift | 76.1 |
| emit | 10.0 |
| text passes | 67.3（tails 39.5 / gotos 11.8 / folds 51.8 / ifs 48.4 / spills 24.7 / returns 17.8，含重叠计时的子项） |
| 余下（frame probe、norm loops、struct fields、struct inst、void fields、globals、constants、stack args、post passes） | 约 35 |
| naming | 14.6 |
| **合计** | **约 556 ms**（墙钟约 0.6 s；阶段初为 12 s） |

与阶段中途的 1.0 s 相比进一步下降，主要因为逐函数缓存与一次遍历命名已全部生效。

### 反例：`ifs` 组的耗时不在括号重算

曾尝试把 `merge_identical_if_arms` / `collapse_nested_if_wrappers` 里"每次合并后整篇重算
`line_braces`"改为"只删除对应条目"（合并/删除的区间都是括号平衡的，因此后续前缀深度不变）。
改完**输出逐字节一致**（453 快照全绿），但 `ifs` 组耗时仍是 **47.1 → 47.2 ms**，说明这 47 ms
不来自括号重算，而来自三个条件改写 pass 每行构造新字符串（`simplify_negated_conditions`、
`strip_condition_parentheses`、`strip_inline_condition_parentheses`）。该改动因**无收益**已回退，
避免留下依赖"区间必定平衡"这一隐含假设的代码。今后若要压缩这 47 ms，应从字符串构造入手。
### 第二个反例：`strip_condition_parentheses` 的整行 clone 也不是瓶颈

该 pass 对每一行先 `line.text.clone()` 再判断前缀。改成"先在借用上判前缀、命中后才 clone"后，
输出逐字节一致（notepad 170 + corpus 31 快照全绿），但 `ifs` 组仍是 52.4 ms（前一版 47–50 ms，
属噪声级别）。说明这组耗时也不来自行文本克隆——47 ms 是四次全表扫描本身的成本，只有把
相邻的文本 pass 合并成一次遍历（或共享一次字符串缓冲）才可能真正下降。改动因无收益已回退。

第三个同类反例：让 `fold_single_use_temporaries` 的 token 统计跳过字面量（数字开头）也没有改变
耗时（`folds` 51.3 ms，对照 50.5 ms）。这说明这三组 pass 的成本是"每行都要做字符串切分/比较"
这一固有开销，只能靠**合并 pass**（一次切分同时服务多个判断）解决；单点微调（去 clone、去重算、
少统计）都已逐一排除。改动因无收益已回退。

### 整样本吞吐（2026-11-02）

notepad.exe 全部 **502** 个函数逐个 `decompile --l2`（每次一个进程，含启动与打开工程）合计
**10.9 s**，平均 **22 ms/函数**；其中最大的 45 KB 函数单独约 0.6 s，即其余 501 个函数平均约 20 ms。
这说明"逐函数浏览"的体验由小函数主导，而大函数是长尾。

### 全样本 metrics 的时间成本

`metrics --max-functions 0` 在 Go 样本（3261 个函数，L1+L2 双层级渲染）上约需 **11–12 分钟**
（debug 构建），因此默认上限为 2000；只做回归时用 `--limit` 或默认值即可，需要"全口径"数字时
才用 `0`。corpus（31 个函数）则是秒级。

### 快照检查的成本（2026-11-02）

notepad 全量 502 个快照的 `golden --check` 约 **12.7 s**（debug 构建，含 576 KB 的那个巨型函数），
Go 的 60 个快照约 **0.6 s**。快照集扩到全部函数后，`--check` 的耗时主要由巨型函数主导，
其余 501 个函数合计约 2 s。