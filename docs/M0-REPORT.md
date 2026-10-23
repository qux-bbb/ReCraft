# M0 Report — 走通骨架

> 状态：**完成**（2026-10-23）。12 项任务全部交付，M0 退出标准达成。

## 1. 退出标准核对（DESIGN §14）

| 标准 | 结果 |
|---|---|
| 能打开真实 PE，看到指令与 L0 伪代码 | ✅ `notepad.exe` / `kernel32.dll`，CLI 与 UI 均可 |
| 改名/注释后关闭重开仍然存在 | ✅ 独立进程重开验证（E2E 步骤 4–7） |
| egui spike 五条验收达标 | ✅ ADR-002；200k 行网格 p95 0.62 ms，500 节点图 p95 2.43 ms |
| 1 GB 级样本可交互 | ✅ 1 GiB overlay 导入 4.44 s；项目重开 3.1 ms |

## 2. 交付物

| crate | 能力 |
|---|---|
| `recraft-common` | `Address`/`AddressRange`/ID/统一错误类型 |
| `recraft-storage` | 单文件 SQLite 项目容器（schema v1）、256 KiB 分块镜像导入（进度/取消/SHA-256）、undo journal、query-only 读连接 |
| `recraft-format` | PE32/PE32+ 解析：头/节/导入（含延迟导入）/导出（含转发器）/TLS/`.pdata`/overlay/RVA 映射；畸形导入导出降级为 warning |
| `recraft-decode` | iced-x86 1.21 无状态解码、`FlowKind` 分类、直接分支目标、Intel 格式化、LRU 解码缓存 |
| `recraft-core` | Session、VA→文件偏移段映射、4 bit/字节 flags 分页缓存、名字/注释/函数、**命令总线 + journal 撤销/重做**、线性反汇编视图 |
| `recraft-ir` | 寄存器视图（rax/eax/ax/al 同源）、表达式 arena、x86/x64 LIR 提升、lazy flags；不支持指令降级为 intrinsic |
| `recraft-decompile` | L0 伪代码打印：label、尾调用、有符号位移、逐行源地址 |
| `recraft-cli` | `info/import/disasm/decompile/funcs/rename/unname/comment/uncomment/mark/undo/redo`，`--json`，符号/`entry` 地址解析 |
| `recraft-ui` | egui 0.36 + Glow 桌面壳：反汇编 Grid、L0 伪代码、段/函数面板、Goto/改名/注释弹窗、撤销重做、键盘流、中英切换、CJK 字体 |

## 3. 实测指标

| 项目 | 数值 |
|---|---|
| 1 GiB 镜像导入（storage bench） | 4.6 s |
| 1 GiB overlay 端到端导入（CLI） | **4.44 s** |
| 项目重开 | 3.1 ms |
| 1000 次随机 4 KiB 读 | 34 ms（≈34 µs/次） |
| 项目文件体积（1 GiB 镜像） | 1062 MiB |
| 解码吞吐（stateless） | 46.3 M insn/s |
| UI headless 帧（400 行 Grid） | 首帧 < 100 ms（测试断言），稳定帧远低于 16.67 ms |
| E2E | 9 个断言全过 + 1 GiB 冒烟 |

## 4. 测试

- workspace 共 **26 个测试套件**（单元 + 集成 + headless UI）。
- 关键覆盖：存储跨块读/取消清理/撤销重做；PE 解析正常与畸形输入；解码长度/流程/32↔64 位差异；IR 提升与 lazy flags；L0 输出确定性；命令总线持久化；UI headless 布局。
- CI：fmt + clippy（`-D warnings`）+ test + cargo-deny + **E2E 脚本**（`scripts/e2e.ps1`）。

## 5. 已知限制（M0 边界）

- **L0 伪代码**：无 SSA/变量合并/类型推断；push/pop 不跟踪 rsp；fs/gs 段、间接 call 未建模；无函数边界（靠人工指定入口）。
- **无自动分析**：没有函数识别、xref、字符串、导入 thunk 命名；函数列表为空。
- **UI 导入是同步的**：1 GiB 导入会阻塞 UI 约 4–5 s（M1 改为后台任务 + 进度）。
- **UI 视觉回归**：远程/无头会话里窗口截图读回不可靠；headless 布局测试代替，本机需人工过一遍。
- **未做**：调试器、MCP/AI、ELF/ARM64、图视图、多标签、插件、签名/FLIRT。
- **i18n**：中英文覆盖 M0 界面，尚无完整翻译工作流（M1 引 fluent）。

## 6. 复现方式

```powershell
cargo build --workspace
cargo test --workspace
pwsh -File scripts/e2e.ps1                     # 端到端
pwsh -File scripts/e2e.ps1 -LargeOverlayMB 1024 # 1 GiB 冒烟（约 5 s 导入）

cargo run -p recraft-cli -- import C:\Windows\System32\notepad.exe -o notepad.rcraft
cargo run -p recraft-cli -- disasm notepad.rcraft --addr entry --count 40
cargo run -p recraft-cli -- decompile notepad.rcraft --addr entry
cargo run -p recraft-ui -- notepad.rcraft
```

## 7. 下一步（M1）

1. **自动分析**：递归下降、函数识别（`.pdata`/导出/跳表/thunk）、xref、字符串（ASCII/UTF-16）、导入 thunk 命名。
2. **运行时恢复**：Go `pclntab`、Rust demangle（恶意样本价值最高的两个）。
3. **UI**：控制流图视图、函数/字符串/交叉引用列表、后台分析进度与取消、增量刷新。
4. **MCP/脚本**：headless JSON-RPC + MCP 工具集（读 + propose/apply 写）。
5. **性能**：1 GiB 全量分析的后台调度与内存预算验证。

风险：egui 0.36 API churn（ADR-002）；L0 到 L1 的跨度（结构恢复 + 变量/类型）；自动分析质量需要语料 + PDB ground truth 才能量化，M1 第一件事就是搭评测语料。
