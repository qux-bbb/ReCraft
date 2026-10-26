# ReCraft 应用图标

> 状态：v1（2026-10）  
> 源文件：[`assets/generate_icon.py`](../assets/generate_icon.py)  
> 产物：`assets/recraft.png`、`assets/recraft.ico`、`assets/recraft-64.rgba`

## 概念

图标把 ReCraft 的三层含义压缩成一个可缩放的几何标记：

| 元素 | 含义 |
|---|---|
| 几何 **R** | ReCraft 的首字母；粗笔画、圆角，强调“工具”而不是字体装饰 |
| 六边形 | 二进制/数据结构与逆向工程的分析网格；也是“样本被拆解再重组”的隐喻 |
| 电路节点与连线 | MCP、AI 辅助和跨工具连接；节点落在六边形顶点上，表示分析结果可追溯 |
| 深青底色 | 终端、二进制与离线分析的基调，同时避免常见的“纯黑黑客风” |
| 薄荷/青色 R | 可读性、信号与“从数据中提取结构”的高亮色 |

设计目标：在 **16px 任务栏图标**下仍能认出 R，在 256px 下能看出六边形与节点细节。

## 几何与色彩

- 画布：1024×1024（生成时 2048 超采样后缩小）；
- 圆角方形底：圆角半径 330/2048，透明四角；
- 六边形：中心对齐，外接圆半径约 760/2048，线宽 10；
- R：笔画宽约 220/2048，圆角端帽；腿末端嵌入一个亮色节点；
- 主色：
  - 背景渐变：`#071B26` → `#0C3A46`
  - R 渐变：`#B2FFEB` → `#1EB5CD`
  - 六边形/节点：`#5FF5E0`（不同透明度）
  - 节点内芯：`#AAFFEF`
- 所有元素由 Pillow 图元绘制，不依赖第三方字体或图库素材。

## 文件说明

| 文件 | 用途 |
|---|---|
| `assets/generate_icon.py` | 唯一设计源；修改概念/颜色后重新生成全部产物 |
| `assets/recraft.png` | 1024×1024 展示/商店/文档用 |
| `assets/recraft.ico` | Windows 可执行文件与安装器；含 16/24/32/48/64/128/256 |
| `assets/recraft-64.rgba` | eframe 窗口图标（64×64 RGBA，避免运行时 PNG 解码） |

## 重新生成

需要 Python 3、Pillow、numpy：

```powershell
python assets/generate_icon.py
```

生成脚本会覆盖上述三个产物。修改后请重新构建 `recraft-ui`，让 `build.rs` 把新 ICO 嵌入 exe。

## 接入位置

- `crates/recraft-ui/build.rs`：调用 Windows SDK `rc.exe` 编译 `1 ICON "assets/recraft.ico"` 并链接进 `recraft-ui.exe`；
- `crates/recraft-ui/src/main.rs`：`ViewportBuilder::with_icon` 设置窗口/任务栏图标；
- `packaging/installer.nsi` + `scripts/package.ps1`：安装器使用同一 ICO；
- 快捷方式/资源管理器直接读取 exe 内嵌图标。

## 使用规范

- 不要拉伸、旋转、改色或给 R 加描边；
- 最小展示尺寸建议 16px；小于 16px 时改用纯 R 的无细节版本；
- 图标四周留白至少为图标宽度的 1/8，不要贴边裁切；
- 深色和浅色背景都可使用；浅色背景下保留圆角深色底，不要去掉底色只留 R；
- 不要把图标用于暗示与 Hex-Rays / IDA Pro 有关联的场景。

## 许可

图标作为 ReCraft 项目资产随代码以 **AGPL-3.0-only** 提供；“ReCraft”名称与 logo 的使用另受
[`TRADEMARK.md`](../TRADEMARK.md) 约束。
