# 打包与发布（DESIGN §13）

## 产物

`scripts/package.ps1` 生成：

| 文件 | 说明 |
|---|---|
| `dist/ReCraft-<version>/` | 绿色目录：`recraft.exe`（GUI）、`recraft-cli.exe`（命令行）、`LICENSE`、`README.md`、`CHANGELOG.md`、`VERSION.txt`、`docs/*.md` |
| `dist/recraft-v<version>-windows-x64.zip` | 便携 zip（约 10.3 MiB；含 rustls/ureq TLS 与 docs/adr） |
| `dist/SHA256SUMS.txt` | zip、两个 exe 与安装器（若生成）的 SHA-256 |
| `dist/recraft-<version>-setup.exe` | NSIS 安装器（检测到 `makensis` 时生成；CI installer job 必建） |

```powershell
pwsh -File scripts/package.ps1                 # release build + zip +（检测到 NSIS 时）安装器
pwsh -File scripts/package.ps1 -NoBuild        # 复用 target\release
pwsh -File scripts/package.ps1 -NoInstaller    # 仅 zip（无 NSIS 的 CI/机器）
pwsh -File scripts/package.ps1 -RequireInstaller  # makensis 缺失则失败（CI installer job）
pwsh -File scripts/verify-installer.ps1        # 安装→CLI/GUI→卸载 全自动验证
```

版本来自 workspace `Cargo.toml` 的 `version`；可用 `-Version` 覆盖。

## 安装器

`packaging/installer.nsi`（NSIS 3.x）：

- **每用户安装**（`RequestExecutionLevel user`，无需 UAC）：默认 `%LOCALAPPDATA%\ReCraft`，
  `/D=<dir>` 可覆盖；开始菜单快捷方式；HKCU 卸载登记项（`DisplayName`/`DisplayVersion`/
  `Publisher`/`DisplayIcon`/`InstallLocation`/`UninstallString`/`QuietUninstallString`）。
- 生成：先装 NSIS（`winget install NSIS.NSIS` 或 `choco install nsis -y`），
  `scripts/package.ps1` 会自动在 `Program Files (x86)\NSIS` 找到 `makensis`；
  也可直接手动调用：
  ```powershell
  makensis /DVERSION=0.1.0 /DSTAGE="dist\ReCraft-0.1.0" /DOUT="dist\recraft-0.1.0-setup.exe" packaging\installer.nsi
  ```
- 静默安装/卸载：`recraft-0.1.0-setup.exe /S [/D=目录]`；`<安装目录>\uninstall.exe /S`。
- 安装器本身是 PEG（PE GUI）自解压包，`scripts/verify-package.ps1` 会校验其 SHA-256 与
  MZ/PE 头。

## CI

- `package` job（windows-latest）：release 构建 → `scripts/package.ps1 -NoInstaller` →
  `verify-package.ps1` → 上传 `recraft-windows-x64`（zip + SHA256SUMS）。
- `installer` job（windows-latest）：`choco install nsis` → `scripts/package.ps1
  -RequireInstaller` → `verify-package.ps1`（含安装器哈希）→ `verify-installer.ps1`
  （静默安装、CLI/GUI 启动、卸载清理）→ 上传 `recraft-windows-x64-installer`。

## 本机验证（2026-10-24，安装器补齐后重建）

```
[package] zip: dist\recraft-v0.1.0-windows-x64.zip (10.3 MiB, 33 entries incl. docs/adr)
[package] installer: dist\recraft-0.1.0-setup.exe (10.3 MiB)
[package] SHA256: dist\SHA256SUMS.txt
[verify] SHA256SUMS matches recraft-v0.1.0-windows-x64.zip
[verify] installer SHA256 + PE header OK (recraft-0.1.0-setup.exe, 10.3 MiB)
[verify] analyze/decompile/AI/metrics OK (functions=502)
[verify] PACKAGE PASS
[release demo] dist\ReCraft-0.1.0\recraft-cli.exe → DEMO PASS
[release metrics] notepad: 502 functions, coverage 99.4%, structured 432/502 (86.1%)
[installer] PASS：silent install → files/shortcut/registry → installed CLI（502 函数、
            覆盖率 99.4%）→ GUI 启动（窗口标题 ReCraft）→ silent uninstall →
            安装目录/快捷方式/注册表全部移除（无残留）
```

`scripts/verify-installer.ps1` 在临时目录执行上述全流程并在 `finally` 中兜底清理；
本机（Windows 10、无提权提示）与默认 `%LOCALAPPDATA%\ReCraft` 路径均验证通过。
CI installer job 运行同一脚本。注意：这是开发机验证，发布前仍建议在干净 Win10/Win11
虚拟机各跑一次（脚本一条命令可复现）。

## 安装器验证记录（2026-10-24）

- NSIS 3.13 通过 `winget install NSIS.NSIS` 安装成功（此前会话因 UAC 取消失败，本轮环境
  允许）；`makensis` 位于 `C:\Program Files (x86)\NSIS\makensis.exe`。
- 安装器无需提权即可安装到 `%LOCALAPPDATA%`，HKCU 卸载项 + 开始菜单快捷方式齐全；
  卸载后目录、快捷方式、注册表键均被删除。
- 仍缺：**干净虚拟机复验**与**代码签名**（见限制）。

## 签名决定（v0.1：暂不签名）

v0.1 预览发布**不进行代码签名**（用户已确认）。原因与后续路径：

- 常规 OV 证书 $150–300/年（且 2023 起强制硬件令牌/HSM），EV $400+/年且 2024 起不再
  绕过 SmartScreen；Azure Artifact Signing（原 Trusted Signing）约 $9.99/月，但个人开发者
  仅限美国/加拿大，当前不可用。
- 开源项目有免费通道：**SignPath Foundation**（免费 OSS 签名，OV 级、HSM、CI 集成，需
  公开仓库+申请审核）；Certum 开源证书对 GitHub ≥1k Star 且中国国籍申请人免费。
  这两条作为后续（公开仓库、申请通过后）的签名路径。
- 在此之前以 `SHA256SUMS.txt` 作为完整性凭据，并在 README/Release Notes 说明 SmartScreen
  操作（更多信息 → 仍要运行）。

签名恢复时需：`signtool` 签 `recraft-cli.exe`、`recraft.exe`、`setup.exe`，NSIS 卸载器用
`!uninstfinalize` 一并签，然后重打 zip/安装器并更新校验和；本仓库打包脚本无需改动。

## 发布清单

1. `cargo test --workspace`、`scripts/e2e.ps1`、`scripts/bench-large.ps1` 全绿。
2. 更新 `CHANGELOG.md` 与 `docs/M*.md` 状态。
3. `pwsh -File scripts/package.ps1`（安装器需先装 NSIS；CI installer job 自动完成）。
4. `scripts/verify-package.ps1` + `scripts/verify-installer.ps1` 全过；
   校验 `SHA256SUMS.txt`；上传 zip/安装器/校验和。
5. 明确 AGPL-3.0-only 与商业许可入口（`LICENSE`/`README`），**不包含**任何样本二进制。
6. 代码签名后，在干净 Win10/Win11 虚拟机各复跑一次安装→启动 UI→卸载。

## 限制

- 未做代码签名（需证书）；SmartScreen 会提示。
- 未做增量/自动更新；未做 MSI/MSIX。
- 安装器为 NSIS 生成，跨 NSIS 版本可能有布局细节差异；发布前用 CI installer job 或
  `verify-installer.ps1` 复验。
