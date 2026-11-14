<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# 应用图标

## 概念

**六边形里的几何 R**：`R` 代表 ReCraft，六边形代表**二进制结构**，六个顶点上的电路节点与
一条引出的走线代表 **MCP / AI 层**。深蓝底把青色图形衬出来。

这是 2026-11-07 定下的第一版（v1）。2026-11-14 曾短暂换成 v2「字母 + 扫描线」以及 v3 的
双变体，但用户认为不如原版，**2026-11-14 恢复 v1**；v2/v3 的脚本与产物仍留在 git 历史
（`0874a32`、`966b9a1`），需要时可以再取。

## 几何与色彩

| 元素 | 参数 |
|---|---|
| 底 | 圆角方形 + 深海底渐变 `#071B26 → #0C3A46`，外加一层薄荷青光晕 |
| 纹理 | 两层很淡的六边形网格（alpha 20 / 14），让 256px 以上不空 |
| 六边形框 | 半径 760（2048 画布），描边 `rgba(95,245,224,120)`、宽 10 |
| 电路节点 | 六顶点各一个 `rgba(150,255,238,210)` 圆点，半径 17；从右上节点引一条走线到 R |
| 字母 R | 几何骨架（竖笔 + 粗圆环碗 + 斜腿），薄荷到青色渐变，带柔和外发光 |
| 亮点 | 斜腿上一个高亮节点，暗示"光标下的那一处" |

画布 2048 绘制、1024 输出（PNG），ICO 由 1024 主图重采样出 16/24/32/48/64/128/256 七帧。

**已知取舍**：16–32 px 下六边形描边与节点会糊、细节发灰；这是 v1 的固有特性，接受它，
因为任务栏主要看整体轮廓与颜色，而 48px 以上（资源管理器、安装器、README、关于窗口）细节
完整。

## 文件说明

| 文件 | 用途 |
|---|---|
| `assets/generate_icon.py` | 唯一设计源；修改概念/颜色后重新生成全部产物 |
| `assets/recraft.png` | 1024×1024 展示/商店/文档用 |
| `assets/recraft.ico` | Windows 可执行文件与安装器；含 16/24/32/48/64/128/256 |
| `assets/recraft-64.rgba` | eframe 窗口图标（64×64 RGBA，避免运行时 PNG 解码） |

## 重新生成

需要 Python 3、Pillow 与 **numpy**（v1 的渐变与光晕用 numpy 计算）：

```powershell
python assets/generate_icon.py
```

脚本会覆盖上述三个产物；生成是**逐字节可复现**的（同一套 Pillow/numpy 版本下，重跑得到
相同的 PNG/ICO，已对照 SHA-256 验证）。修改后重新构建 `recraft-ui`，让 `build.rs` 把新 ICO
嵌入 exe。

## 接入位置

- `crates/recraft-ui/build.rs`：调用 Windows SDK `rc.exe` 编译 `1 ICON "assets/recraft.ico"`
  并链接进 `recraft.exe`（GUI）；
- `crates/recraft-ui/src/main.rs`：`ViewportBuilder::with_icon` 设置窗口/任务栏图标；
- `packaging/wix/recraft.wxs` + `scripts/package.ps1`：MSI 安装器使用同一 ICO（向导横幅/
  背景图由 `recraft.png` 生成，配色 `#0C3A46` 底 + `#1EB5CD` 强调）；
- 关于窗口（Help → About）显示 `assets/recraft-64.rgba`；
- README / README.en 顶部显示 `assets/recraft.png`；
- 快捷方式/资源管理器直接读取 exe 内嵌图标。

## 使用规范

- 图标四周留白至少为图标宽度的 1/8；
- 不要拉伸、旋转、改色或给 R 加描边；六边形与节点是图标的一部分，不要裁掉；
- 最小展示尺寸 16px；更小的场合只保留整体轮廓，不要再加细节；
- 深色背景直接使用；浅色背景下保留深色圆角底，不要只留图形；
- 不要把图标用在暗示与 Hex-Rays / IDA Pro 有关联的场景。

## 许可

图标作为 ReCraft 项目资产随代码以 **AGPL-3.0-only** 提供；"ReCraft"名称与 logo 的使用另受
[`TRADEMARK.md`](../TRADEMARK.md) 约束。
