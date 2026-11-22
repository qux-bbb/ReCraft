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
# release build + zip +（检测到 WiX 时）MSI
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package.ps1
# 复用 target\release
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package.ps1 -NoBuild
# 仅 zip（无 dotnet/WiX 的机器；CI package job）
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package.ps1 -NoMsi
# WiX 5 缺失则失败（CI msi job）
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package.ps1 -RequireMsi
# zip + MSI 的哈希/条目/包内二进制
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/verify-package.ps1
# MSI：结构+载荷+安装/卸载（需提权提示符）
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/verify-msi.ps1
# 干净机：安装→CLI→GUI→卸载→清理（需提权提示符）
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/verify-install.ps1 -Msi dist\recraft-v0.3.0-windows-x64.msi
```

版本来自 workspace `Cargo.toml` 的 `version`；可用 `-Version` 覆盖。`dist` 里上一次运行的
`setup.exe` / `*.msi` 会在构建前清掉，避免留下 `SHA256SUMS.txt` 没有覆盖的陈旧产物。

## MSI 安装器

`packaging/wix/recraft.wxs`（WiX v5）：布局参考 [storytold/photocraft](https://github.com/storytold/photocraft)
的 `packaging/windows/photocraft.wxs`（自有向导横幅/背景图、开始菜单快捷方式、
“应用和功能”登记项、文件关联、安装结束可选启动），按本仓库的双 exe 结构重写。

- **按机器安装**（`Scope="perMachine"`）：默认 `%ProgramFiles%\ReCraft`，一次 UAC 提示；
  开始菜单快捷方式、HKLM 卸载登记项；`.rcraft` 工程关联（GUI 接受 argv[1]，见
  `crates/recraft-ui/src/main.rs`）、GUI 与 `recraft-cli.exe` 的 App Paths（Win+R 输入
  `recraft` / `recraft-cli`）。免提权的按用户安装由便携 zip 承担——Windows Installer 的
  per-user 模式在管理员账户下会被改判为 per-machine，为此专门做一套 ALLUSERS 策略不值得。
- **中文向导**：WixUI 自带 zh-cn，自有字符串在 `packaging/wix/zh-CN.wxl`；MSI 的
  `Language`/`Codepage` 由 `scripts/package.ps1 -Culture` 推导（zh-CN → 2052/936，
  en-US → 1033/1252，英文串在 `packaging/wix/en-US.wxl`）。发布包默认中文。
- **安装选项页**：目录页之后一页三个复选框——把 `recraft-cli` 加入系统 PATH（默认勾选）、
  创建桌面快捷方式（默认勾选）、在文件右键菜单添加“用 ReCraft 打开”（默认勾选）；静默安装用
  `ADDTOPATH=0` 关掉 PATH、`DESKTOPSHORTCUT=0` 不建桌面图标、`CONTEXTMENU=0` 不写右键菜单项。
  三个属性在 Property 表里都默认 `1`：MSI 的复选框只要属性非空就画成勾选，而组件只在
  `属性 = 1` 时安装，写 `0` 会出现“看着勾上、其实不装”的静默失效（0.2.0 的桌面快捷方式
  就是这么坏的）。
- **右键菜单**：勾选后在 `HKLM\Software\Classes\*\shell\ReCraft` 写一个经典 shell 谓词
  （`MUIVerb` 为“用 ReCraft 打开”、图标取 `recraft.exe`、命令
  `"[INSTALLFOLDER]recraft.exe" "%1"`），对任意文件类型生效，PE/ELF 样本和 `.rcraft` 工程
  都在内。它只是并列的一个动词：不改默认打开方式，也不抢 `.exe`/`.dll` 的关联。Windows 11
  的经典动词收在“显示更多选项”（Shift+F10）里；新式右键菜单要打包的 `IExplorerCommand`，
  未签名 MSI 做不到，因此不做。
- **CLI 入口**：开始菜单多一个“ReCraft 命令行”，快捷方式指向
  `cmd.exe /k cd /d [INSTALLFOLDER]`。
- **默认应用集成**：`HKLM\Software\ReCraft\Capabilities` + `Software\RegisteredApplications`，
  ReCraft 因此出现在“设置 → 默认应用”里；只登记能力，不抢 `.rcraft` 的默认项。
- 启动条件会检查 `%SystemRoot%\System32\vcruntime140.dll`：GUI 动态链接 MSVC 运行时，
  缺 VC++ 2015–2022 可再发行组件时安装会直接给出下载地址，而不是装完打不开。
- 生成：仓库内固定 WiX **5.0.2**（`.config/dotnet-tools.json`，`dotnet tool restore`），
  用 `dotnet wix` 调用；WiX 6/7 需要接受 OSMF EULA，故不采用。UI/Util 扩展由
  `scripts/package.ps1` 自动 `wix extension add -g`。也可手动：
  ```powershell
  dotnet wix build packaging\wix\recraft.wxs -arch x64 `
    -ext WixToolset.UI.wixext -ext WixToolset.Util.wixext `
    -culture zh-CN -loc packaging\wix\zh-CN.wxl `
    -d MsiLanguage=2052 -d MsiCodepage=936 `
    -d MsiVersion=0.1.0 -d DisplayVersion=0.1.0 -d BinDir=dist\ReCraft-0.1.0 `
    -d DocsDir=docs -d IconPath=assets\recraft.ico `
    -d BannerBmp=... -d DialogBmp=... -o dist\recraft-v0.1.0-windows-x64.msi
  ```
- 静默安装/卸载：`msiexec /i <msi> /qn [INSTALLFOLDER=<目录>] [ADDTOPATH=0] [DESKTOPSHORTCUT=0]
  [CONTEXTMENU=0]`；`msiexec /x {ProductCode} /qn`（ProductCode 见“应用和功能”登记项）。
- `scripts/verify-msi.ps1` 总是检查结构与载荷（24 项表断言 + 选项属性默认值守卫：per-machine、
  两个快捷方式、CLI App Paths、Capabilities/RegisteredApplications、安装选项页与
  PATH/桌面/右键菜单三个选项（桌面默认勾选）、zh-CN 2052/936、`.rcraft` 关联、`MajorUpgrade`、
  运行时启动条件；`msiexec /a` 解包后与
  `dist/ReCraft-<version>/` 逐字节比对），`-SkipInstall` 用于免提权环境。完整流程额外验证
  选项生效（PATH 条目、桌面快捷方式、右键菜单谓词及其命令、CLI 入口）以及卸载后目录/
  快捷方式/PATH/注册项的清理。
- `wix msi validate -ice ICE50` 由 `scripts/package.ps1` 顺带跑，仅告警不拦截。

## CI

- `package` job（windows-latest）：release 构建 → `scripts/package.ps1 -NoMsi` →
  `verify-package.ps1` → 上传 `recraft-windows-x64`（zip + SHA256SUMS）。
- `msi` job（windows-latest，runner 自身已提权）：release 构建 → `scripts/package.ps1
  -RequireMsi`（`dotnet tool restore` 还原 WiX 5.0.2）→ `verify-package.ps1` →
  `verify-msi.ps1`（结构/载荷 + 安装→CLI→卸载）→ 上传 `recraft-windows-x64-msi`。

## 干净机复验（2026-11-12，0.2.0 发布前）

本节关闭待办「干净 VM 安装/卸载复验（Win10/Win11）」：用当前 HEAD 重新打包的 MSI
（`scripts/package.ps1`；版本号仍为 0.1.0，SHA-256 `D30F45C5DB24F554177F0BEE3F830B8AF4CA49CAE0DAAEA974F28286FDF6D12D`）
在**干净 Win10 22H2 虚机**与**Win11 本机**各跑一遍完整流程：

安装（默认选项：PATH 开、桌面快捷方式开）→ 文件与开始菜单/注册项 → 装好的 CLI
（`--version` / `info` / `import` / `analyze` / `metrics`）→ 启动 GUI（窗口出现、截图、
`WM_CLOSE` 正常退出）→ 卸载 → 目录/快捷方式/PATH/“应用和功能”/App Paths/
Capabilities/RegisteredApplications 清理。

两段日志都来自 `scripts/verify-install.ps1`（本次新增：干净机「安装→CLI→GUI→卸载→清理」一体化
检查，含窗口枚举与截图；`verify-msi.ps1` 仍负责静默安装与表/载荷断言，两者互补）。

Win10 22H2 虚机（Microsoft Windows 10 专业版 10.0.19045.6466，VMware，4 vCPU/8 GiB）：

```
[08:43:33] user      DESKTOP-SKPNJND\alice
[08:43:33] admin     True
[08:43:33] msi sha   D30F45C5DB24F554177F0BEE3F830B8AF4CA49CAE0DAAEA974F28286FDF6D12D
[08:43:33] ok   no previous install at C:\Program Files\ReCraft
[08:43:34] ok   msiexec /i exit code (0)
[08:43:34] ok   installed payload complete (10 files; missing: )
[08:43:34] ok   Start-menu shortcuts present (ReCraft 命令行.lnk, ReCraft.lnk)
[08:43:34] ok   install folder added to the machine PATH
[08:43:34] ok   Apps & Features entry created
[08:43:34] ok   App Paths\recraft-cli.exe points at the installed CLI
[08:43:36] ok   recraft-cli analyze/metrics (functions=487, coverage=99.7%)
[08:43:36] ok   main window appeared (title: 'ReCraft')
[08:43:38] ok   window screenshot saved (...\gui-20261112-084332.png, 93.8 KiB)
[08:43:38] ok   GUI closed cleanly on WM_CLOSE
[08:43:39] ok   msiexec /x exit code (0)
[08:43:39] ok   install directory removed
[08:43:39] ok   Start-menu shortcuts removed
[08:43:39] ok   install folder removed from the machine PATH
[08:43:39] ok   Apps & Features entry removed
[08:43:39] ok   Software\ReCraft values removed
[08:43:39] ok   machine PATH restored to its pre-install value
[08:43:39] RESULT PASS
```

Win11 25H2 本机（Microsoft Windows 11 家庭版 10.0.26200）：

```
[08:46:56] admin     True
[08:46:56] ok   no previous install at C:\Program Files\ReCraft
[08:47:00] ok   msiexec /i exit code (0)
[08:47:00] ok   installed payload complete (10 files; missing: )
[08:47:00] ok   Start-menu shortcuts present (ReCraft 命令行.lnk, ReCraft.lnk)
[08:47:01] ok   recraft-cli import created a project
[08:47:02] ok   recraft-cli analyze/metrics (functions=506, coverage=99.6%)
[08:47:03] ok   main window appeared (title: 'ReCraft')
[08:47:04] ok   window screenshot saved (...\gui-20261112-084656.png, 68.7 KiB)
[08:47:05] ok   GUI closed cleanly on WM_CLOSE
[08:47:07] ok   msiexec /x exit code (0)
[08:47:07] ok   install directory removed
[08:47:07] ok   Start-menu shortcuts removed
[08:47:07] ok   Apps & Features entry removed
[08:47:07] ok   App Paths value removed
[08:47:07] ok   Capabilities values removed
[08:47:07] ok   RegisteredApplications\ReCraft removed
[08:47:07] ok   machine PATH restored to its pre-install value
[08:47:07] RESULT PASS
```

（Win10 上是 487 个函数，因为该系统的 `notepad.exe` 是老版本；本机 506 与既有基线一致。）

两个需要记住的结论：

1. **同版本号重装不会触发 `MajorUpgrade`**。本机原本装着一份 2026-11-09 的 0.1.0（en-US 包，
   装在 `D:\sec_tools\ReCraft`），本次又装了同版本的 0.1.0：两份产品并存、共享组件 GUID，
   卸载新装的那份后旧产品的 ARP 项、开始菜单快捷方式与 `HKLM\Software\ReCraft` 仍在
   （首轮 8 项失败全是这个原因，不是安装器缺陷；清掉旧产品后重跑全绿）。**发版必须 bump
   版本号**，否则 0.1.0→0.1.0 的升级路径未定义——`MajorUpgrade` 表检查通过但从不触发。
   Win10 虚机里也有一份 2026-11-08 的 0.1.0，先用 `msiexec /x {F82F758C-…}` 清掉，那次卸载
   同样 exit 0 且目录/ARP 项全消失，可算一次额外的卸载验证。
2. **自动化方式**：Win10 虚机用 VMware VIX（`vmrun runProgramInGuest`）驱动；为拿到管理员
   令牌临时关闭 guest 的 UAC（`EnableLUA=0`）并临时启用自动登录，验证后已恢复
   （`EnableLUA=1`、`ConsentPromptBehaviorAdmin=5`、`AutoAdminLogon=0`、删除 `DefaultPassword`，
   测试文件也已删除）。安装本身是 `msiexec /qn` 以管理员权限执行，与用户点一次 UAC 等价。
   本机由 `Start-Process -Verb RunAs` 发起提权，用户批准 UAC。

## 本机验证（2026-11-09，安装选项 / 中文向导）

```
[package] wix build (MSI, zh-CN)
[package] msi: dist\recraft-v0.1.0-windows-x64.msi (13.4 MiB)
[msi] structure OK (21 table checks, per-machine)
[msi] payload OK (10 files + 41 docs, byte-identical to dist\ReCraft-0.1.0)
[msi] PASS (structure + payload)          # -SkipInstall：本机提示符未提权
```

同一份源码用 `-Culture en-US`（1033/1252）也构建通过，`wix msi decompile` 可见 `OptionsDlg`、
CLI 快捷方式、PATH 环境项与 Capabilities/RegisteredApplications 表项。安装→选项生效→卸载的
运行时断言需要提权提示符，本机未提权，仍由 CI 的 msi job 覆盖。

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

## 签名决定（已废弃，2026-11-14）

**用户于 2026-11-14 决定：代码签名不做了，整条线废弃**，不再申请 SignPath Foundation、
Certum 开源证书或 Azure Artifact Signing，也不购买 OV/EV 证书。发布物保持未签名。

原有的成本/资格调研（证书价格、SignPath Foundation 与 Certum 的免费通道、Azure 个人开发者
地域限制）不再作为路线图项，仅归档在 `docs/SIGNING.md`。完整性凭据继续是
`SHA256SUMS.txt`，README 与 Release Notes 保留 SmartScreen 操作说明
（更多信息 → 仍要运行）。

若将来重启：`signtool` 签 `recraft-cli.exe`、`recraft.exe` 与 `dist\*.msi`（MSI 可直接
签名），重打 zip/MSI 并更新校验和；打包脚本本身无需改动。

## 发布清单

下面是 0.3.0 这一轮的实际清单（2026-11-22 定，版本号已经用户确认）；换版本时把 `0.3.0` /
`v0.3.0` 整体替换即可。每步都必须做完才进下一步，前一步不过不要在后一步上打补丁。

### 0. 前置

- 版本号已由用户拍板：**0.3.0**。没有这句话不动 `Cargo.toml`、不打标签、不建 Release。
- 工作区干净（`git status --short` 只剩本轮要提交的改动），本地提交全部就绪。
- `git ls-remote --tags origin` 确认远端只有 `v0.1.0` / `v0.2.0`，新标签不会与历史号冲突
  （开发中期的 0.3.x/0.4.x 是已作废的本地标签，从未推送）。

### 1. 全量门禁

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

```powershell
# 反编译器三层：lint → golden → metrics，任一层解释不通就停
# 用 debug 产物：这一轮还没打包，别误用 dist 里上一轮遗留的 release 二进制
target\debug\recraft-cli.exe lint     <project>
target\debug\recraft-cli.exe golden  <project> --dir tests/golden\<set> --check
target\debug\recraft-cli.exe metrics <project>
```

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\e2e.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\bench-large.ps1
```

