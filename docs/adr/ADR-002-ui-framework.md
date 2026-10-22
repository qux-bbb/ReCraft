# ADR-002: 原生 UI 框架选型（egui）

- 状态：**已接受**（2026-10-22，spike 通过）
- 相关：DESIGN §8、ADR-001
- 实验代码：`spike/egui-grid`（一次性，M0 结束删除，不入库）

## 背景

ReCraft 的 UI 需要同时满足：

1. 十几万到上百万行的多列反汇编网格（地址/字节/助记符/操作数/注释），逐 token 着色、
   命中、跨行选区复制、内联编辑；
2. 控制流图（数百到数千节点）的绘制、拖拽、缩放；
3. 多面板停靠、键盘优先的操作流、深色主题；
4. 中文注释与输入法；
5. 全 Rust、无 webview（项目所有者明确要求）、MIT/Apache 许可（AGPL + 商业双许可兼容）；
6. 单人、每周 3 天的投入，迭代速度优先。

候选：egui / iced / Tauri（web） / Qt(cxx-qt) / Slint / GPUI。

## 决策

使用 **egui 0.36.2 + eframe 0.36.2 + egui_dock 0.21.1**，自绘反汇编网格与 CFG。

### 版本与渲染后端（重要）

- egui 0.37 已发布，但 **egui_dock 0.21.1 只接受 egui ^0.36**，因此整套栈锁在 0.36.2，
  直到 dock 跟进。升级前先确认兼容组合，不要单独升 egui。
- **默认渲染后端用 Glow（OpenGL）**：eframe 默认的 wgpu/DX12 在本机（Intel UHD 750，
  驱动 30.0.100.9805 + 虚拟显示适配器）启动即 0xC0000005 崩溃，发生在业务代码之前。
  Glow 正常；保留 `SPIKE_RENDERER=wgpu` 可切换，产品里应做成配置项并对崩溃做降级提示。
- egui_dock 0.21 依赖 egui 0.36 的 root-Ui/panel 重构，说明 egui 面板 API 仍在演进，
  升级需要回归网格、停靠布局与快捷键。

## 证据（headless spike，2026-10-22）

| 指标 | 结果 |
|---|---|
| egui / eframe | 0.36.2 / 0.36.2 |
| egui_dock / egui_extras | 0.21.1 / 0.36.2 |
| 网格行数 | 200,000 |
| 网格帧时间 | p50 0.45 ms / p95 0.62 ms / max 0.89 ms |
| CFG 节点数 | 500（分层布局一次 0.33 ms） |
| 图帧时间 | p50 1.54 ms / p95 2.43 ms / max 2.86 ms |
| 帧预算 16.67 ms | 两项 PASS（p95 约预算的 4%–15%） |
| CJK 字体 | 加载成功（msyh.ttc），字形覆盖 true |
| 输入合成检查 | 点击命中 PASS / 拖拽选区 PASS / Ctrl+C 复制 PASS |
| fmt / clippy -D warnings / 8 个单测 | 全部通过 |

关键结论：**只对可见行做布局**时，200k 行数据本身不构成负担；egui 的即时模式自绘
非常适合"格子 + 图"这种非标准控件。

## 风险与保留

1. **headless 只测了 CPU 布局 + tessellation**，不含 GPU 上传/合成/vsync。真实滚动手感
   需要本机开一次 GUI 验证（`cargo run --release`）。
2. **IME 输入**无法无头验证，只能人工确认；这是 egui 相对 Qt 的弱项。
3. 文本编辑器级别的精细交互（多光标、复杂选区、无障碍）egui 较弱；ReCraft 的编辑面很窄
   （改名/注释单行输入），可接受。
4. 规模上限：500 节点已验，2,000 节点未验；M3 前用真实样本补测。
5. egui API 仍在演进，升级需回归网格与图。

## 过程中发现并修复的真实缺陷

- **焦点 id 错误**：网格原先对自造的 `ui.id().with("disasm_grid")` 调 `request_focus`，
  但该 id 没有控件认领，egui 下一帧会清掉焦点 → 真实 UI 里 Ctrl+C 永远不生效。
  修复：使用 `response.id`。已写入 T9 的实现注意事项。
- 无头测试需显式发送 `Event::ModifiersChanged` 才会更新 `InputState::modifiers`
  （egui 0.36 的行为），这是测试基建陷阱，不是 UI bug。

## 被否方案

| 方案 | 否因 |
|---|---|
| Tauri（web） | 违反"不要 webview"；双语言与 IPC 复杂度 |
| Qt / cxx-qt | LGPL 许可与商业版冲突；C++ 构建链；迭代慢 |
| Slint | UI 许可 GPLv3/商业；密集文本控件弱 |
| iced | 生态小，文本编辑控件不成熟，仍需自绘网格 |
| GPUI | API 不稳定、文档少、许可证关系需核实 |

## 后果

- T9 复用本 spike 的网格思路（可见行布局、token 跨度、命中测试），但按产品架构重写：
  命令总线、主题、i18n、真实数据分页。
- `spike/egui-grid` 不入库、M0 收尾时删除；测量数据保留在本 ADR。
- 若后续真实 GUI 手感不达标（GPU/IME），触发备选：Tauri（重开 ADR）。
