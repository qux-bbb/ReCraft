# 打包与发布（DESIGN §13）

## 产物

`scripts/package.ps1` 生成：

| 文件 | 说明 |
|---|---|
| `dist/ReCraft-<version>/` | 绿色目录：`recraft.exe`、`recraft-ui.exe`、`LICENSE`、`README.md`、`CHANGELOG.md`、`VERSION.txt`、`docs/*.md` |
| `dist/recraft-v<version>-windows-x64.zip` | 便携 zip（当前约 9 MiB） |
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

## 本机验证（2026-10-24）

```
[package] zip: dist\recraft-v0.1.0-windows-x64.zip (9.2 MiB)
[package] SHA256: dist\SHA256SUMS.txt
[package] PASS
```

zip 内 18 个条目（2 个 exe + 10 篇 docs + ADR + LICENSE/README/CHANGELOG/VERSION）；
解压后的 `recraft.exe info notepad.exe` 正常输出 PE 头；release 产物上
`scripts/demo.ps1 -Recraft dist\ReCraft-0.1.0\recraft.exe` 输出 `DEMO PASS`。

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
