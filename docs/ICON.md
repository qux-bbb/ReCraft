# ReCraft 应用图标

> 状态：v2（2026-11-14，W1 重设计：去掉六边形外框，改为"字母 + 扫描线"）  
> 源文件：[`assets/generate_icon.py`](../assets/generate_icon.py)  
> 产物：`assets/recraft.png`、`assets/recraft.ico`、`assets/recraft-64.rgba`

## 概念

图标只保留一眼能认出的东西：一个字母，一条扫描线。

| 元素 | 含义 |
|---|---|
| 几何 **R** | ReCraft 的首字母；粗笔画、圆角端帽，强调“工具”而不是字体装饰 |
| 青色扫描线 + 字节刻度 | 光标下的那行指令与它读到的字节；也是“逐字节看”的逆向动作本身 |
| 深色圆角底 | 终端与离线分析的基调；白色字母在深浅背景、任务栏和安装器里都保持对比度 |

v1 的六边形与电路节点在 16–32px 会糊成噪点，所以 v2 把细节全部收敛成一条贯穿画面的
扫描带：**16px 下能认出 R，32px 起能看到扫描线与刻度**，不再有第三层细节。

## 几何与色彩

- 画布：1024×1024（生成时 2048 超采样后缩小）；
- 圆角方形底：圆角半径 22%（约 450/2048），透明四角，2px 高光描边；
- R：笔画宽约 220/2048，整体放大 8% 并略上移，让字面更满又不压到扫描带；
- 扫描带：位于 70% 高度，深色底带 + 青色横线 + 7 个字节刻度，另有一条暗青下沿；
- 主色：
  - 底渐变：`#1E2936` → `#0F141B`
  - R 渐变：`#F7FCFF` → `#C4DBE4`
  - 扫描线/刻度：`#40CDE0`，下沿 `#2B808F`
  - 高光描边：`#465C6E`（约 60% 不透明度）
- 所有元素由 Pillow 图元绘制，不依赖第三方字体或图库素材。

## 文件说明

| 文件 | 用途 |
|---|---|
| `assets/generate_icon.py` | 唯一设计源；修改概念/颜色后重新生成全部产物 |
| `assets/recraft.png` | 1024×1024 展示/商店/文档用 |
| `assets/recraft.ico` | Windows 可执行文件与安装器；含 16/24/32/48/64/128/256 |
| `assets/recraft-64.rgba` | eframe 窗口图标（64×64 RGBA，避免运行时 PNG 解码） |

## 重新生成

需要 Python 3 与 Pillow（不再需要 numpy）：

```powershell
python assets/generate_icon.py
```

生成脚本会覆盖上述三个产物。修改后请重新构建 `recraft-ui`，让 `build.rs` 把新 ICO 嵌入 exe。

## 接入位置

- `crates/recraft-ui/build.rs`：调用 Windows SDK `rc.exe` 编译 `1 ICON "assets/recraft.ico"` 并链接进 `recraft.exe`（GUI）；
- `crates/recraft-ui/src/main.rs`：`ViewportBuilder::with_icon` 设置窗口/任务栏图标；
- `packaging/wix/recraft.wxs` + `scripts/package.ps1`：MSI 安装器使用同一 ICO（向导横幅/
  背景图另由 `assets/recraft.png` 生成）；
- 快捷方式/资源管理器直接读取 exe 内嵌图标。

## 使用规范

- 不要拉伸、旋转、改色或给 R 加描边；扫描线是图标的一部分，不要裁掉；
- 最小展示尺寸建议 16px；16px 下只保留 R 与底色（扫描线自然淡出），不要再加细节；
- 图标四周留白至少为图标宽度的 1/8，不要贴边裁切；
- 深色和浅色背景都可使用；浅色背景下保留圆角深色底，不要去掉底色只留 R；
- 不要把图标用于暗示与 Hex-Rays / IDA Pro 有关联的场景。

## 许可

图标作为 ReCraft 项目资产随代码以 **AGPL-3.0-only** 提供；“ReCraft”名称与 logo 的使用另受
[`TRADEMARK.md`](../TRADEMARK.md) 约束。
