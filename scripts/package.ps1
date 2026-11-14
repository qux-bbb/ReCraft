# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 ReCraft contributors
#
# Builds the distributables (DESIGN §13):
#   dist/ReCraft-<version>/                     green directory (exe + docs + license)
#   dist/recraft-v<version>-windows-x64.zip     portable zip
#   dist/SHA256SUMS.txt                         checksums
#   dist/recraft-v<version>-windows-x64.msi     WiX v5 MSI installer (needs the pinned
#                                               `wix` dotnet tool, see .config/dotnet-tools.json)
#
#   pwsh -File scripts/package.ps1
#   pwsh -File scripts/package.ps1 -NoBuild       # reuse target\release
#   pwsh -File scripts/package.ps1 -NoMsi         # zip only (no dotnet/WiX toolchain)
#   pwsh -File scripts/package.ps1 -RequireMsi    # fail if the WiX toolchain is missing (CI)
#   pwsh -File scripts/package.ps1 -Culture en-US # English wizard instead of the zh-CN default
#
# The MSI follows the installation conventions of the PhotoCraft project
# (storytold/photocraft): the project's own wizard art drawn from assets/recraft.png, a
# Start-menu shortcut, an Apps & Features entry, and a .rcraft project association. It is a
# per-machine install; the portable zip is the no-elevation, per-user path.
#
# -Culture picks the wizard language (stock WixUI dialogs plus the strings in
# packaging\wix\<Culture>.wxl) and the MSI Language/Codepage pair that can represent it.

[CmdletBinding()]
param(
    [string]$Version = '',
    [switch]$NoBuild,
    [switch]$NoMsi,
    [switch]$RequireMsi,
    [ValidateSet('zh-CN', 'en-US')]
    [string]$Culture = 'zh-CN'
)

$ErrorActionPreference = 'Stop'

# ---- WiX helpers -------------------------------------------------------------------------------
# WiX 6/7 require accepting the Open Source Maintenance Fee EULA, so the MSI is built with the
# repo-local WiX 5.0.2 tool pinned in .config/dotnet-tools.json (`dotnet tool restore`).
$script:Wix = $null
function Get-Wix {
    if ($script:Wix) { return $script:Wix }
    if (Get-Command dotnet -ErrorAction SilentlyContinue) {
        $manifest = Join-Path $repo '.config\dotnet-tools.json'
        if (Test-Path -LiteralPath $manifest) {
            & dotnet tool restore --ignore-failed-sources 2>&1 | Out-Null
            $probe = (& dotnet wix --version 2>$null) -join ''
            if ($LASTEXITCODE -eq 0 -and $probe -match '^5\.') {
                $script:Wix = 'dotnet'
                return $script:Wix
            }
        }
    }
    if (Get-Command wix -ErrorAction SilentlyContinue) {
        $probe = (& wix --version 2>$null) -join ''
        if ($LASTEXITCODE -eq 0 -and $probe -match '^5\.') { $script:Wix = 'wix' }
    }
    return $script:Wix
}

function Invoke-Wix {
    param([string[]]$WixArgs)
    if ($script:Wix -eq 'dotnet') {
        & dotnet (@('wix') + $WixArgs)
    } else {
        & wix @WixArgs
    }
}

function Ensure-WixExtensions {
    foreach ($extension in @('WixToolset.UI.wixext/5.0.2', 'WixToolset.Util.wixext/5.0.2')) {
        $listed = (Invoke-Wix @('extension', 'list', '-g') 2>$null) -join "`n"
        $name = $extension.Split('/')[0]
        if ($listed -notmatch [regex]::Escape($name)) {
            Write-Host "[package] wix extension add $extension"
            Invoke-Wix @('extension', 'add', '-g', $extension) | Out-Null
            if ($LASTEXITCODE -ne 0) { throw "wix extension add $extension failed" }
        }
    }
}

