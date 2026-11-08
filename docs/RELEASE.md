# 打包与发布（DESIGN §13）

## 产物

`scripts/package.ps1` 生成：

| 文件 | 说明 |
|---|---|
| `dist/ReCraft-<version>/` | 绿色目录：`recraft.exe`（GUI）、`recraft-cli.exe`（命令行）、`LICENSE`、`NOTICE.md`、`README.md`、`CHANGELOG.md`、`VERSION.txt`、`docs/`（含 `adr/`、`cases/`） |
| `dist/recraft-v<version>-windows-x64.zip` | 便携 zip（约 13.9 MiB；含 rustls/ureq TLS 与整棵 docs） |
| `dist/SHA256SUMS.txt` | zip、两个 exe 与 MSI（若生成）的 SHA-256 |
| `dist/recraft-v<version>-windows-x64.msi` | WiX v5 MSI 安装器（仓库内固定 WiX 5.0.2 可用时生成；CI msi job 必建） |

```powershell
pwsh -File scripts/package.ps1                 # release build + zip +（检测到 WiX 时）MSI
pwsh -File scripts/package.ps1 -NoBuild        # 复用 target\release
pwsh -File scripts/package.ps1 -NoMsi          # 仅 zip（无 dotnet/WiX 的机器；CI package job）
pwsh -File scripts/package.ps1 -RequireMsi     # WiX 5 缺失则失败（CI msi job）
pwsh -File scripts/verify-package.ps1          # zip + MSI 的哈希/条目/包内二进制
pwsh -File scripts/verify-msi.ps1              # MSI：结构+载荷+安装/卸载（需提权提示符）
```

版本来自 workspace `Cargo.toml` 的 `version`；可用 `-Version` 覆盖。`dist` 里上一次运行的
`setup.exe` / `*.msi` 会在构建前清掉，避免留下 `SHA256SUMS.txt` 没有覆盖的陈旧产物。

## MSI 安装器