`e2e.ps1` 与 `bench-large.ps1` 默认拿 `notepad.exe` 当样本（`-PEFile` 可换），自己造 1 GiB
overlay，不依赖语料；三层门禁用的语料工程由 `corpus\build.ps1` 生成（需 MSVC）。CI 对应的
job：`check`（Windows）、`core`（Linux）、`licenses`。

### 2. 版本与文档

- `Cargo.toml` 的 workspace `version` 改成 `0.3.0`：14 个 crate 全部继承，`Cargo.lock` 里
  对应条目随之刷新，改完 `cargo build -p recraft-ui -p recraft-cli` 确认 lock 已更新。
- `CHANGELOG.md`：`## [Unreleased]` 改成 `## [0.3.0] - <发布日>`，标题下补一句总述
  （照 0.2.0 段的体例），文末补链接定义
  `[0.3.0]: https://github.com/qux-bbb/ReCraft/releases/tag/v0.3.0`。
- 状态行口径：`README.md`、`README.en.md`、`docs/FAQ.md`、`docs/M4.md`（`docs/ROADMAP.md`
  的发布记录留到第 6 步，等打包有了体积再写）。
- 本文档与 README 里 dist 文件名示例（`-Msi dist\…`）跟着换成 `v0.3.0`。
- **不改**历史叙述里的旧版本号：`docs/DECBENCH.md`、`docs/UI.md`、
  `docs/adr/ADR-015-decoder-abstraction.md`、`docs/ROADMAP.md` 的波次记录都是历史事实。

