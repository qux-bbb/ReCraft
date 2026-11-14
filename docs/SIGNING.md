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

1. **`release.yml` 已写好（2026-11-14）**：`.github/workflows/release.yml` 在 `v*` tag 推送或
   手动 `workflow_dispatch` 时，于 Windows runner 上执行 `package.ps1 -RequireMsi` →
   `verify-package.ps1` → `verify-msi.ps1`（静默安装/运行/卸载）→ `demo.ps1`，上传
   zip/MSI/SHA256SUMS 为 artifact；只有显式 `publish=true`（或 tag 推送）才创建 GitHub
   release。tag 推送默认**只构建不发布**，避免误打的 tag 直接变成下载。签名步骤的位置已在
   文件里用注释标出（"verify MSI" 与 "publish" 之间）；
2. 在 SignPath 注册组织 → 新建 project（选 GitHub Actions 集成）→ 提交 Foundation 申请；
3. 审核通过后：在仓库 secrets 里配 `SIGNPATH_API_TOKEN` / `SIGNPATH_ORGANIZATION_ID`，把
   `release.yml` 里的签名占位替换成 `signpath/github-action-submit-signing-request`，
   签 `recraft.exe`、`recraft-cli.exe` 与 MSI，并让 release 上传签名后的文件；
4. 验证：`signtool verify /pa /v dist\...msi`；下一个补丁版确认 SmartScreen 不再提示。

## 4. 风险与备选

- 审核周期不确定（通常数周），**不阻塞 0.4.0**：0.4.0 先按现状发布，签名在获批后补到
  下一个补丁版；
- 若 Foundation 拒绝（例如要求更长的公开历史），备选：SignPath 付费计划、或
  Azure Trusted Signing（按签名次数计费）。两条路都在 `docs/ROADMAP.md` 的 P 线里记录；
- 私钥始终在 SignPath 侧，构建产物通过哈希提交签名，本地不接触证书。

## 5. 当前状态

- 2026-11-14：申请包（本文）先落库；`release.yml` 与正式提交等用户确认仓库地址与
  GitHub 组织后再做——这两项需要用户拍板，属于"外部动作"，代理不擅自提交。
