# 打包与发布（DESIGN §13）

## 产物

`scripts/package.ps1` 生成：

| 文件 | 说明 |
|---|---|
| `dist/ReCraft-<version>/` | 绿色目录：`recraft.exe`、`recraft-ui.exe`、`LICENSE`、`README.md`、`CHANGELOG.md`、`VERSION.txt`、`docs/*.md` |
| `dist/recraft-v<version>-windows-x64.zip` | 便携 zip（当前约 10 MiB；含 rustls/ureq TLS） |
| `dist/SHA256SUMS.txt` | zip 与两个 exe 的 SHA-256 |
| `dist/recraft-<version>-setup.exe` | 可选 NSIS 安装器（本机有 `makensis` 时生成） |

```powershell
pwsh -File scripts/package.ps1                 # release build + zip +（可选）安装器
pwsh -File scripts/package.ps1 -NoBuild        # 复用 target\release
pwsh -File scripts/package.ps1 -NoInstaller    # 仅 zip（CI）
pwsh -File scripts/package.ps1 -RequireInstaller  # makensis 缺失则失败
```

版本来自 workspace `Cargo.toml` 的 `version`；可用 `-Version` 覆盖。

## 安装器（可选）

`packaging/installer.nsi`：

- 每用户安装到 `%LOCALAPPDATA%\ReCraft`，开始菜单快捷方式，HKCU 卸载项；
- 需要 NSIS 的 `makensis`（`choco install nsis` / `winget install NSIS.NSIS`）；
- 本仓库 CI 只构建 **zip**（runner 未安装 NSIS）；发布前在本地/发布机生成并验证安装器。

```powershell
makensis /DVERSION=0.1.0 /DSTAGE="dist\ReCraft-0.1.0" /DOUT="dist\recraft-0.1.0-setup.exe" packaging\installer.nsi
```

## CI

`package` job（windows-latest）：`cargo build --release` → `scripts/package.ps1 -NoInstaller`
→ 上传 `recraft-windows-x64` artifact（zip + SHA256SUMS）。

## 本机验证（2026-10-24，M2/MCP/CLI 功能收尾后重建）

```
[package] zip: dist\recraft-v0.1.0-windows-x64.zip (10.2 MiB)
[package] staged: dist\ReCraft-0.1.0
[package] SHA256: dist\SHA256SUMS.txt
[package] PASS
[verify] SHA256SUMS matches recraft-v0.1.0-windows-x64.zip
[verify] package entries present (11 checked)
[verify] analyze/decompile/AI/metrics OK (functions=502)
[verify] PACKAGE PASS
[release demo] dist\ReCraft-0.1.0\recraft.exe → DEMO PASS
[release metrics] notepad: 502 functions, coverage 98.5%, structured 407/502 (81.1%)
```

zip 内 23 个条目（2 个 exe + 19 篇 docs/ADR + LICENSE/README/CHANGELOG/VERSION）；
`scripts/verify-package.ps1` 会校验 SHA256SUMS、解压后检查必需条目，并用**解压出来的
exe** 跑 info/import/analyze/decompile/ai provider/metrics。CI 的 package job 在打包后
自动执行该脚本；`scripts/demo.ps1` 在 release 产物上复跑全部 9 步演示。2026-10-24 重建
后的产物指标与源码构建一致（notepad 覆盖率 98.5%、结构化 81.1%）。

## 安装器验证状态（诚实记录）

- 本开发会话无交互式 UAC：`winget install NSIS.NSIS` 与静默安装 NSIS setup 均在提权
  阶段被取消（`0x800704c7`），`makensis` 不可用，因此 **NSIS 安装器尚未在本环境生成/实测**。
- 绿色 zip 是当前正式发布产物（已自动验证）；安装器脚本保留，发布前需在允许提权的机器或
  干净虚拟机上执行：
  ```powershell
  choco install nsis -y          # 或 winget install NSIS.NSIS
  pwsh -File scripts/package.ps1
  dist\recraft-0.1.0-setup.exe /S /D=%TEMP%\ReCraftInstall   # 静默安装到自定义目录
  # 从开始菜单/安装目录启动 recraft-ui.exe，再运行 uninstall.exe /S 验证卸载
  ```
- 替代方案（后续）：MSIX 打包 + 代码签名；在此之前以 zip + SHA256SUMS 分发。

## 发布清单

1. `cargo test --workspace`、`scripts/e2e.ps1`、`scripts/bench-large.ps1` 全绿。
2. 更新 `CHANGELOG.md` 与 `docs/M*.md` 状态。
3. `pwsh -File scripts/package.ps1`（如需安装器先装 NSIS）。
4. 校验 `SHA256SUMS.txt`；上传 zip/安装器/校验和。
5. 明确 AGPL-3.0-only 与商业许可入口（`LICENSE`/`README`），**不包含**任何样本二进制。
6. 安装器发布前在干净 Win10/Win11 虚拟机各跑一遍（安装→启动 UI→卸载）。

## 限制

- 未做代码签名（需证书）；SmartScreen 会提示。
- 未做增量/自动更新；未做 MSI。
- 安装器脚本随 NSIS 版本可能有细节差异，首次发布前需实测。