`packaging/wix/recraft.wxs`（WiX v5）：布局参考 [storytold/photocraft](https://github.com/storytold/photocraft)
的 `packaging/windows/photocraft.wxs`（自有向导横幅/背景图、开始菜单快捷方式、
“应用和功能”登记项、文件关联、安装结束可选启动），按本仓库的双 exe 结构重写。

- **按机器安装**（`Scope="perMachine"`）：默认 `%ProgramFiles%\ReCraft`，一次 UAC 提示；
  开始菜单快捷方式、HKLM 卸载登记项；`.rcraft` 工程关联（GUI 接受 argv[1]，见
  `crates/recraft-ui/src/main.rs`）、`recraft.exe` 的 App Paths（Win+R 输入 `recraft`）。
  免提权的按用户安装由便携 zip 承担——Windows Installer 的 per-user 模式在管理员账户下
  会被改判为 per-machine，为此专门做一套 ALLUSERS 策略不值得。
- 启动条件会检查 `%SystemRoot%\System32\vcruntime140.dll`：GUI 动态链接 MSVC 运行时，
  缺 VC++ 2015–2022 可再发行组件时安装会直接给出下载地址，而不是装完打不开。
- 生成：仓库内固定 WiX **5.0.2**（`.config/dotnet-tools.json`，`dotnet tool restore`），
  用 `dotnet wix` 调用；WiX 6/7 需要接受 OSMF EULA，故不采用。UI/Util 扩展由
  `scripts/package.ps1` 自动 `wix extension add -g`。也可手动：
  ```powershell
  dotnet wix build packaging\wix\recraft.wxs -arch x64 `
    -ext WixToolset.UI.wixext -ext WixToolset.Util.wixext `
    -d MsiVersion=0.1.0 -d DisplayVersion=0.1.0 -d BinDir=dist\ReCraft-0.1.0 `
    -d DocsDir=docs -d IconPath=assets\recraft.ico `
    -d BannerBmp=... -d DialogBmp=... -o dist\recraft-v0.1.0-windows-x64.msi
  ```
- 静默安装/卸载：`msiexec /i <msi> /qn [INSTALLFOLDER=<目录>]`；
  `msiexec /x {ProductCode} /qn`（ProductCode 见“应用和功能”登记项）。
- `scripts/verify-msi.ps1` 总是检查结构与载荷（per-machine 标记、`.rcraft` 关联、
  快捷方式、`MajorUpgrade`、运行时启动条件；`msiexec /a` 解包后与 `dist/ReCraft-<version>/`
  逐字节比对），`-SkipInstall` 用于免提权环境；完整安装→CLI→卸载流程需要提权提示符。
- `wix msi validate -ice ICE50` 由 `scripts/package.ps1` 顺带跑，仅告警不拦截。

## CI

- `package` job（windows-latest）：release 构建 → `scripts/package.ps1 -NoMsi` →
  `verify-package.ps1` → 上传 `recraft-windows-x64`（zip + SHA256SUMS）。
- `msi` job（windows-latest，runner 自身已提权）：release 构建 → `scripts/package.ps1
  -RequireMsi`（`dotnet tool restore` 还原 WiX 5.0.2）→ `verify-package.ps1` →
  `verify-msi.ps1`（结构/载荷 + 安装→CLI→卸载）→ 上传 `recraft-windows-x64-msi`。

## 本机验证（2026-11-08，0.1.0 发布前重打）

上一节的包是 2026-11-03 的，落后约 150 个提交（MCP 写模式/分页、十几个反编译 pass、
UI 修复都不在里面，`VERSION.txt` 还留着已移除的商业许可字样）。0.1.0 前用 HEAD 重打：

```
[package] zip: dist\recraft-v0.1.0-windows-x64.zip (14.9 MiB)
[package] msi: dist\recraft-v0.1.0-windows-x64.msi (13.4 MiB)
[package] SHA256: dist\SHA256SUMS.txt
[verify] SHA256SUMS matches recraft-v0.1.0-windows-x64.zip
[verify] MSI SHA256 + compound-file header OK (13.4 MiB)
[verify] analyze/decompile/AI/metrics OK (functions=506)
[verify] PACKAGE PASS
[msi] structure OK (8 table checks, per-machine)
[msi] payload OK (10 files + 40 docs, byte-identical to dist\ReCraft-0.1.0)
[msi] PASS (structure + payload)          # -SkipInstall：本机提示符未提权
```

同批跑过的其它门禁（Windows 10 x64，MSVC 19.4x）：

- `demo.ps1 -Recraft dist\ReCraft-0.1.0\recraft-cli.exe` → `DEMO PASS`（AI 步骤按设计
  在未配置 provider 时跳过）
- `e2e.ps1` → `E2E PASS`；`ai-cache-smoke.ps1` → `PASS`
- `bench-large.ps1 -OverlayMB 1024` → import 8.75 s、analyze 0.57 s、峰值 1.9 MiB，`PASS`
- `lint` notepad 506 / crackme 495 → 0 findings；`golden --check` 789/789
- `cargo deny check licenses bans sources` → 全部 ok

包内 `VERSION.txt` 的 license 行已是 `AGPL-3.0-only (see LICENSE)`，`serve-mcp` 带
`--write`。完整安装→CLI→卸载仍按下一节说明在提权提示符或 CI msi job 里复验。

## 本机验证（2026-11-03，改为 MSI 单安装包后重跑）

```
[package] zip: dist\recraft-v0.1.0-windows-x64.zip (13.9 MiB)
[package] msi: dist\recraft-v0.1.0-windows-x64.msi (12.6 MiB)
[package] SHA256: dist\SHA256SUMS.txt
[verify] SHA256SUMS matches recraft-v0.1.0-windows-x64.zip
[verify] MSI SHA256 + compound-file header OK (recraft-v0.1.0-windows-x64.msi, 12.6 MiB)
[verify] analyze/decompile/AI/metrics OK (functions=502；2026-11-05 起 notepad 为 506)
[verify] PACKAGE PASS
[msi] structure OK (8 table checks, per-machine)
[msi] payload OK (10 files + 33 docs, byte-identical to dist\ReCraft-0.1.0)
[msi] PASS (structure + payload)          # -SkipInstall：本机提示符未提权
```

MSI 的安装→CLI→卸载流程需要提权的提示符：本机普通提示符下 msiexec 返回 1603，因此只跑了
`-SkipInstall` 的结构/载荷两段；提权后执行 `powershell -File scripts/verify-msi.ps1` 与 CI
的 msi job 覆盖完整流程（此前 NSIS 安装器的同流程验证记录见 git 历史与 CHANGELOG）。

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

签名恢复时需：`signtool` 签 `recraft-cli.exe`、`recraft.exe` 和 `dist\*.msi`（MSI 可直接
签名，无需额外步骤），然后重打 zip/MSI 并更新校验和；本仓库打包脚本无需改动。

## 发布清单

1. `cargo test --workspace`、`scripts/e2e.ps1`、`scripts/bench-large.ps1` 全绿。
2. 更新 `CHANGELOG.md` 与 `docs/M*.md` 状态。
3. `pwsh -File scripts/package.ps1`（MSI 需要 `dotnet tool restore`；CI msi job 自动完成）。
4. `scripts/verify-package.ps1` + 提权提示符下的 `scripts/verify-msi.ps1` 全过；
   校验 `SHA256SUMS.txt`；上传 zip/MSI/校验和。
5. 明确 AGPL-3.0-only（`LICENSE`/`README`），**不包含**任何样本二进制。
6. 代码签名后，在干净 Win10/Win11 虚拟机各复跑一次安装→启动 UI→卸载。
7. 打附注标签并推送：`git tag -a v0.1.0 -m "ReCraft 0.1.0"` +
   `git push origin main v0.1.0`；用 CHANGELOG 的 0.1.0 节作为 Release notes
   （`gh release create v0.1.0 --notes-file <utf8 临时文件> <zip> <msi> SHA256SUMS.txt`）。
   发布后把 README / ROADMAP / M4 里“公开预览待…”的状态行改成已发布版本。

## 限制

- 未做代码签名（需证书）；SmartScreen 会提示。
- 未做增量/自动更新；未做 MSIX。安装包只有 MSI 一种（按机器安装，需管理员）。
- MSI 依赖固定的 WiX 5.0.2；发布前用 CI msi job 或 `verify-msi.ps1` 复验。