# The setup wizard's art, drawn from the app icon so the MSI shows ReCraft rather than WiX's
# stock bitmaps: a 493x58 banner across the top of the inner pages (the page title is drawn
# over its left side, so that area stays white) and a 493x312 backdrop for the Welcome and
# Finish pages (their text sits on the right; the left 164 px are ours). Both 24-bit BMPs.
function New-InstallerArt([string]$IconPng, [string]$BannerOut, [string]$DialogOut) {
    Add-Type -AssemblyName System.Drawing
    $panelColor = [System.Drawing.ColorTranslator]::FromHtml('#0C3A46')
    $accent = [System.Drawing.ColorTranslator]::FromHtml('#1EB5CD')
    $icon = [System.Drawing.Image]::FromFile($IconPng)
    try {
        foreach ($spec in @(@($BannerOut, 493, 58), @($DialogOut, 493, 312))) {
            $bmp = New-Object System.Drawing.Bitmap $spec[1], $spec[2], ([System.Drawing.Imaging.PixelFormat]::Format24bppRgb)
            $graphics = [System.Drawing.Graphics]::FromImage($bmp)
            try {
                $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
                $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
                $graphics.Clear([System.Drawing.Color]::White)
                if ($spec[2] -eq 58) {
                    $stripe = New-Object System.Drawing.SolidBrush $accent
                    try { $graphics.FillRectangle($stripe, 0, 52, 493, 6) } finally { $stripe.Dispose() }
                    $graphics.DrawImage($icon, 493 - 46 - 8, 3, 46, 46)
                } else {
                    $panel = New-Object System.Drawing.SolidBrush $panelColor
                    $stripe = New-Object System.Drawing.SolidBrush $accent
                    try {
                        $graphics.FillRectangle($panel, 0, 0, 164, 312)
                        $graphics.FillRectangle($stripe, 160, 0, 4, 312)
                    } finally { $panel.Dispose(); $stripe.Dispose() }
                    $graphics.DrawImage($icon, 18, 64, 124, 124)
                }
            } finally { $graphics.Dispose() }
            $bmp.Save($spec[0], [System.Drawing.Imaging.ImageFormat]::Bmp)
            $bmp.Dispose()
        }
    } finally { $icon.Dispose() }
}

