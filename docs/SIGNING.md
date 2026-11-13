<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# 代码签名：SignPath Foundation 申请包（W6）

目标：消除 Windows 上"未知发布者"的 SmartScreen 警告，让 0.4.0 的安装包带一个免费、
可验证的签名。选 SignPath Foundation，因为它是面向开源项目的免费 Authenticode/EV 渠道，
不要求购买证书，也不把私钥交给项目。

## 1. 资格核对

| SignPath Foundation 要求 | ReCraft 现状 |
|---|---|
| OSI 认可的开源许可证 | AGPL-3.0-only ✅（`LICENSE`，ADR-013） |
| 公开源码仓库、活跃维护 | github.com/…（待确认最终仓库地址）✅ |
| 项目不含恶意功能、不打包第三方闭源二进制 | 纯 Rust + 公共依赖（iced-x86 MIT、yaxpeax-arm 0BSD、egui MIT/Apache-2.0…）✅ |
| 可复现的构建流程 | `scripts/package.ps1` 一条命令产出 zip + MSI；`verify-package.ps1` 校验 ✅ |
| 发布物由 CI 构建（推荐） | 目前是本地脚本；**需要补一条 `release.yml` 在 GitHub Actions 里打包** ⚠️ |
| 维护者身份可核验 | 需要 GitHub 账号与公开联系方式 ⚠️（用户提供） |

## 2. 申请需要提交的材料

1. 项目主页与仓库地址、许可证（AGPL-3.0-only）、项目一句话简介；
2. 构建说明：Rust stable + MSVC 工具链；`cargo build --release`；`scripts/package.ps1`
   产出 `dist/recraft-v<version>-windows-x64.zip` 与 `.msi`；
3. 可执行文件清单：`recraft.exe`（GUI）、`recraft-cli.exe`、`recraft-mcp.exe`（若随包）；
4. 维护者信息与发布节奏（0.x 预发布，按 ROADMAP 波次发版）；
5. 说明签名用途：只签 `dist/` 里的自有可执行文件与 MSI，不签第三方代码。

## 3. 落地步骤

1. **补 `release.yml`**：`on: push: tags: ['v*']` → Windows runner 上 `package.ps1` →
   上传 zip/MSI 为 artifact；这一步既是发版自动化，也是 SignPath 审核时看的"构建可复现"证据；
2. 在 SignPath 注册组织 → 新建 project（选 GitHub Actions 集成）→ 提交 Foundation 申请；
3. 审核通过后：在仓库 secrets 里配 `SIGNPATH_API_TOKEN`，把签名步骤加到 `release.yml`
   （SignPath 的 `signpath/github-action-submit-signing-request`），签 `recraft.exe`、
   `recraft-cli.exe` 与 MSI；
4. 验证：`signtool verify /pa /v dist\...msi`；0.4.0 发版时确认 SmartScreen 不再提示。

## 4. 风险与备选

- 审核周期不确定（通常数周），**不阻塞 0.4.0**：0.4.0 先按现状发布，签名在获批后补到
  下一个补丁版；
- 若 Foundation 拒绝（例如要求更长的公开历史），备选：SignPath 付费计划、或
  Azure Trusted Signing（按签名次数计费）。两条路都在 `docs/ROADMAP.md` 的 P 线里记录；
- 私钥始终在 SignPath 侧，构建产物通过哈希提交签名，本地不接触证书。

## 5. 当前状态

- 2026-11-14：申请包（本文）先落库；`release.yml` 与正式提交等用户确认仓库地址与
  GitHub 组织后再做——这两项需要用户拍板，属于"外部动作"，代理不擅自提交。