### 3. 打包

```powershell
dotnet tool restore   # 仓库钉的 WiX 5.0.2
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\package.ps1 -RequireMsi
```

产物：`dist\ReCraft-0.3.0\`（绿色目录，含 `VERSION.txt`）、
`dist\recraft-v0.3.0-windows-x64.zip`、同版本 `.msi`、`dist\SHA256SUMS.txt`（含两个 exe）。
CI 侧同一件事由 `package` / `msi` 两个 job 完成。

### 4. 验证

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\verify-package.ps1
# 需提权提示符
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\verify-msi.ps1
# 干净机：安装→CLI→GUI→卸载→清理
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\verify-install.ps1 -Msi dist\recraft-v0.3.0-windows-x64.msi
# 优先用 target\release 的二进制，断言 MCP serverInfo
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\demo.ps1
```

再手工过两处本版新增项：MSI 三个复选框默认全勾且桌面图标真的出现；任意文件右键有
「用 ReCraft 打开」（Windows 11 在「显示更多选项」里）。

### 5. 推送与发布

```powershell
git push origin main
git tag -a v0.3.0 -m "ReCraft 0.3.0"
git push origin v0.3.0
gh release create v0.3.0 --notes-file <notes.md> <zip> <msi> SHA256SUMS.txt
```

