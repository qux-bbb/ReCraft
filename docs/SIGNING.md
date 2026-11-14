<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# 代码签名：已废弃（2026-11-14）

**结论：不做代码签名。** 用户于 2026-11-14 决定废弃整条签名线——不再申请 SignPath
Foundation、Certum 开源证书或 Azure Artifact Signing（原 Trusted Signing），也不购买
OV/EV 证书。发布物保持未签名。

## 保留的依据

- 完整性凭据是 `SHA256SUMS.txt`：每次发版都随 zip/MSI 一起生成并写进发布说明；
- README 保留 SmartScreen 操作说明（更多信息 → 仍要运行）；
- `.github/workflows/release.yml` 保留：它负责 tag 触发的打包、校验（含 MSI 静默安装/
  运行/卸载）与可选发布，与签名无关。

## 当初的调研（归档）

- 常规 OV 证书 $150–300/年，2023 起强制硬件令牌/HSM；EV $400+/年，2024 起不再绕过
  SmartScreen；
- 免费通道：SignPath Foundation（OSS 项目，需公开仓库 + 审核）；Certum 对 GitHub
  ≥1k Star 且中国国籍申请人免费；
- Azure Artifact Signing 约 $9.99/月，个人开发者仅限美国/加拿大。

这些成本与资格结论仍然成立，只是不再作为项目计划的一部分。

## 如果将来重启

`signtool` 签 `recraft-cli.exe`、`recraft.exe` 与 `dist\*.msi`（MSI 可直接签名），重打
zip/MSI 并更新校验和；打包脚本与 `release.yml` 都不需要改结构，只在 "verify MSI" 与
"publish" 之间插入签名步骤即可。