$repo = Split-Path -Parent $PSScriptRoot
Push-Location $repo
try {
    if (-not $Version) {
        $match = Select-String -Path 'Cargo.toml' -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1
        if (-not $match) { throw 'cannot determine workspace version from Cargo.toml' }
        $Version = $match.Matches[0].Groups[1].Value
    }

    if (-not $NoBuild) {
        Write-Host '[package] cargo build --release -p recraft-cli -p recraft-ui'
        # Windows PowerShell 5.1 raises NativeCommandError for every line cargo writes to
        # stderr, and with $ErrorActionPreference = 'Stop' that aborts a healthy build.
        # Relax the preference for this call only and keep the exit code as the verdict.
        $previous = $ErrorActionPreference
        $ErrorActionPreference = 'Continue'
        try {
            & cargo build --release -p recraft-cli -p recraft-ui
        } finally {
            $ErrorActionPreference = $previous
        }
        if ($LASTEXITCODE -ne 0) { throw 'release build failed' }
    }

    $cli = Join-Path $repo 'target\release\recraft-cli.exe'
    $ui = Join-Path $repo 'target\release\recraft.exe'
    foreach ($binary in @($cli, $ui)) {
        if (-not (Test-Path -LiteralPath $binary)) { throw "missing release binary: $binary" }
    }

    $dist = Join-Path $repo 'dist'
    # Drop installers from earlier runs first, so a build that skips the MSI cannot leave a
    # stale artifact in dist that SHA256SUMS.txt does not cover. The setup.exe pattern also
    # sweeps up NSIS installers from before the MSI replaced them.
    foreach ($pattern in @('recraft-*-setup.exe', 'recraft-*-windows-x64.msi')) {
        Get-ChildItem -LiteralPath $dist -Filter $pattern -File -ErrorAction SilentlyContinue |
            Remove-Item -Force -ErrorAction SilentlyContinue
    }
    $stage = Join-Path $dist "ReCraft-$Version"
    Remove-Item -LiteralPath $stage -Recurse -Force -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force -Path $stage | Out-Null

    Copy-Item -LiteralPath $cli -Destination (Join-Path $stage 'recraft-cli.exe')
    Copy-Item -LiteralPath $ui -Destination (Join-Path $stage 'recraft.exe')
    foreach ($file in @('LICENSE', 'NOTICE.md', 'README.md', 'CHANGELOG.md')) {
        if (Test-Path -LiteralPath $file) {
            Copy-Item -LiteralPath $file -Destination (Join-Path $stage $file)
        }
    }
    $icon = Join-Path $repo 'assets\recraft.ico'
    if (Test-Path -LiteralPath $icon) {
        Copy-Item -LiteralPath $icon -Destination (Join-Path $stage 'recraft.ico')
    }
    $docs = Join-Path $stage 'docs'
    New-Item -ItemType Directory -Force -Path $docs | Out-Null
    # The whole docs tree (docs/*.md, docs/adr, docs/cases, ...) so the zip and the MSI ship
    # the same documentation and verify-msi.ps1 can compare them byte for byte.
    Copy-Item -Path (Join-Path $repo 'docs\*') -Destination $docs -Recurse -Force

    $stamp = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    @(
        "ReCraft $Version"
        "built: $stamp"
        "target: windows-x64"
        "license: AGPL-3.0-only (see LICENSE)"
    ) | Set-Content -LiteralPath (Join-Path $stage 'VERSION.txt') -Encoding ascii

    $zipName = "recraft-v$Version-windows-x64.zip"
    $zipPath = Join-Path $dist $zipName
    Remove-Item -LiteralPath $zipPath -Force -ErrorAction SilentlyContinue
    Compress-Archive -Path (Join-Path $stage '*') -DestinationPath $zipPath -CompressionLevel Optimal
    Write-Host ('[package] zip: {0} ({1:N1} MiB)' -f $zipPath, ((Get-Item -LiteralPath $zipPath).Length / 1MB))

    $exeHash = (Get-FileHash -LiteralPath (Join-Path $stage 'recraft-cli.exe') -Algorithm SHA256).Hash
    $uiHash = (Get-FileHash -LiteralPath (Join-Path $stage 'recraft.exe') -Algorithm SHA256).Hash
    $zipHash = (Get-FileHash -LiteralPath $zipPath -Algorithm SHA256).Hash

    # ---- MSI (WiX v5) ---------------------------------------------------------------------------
    $msi = $null
    if (-not $NoMsi) {
        $wix = Get-Wix
        if ($wix) {
            Ensure-WixExtensions
            # MSI ProductVersion is numeric (major.minor.build); pre-release tags are dropped.
            $msiVersion = ($Version -split '-')[0]
            $msi = Join-Path $dist "recraft-v$Version-windows-x64.msi"
            # Wizard language: the UI extension localizes the stock dialogs, the strings
            # ReCraft adds come from packaging\wix\<Culture>.wxl. Chinese needs codepage 936
            # in the MSI string table, English the traditional 1252.
            $msiLanguage = if ($Culture -eq 'zh-CN') { 2052 } else { 1033 }
            $msiCodepage = if ($Culture -eq 'zh-CN') { 936 } else { 1252 }
            $locFile = Join-Path $repo "packaging\wix\$Culture.wxl"
            if (-not (Test-Path -LiteralPath $locFile)) { throw "missing localization file: $locFile" }

            $artDir = Join-Path $repo '.tmp\installer-art'
            New-Item -ItemType Directory -Force -Path $artDir | Out-Null
            $banner = Join-Path $artDir 'banner.bmp'
            $dialog = Join-Path $artDir 'dialog.bmp'
            New-InstallerArt (Join-Path $repo 'assets\recraft.png') $banner $dialog

            Remove-Item -LiteralPath $msi -Force -ErrorAction SilentlyContinue
            Write-Host "[package] wix build (MSI, $Culture)"
            $wixArgs = @(
                'build', (Join-Path $repo 'packaging\wix\recraft.wxs')
                '-arch', 'x64'
                '-ext', 'WixToolset.UI.wixext', '-ext', 'WixToolset.Util.wixext'
                '-culture', $Culture
                '-loc', $locFile
                '-d', "MsiLanguage=$msiLanguage"
                '-d', "MsiCodepage=$msiCodepage"
                '-d', "MsiVersion=$msiVersion"
                '-d', "DisplayVersion=$Version"
                '-d', "BinDir=$stage"
                '-d', "DocsDir=$(Join-Path $repo 'docs')"
                '-d', "IconPath=$(Join-Path $repo 'assets\recraft.ico')"
                '-d', "BannerBmp=$banner"
                '-d', "DialogBmp=$dialog"
                '-o', $msi
            )
            Invoke-Wix $wixArgs
            if ($LASTEXITCODE -ne 0) { throw 'wix build failed' }
            # WiX writes its debug symbols (.wixpdb) next to the MSI; keep them out of dist.
            Remove-Item -Force -ErrorAction SilentlyContinue ([IO.Path]::ChangeExtension($msi, '.wixpdb'))

            # ICE validation needs the Windows SDK's ICE tooling; it is advisory, not a gate.
            $validation = (Invoke-Wix @('msi', 'validate', $msi, '-ice', 'ICE50') 2>&1) -join "`n"
            if ($LASTEXITCODE -ne 0) {
                Write-Host "[package] msi validate unavailable or reported issues:`n$validation"
            }
            Write-Host ('[package] msi: {0} ({1:N1} MiB)' -f $msi, ((Get-Item -LiteralPath $msi).Length / 1MB))
        } elseif ($RequireMsi) {
            throw 'WiX 5 not found; run `dotnet tool restore` (wix 5.0.2 is pinned in .config/dotnet-tools.json)'
        } else {
            Write-Host '[package] WiX 5 not found; skipped MSI (run `dotnet tool restore`)'
        }
    }

    # Checksums are written after the optional installer so it is covered too.
    $sums = @(
        "$zipHash  $zipName"
        "$exeHash  recraft-cli.exe"
        "$uiHash  recraft.exe"
    )
    if ($msi) {
        $msiName = Split-Path -Leaf $msi
        $msiHash = (Get-FileHash -LiteralPath $msi -Algorithm SHA256).Hash
        $sums += "$msiHash  $msiName"
    }
    $sums | Set-Content -LiteralPath (Join-Path $dist 'SHA256SUMS.txt') -Encoding ascii

    Write-Host ('[package] staged: {0}' -f $stage)
    Write-Host ('[package] SHA256: {0}' -f (Join-Path $dist 'SHA256SUMS.txt'))
    Write-Host '[package] PASS'
} finally {
    Pop-Location
}