- Release notes 用 CHANGELOG 的 `0.3.0` 节；临时文件必须 UTF-8 无 BOM
  （PS 5.1 的 `>` 会写成 UTF-16 或 GBK）。
- 明确 AGPL-3.0-only（`LICENSE` / `README`），发布物**不含任何样本二进制**。
- 发布物保持未签名（2026-11-14 决定），完整性凭据只有 `SHA256SUMS.txt`；
  签名若重启见上节，`signtool` 签两个 exe 与 MSI 后重打产物并更新校验和。

### 6. 发布后

- Release 页面三个附件齐全，且与 `SHA256SUMS.txt` 对得上。
- `docs/ROADMAP.md`、`docs/M4.md` 补产物体积与日期。
- 新开一个空的 `## [Unreleased]` 供下一批累积。

### 已知坑

1. **MSI 选项属性必须默认 `1`**：复选框只要属性非空就画成勾选，而组件只在 `属性 = 1`
   时安装，写 `0` 会「框勾着但不装」（0.2.0 的桌面快捷方式就是这么坏的）。`verify-msi.ps1`
   已有「不得默认 0」的守卫，别在 wxs 里改回去。
2. **本机只有 PS 5.1**：一律 `powershell -NoProfile -ExecutionPolicy Bypass -File`；
   脚本里写文件用 `[System.IO.File]::WriteAllText` + `UTF8Encoding($false)`，
   不要用 `>` / `Set-Content`（GBK 会把中文写成 mojibake）。
