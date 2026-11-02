# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 ReCraft contributors
#
# Builds the M3 distributables (DESIGN §13):
#   dist/ReCraft-<version>/                     green directory (exe + docs + license)
#   dist/recraft-v<version>-windows-x64.zip     portable zip
#   dist/SHA256SUMS.txt                         checksums
#   dist/recraft-<version>-setup.exe            optional NSIS installer (needs makensis)
#
#   pwsh -File scripts/package.ps1
#   pwsh -File scripts/package.ps1 -NoBuild          # reuse target\release
#   pwsh -File scripts/package.ps1 -NoInstaller      # zip only (CI)
#   pwsh -File scripts/package.ps1 -RequireInstaller # fail if makensis is missing

[CmdletBinding()]
param(
    [string]$Version = '',
    [switch]$NoBuild,
    [switch]$NoInstaller,
    [switch]$RequireInstaller
)

$ErrorActionPreference = 'Stop'

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
        & cargo build --release -p recraft-cli -p recraft-ui
        if ($LASTEXITCODE -ne 0) { throw 'release build failed' }
    }

    $cli = Join-Path $repo 'target\release\recraft-cli.exe'
    $ui = Join-Path $repo 'target\release\recraft.exe'
    foreach ($binary in @($cli, $ui)) {
        if (-not (Test-Path -LiteralPath $binary)) { throw "missing release binary: $binary" }
    }

    $dist = Join-Path $repo 'dist'
    $stage = Join-Path $dist "ReCraft-$Version"
    Remove-Item -LiteralPath $stage -Recurse -Force -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force -Path $stage | Out-Null

    Copy-Item -LiteralPath $cli -Destination (Join-Path $stage 'recraft-cli.exe')
    Copy-Item -LiteralPath $ui -Destination (Join-Path $stage 'recraft.exe')
    foreach ($file in @('LICENSE', 'README.md', 'CHANGELOG.md')) {
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
    Copy-Item -Path (Join-Path $repo 'docs\*.md') -Destination $docs -Force
    $adr = Join-Path $repo 'docs\adr'
    if (Test-Path -LiteralPath $adr) {
        Copy-Item -LiteralPath $adr -Destination $docs -Recurse -Force
    }

    $stamp = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    @(
        "ReCraft $Version"
        "built: $stamp"
        "target: windows-x64"
        "license: AGPL-3.0-only (commercial licensing available, see LICENSE)"
    ) | Set-Content -LiteralPath (Join-Path $stage 'VERSION.txt') -Encoding ascii

    $zipName = "recraft-v$Version-windows-x64.zip"
    $zipPath = Join-Path $dist $zipName
    Remove-Item -LiteralPath $zipPath -Force -ErrorAction SilentlyContinue
    Compress-Archive -Path (Join-Path $stage '*') -DestinationPath $zipPath -CompressionLevel Optimal
    Write-Host ('[package] zip: {0} ({1:N1} MiB)' -f $zipPath, ((Get-Item -LiteralPath $zipPath).Length / 1MB))

    $exeHash = (Get-FileHash -LiteralPath (Join-Path $stage 'recraft-cli.exe') -Algorithm SHA256).Hash
    $uiHash = (Get-FileHash -LiteralPath (Join-Path $stage 'recraft.exe') -Algorithm SHA256).Hash
    $zipHash = (Get-FileHash -LiteralPath $zipPath -Algorithm SHA256).Hash

    $installer = $null
    if (-not $NoInstaller) {
        $makensis = (Get-Command makensis -ErrorAction SilentlyContinue).Source
        if (-not $makensis) {
            foreach ($candidate in @(
                    (Join-Path ${env:ProgramFiles(x86)} 'NSIS\makensis.exe'),
                    (Join-Path $env:ProgramFiles 'NSIS\makensis.exe'))) {
                if ($candidate -and (Test-Path -LiteralPath $candidate)) { $makensis = $candidate; break }
            }
        }
        if ($makensis) {
            $installer = Join-Path $dist "recraft-$Version-setup.exe"
            $script = Join-Path $repo 'packaging\installer.nsi'
            & $makensis "/DVERSION=$Version" "/DSTAGE=$stage" "/DOUT=$installer" $script
            if ($LASTEXITCODE -ne 0) { throw 'makensis failed' }
            Write-Host ('[package] installer: {0} ({1:N1} MiB)' -f $installer, ((Get-Item -LiteralPath $installer).Length / 1MB))
        } elseif ($RequireInstaller) {
            throw 'makensis not found; install NSIS or pass -NoInstaller'
        } else {
            Write-Host '[package] makensis not found; skipped installer (zip is the portable artifact)'
        }
    }

    # Checksums are written after the optional installer so it can be covered too.
    $sums = @(
        "$zipHash  $zipName"
        "$exeHash  recraft-cli.exe"
        "$uiHash  recraft.exe"
    )
    if ($installer) {
        $setupName = Split-Path -Leaf $installer
        $setupHash = (Get-FileHash -LiteralPath $installer -Algorithm SHA256).Hash
        $sums += "$setupHash  $setupName"
    }
    $sums | Set-Content -LiteralPath (Join-Path $dist 'SHA256SUMS.txt') -Encoding ascii

    Write-Host ('[package] staged: {0}' -f $stage)
    Write-Host ('[package] SHA256: {0}' -f (Join-Path $dist 'SHA256SUMS.txt'))
    Write-Host '[package] PASS'
} finally {
    Pop-Location
}