3. **`package.ps1` 会清 `dist` 里上一轮的 msi/setup**，但 `artifacts\` 下从 CI 下载的产物
   要手清，别混进 Release。
4. **MSI 的 `ProductVersion` 是数字三段**，预发布后缀会被截掉——发纯数字版本最省事。

**版本号由用户拍板**：`Cargo.toml` 的 `version` 每次改动、每个标签、每次 GitHub Release 前
都必须先得到用户的明确同意，不得自行推进版本号（2026-11-15 约定；此前的 0.2.0→0.4.1 中间
版本号因此被收回成单一的 0.2.0）。**0.3.0 已于 2026-11-22 经用户确认**，用于 0.2.0 之后的
这一批（列表/图形/反汇编的 IDA 对齐、导出入口点、反编译变量注释、右键动词等）。

## 限制

- 没有代码签名（2026-11-14 决定废弃）；SmartScreen 会提示，按 README 的步骤绕过。
- 未做增量/自动更新；未做 MSIX。安装包只有 MSI 一种（按机器安装，需管理员）。
- 发布 MSI 是中文向导；英文向导用 `scripts/package.ps1 -Culture en-US` 重打（产物同名，会
  覆盖中文包）。
- PATH 默认加入**系统** PATH（可用 `ADDTOPATH=0` 关闭），卸载时移除；桌面快捷方式默认创建
  （可用 `DESKTOPSHORTCUT=0` 关闭）。
- 卸载不删除 `%LOCALAPPDATA%\ReCraft`（布局/主题与最近项目），那是按用户的个人数据。
- MSI 依赖固定的 WiX 5.0.2；发布前用 CI msi job 或 `verify-msi.ps1` 复验。
