# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 ReCraft contributors
#
# Verifies a packaged release without needing NSIS:
#   - dist/*.zip exists and matches SHA256SUMS.txt
#   - expected files are inside the zip (both exes, LICENSE, docs, VERSION.txt)
#   - the extracted binaries actually run: PE info, import + analyze + decompile,
#     AI provider defaults offline, metrics JSON.
#
#   pwsh -File scripts/verify-package.ps1
#   pwsh -File scripts/verify-package.ps1 -Zip dist\recraft-v0.1.0-windows-x64.zip

[CmdletBinding()]
param(
    [string]$Zip = '',
    [string]$PEFile = ''
)

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
if (-not $Zip) {
    $Zip = Get-ChildItem -Path (Join-Path $repo 'dist') -Filter 'recraft-v*-windows-x64.zip' -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1 -ExpandProperty FullName
}
if (-not $Zip -or -not (Test-Path -LiteralPath $Zip)) {
    throw 'no package found; run scripts/package.ps1 first'
}
$Zip = (Resolve-Path -LiteralPath $Zip).Path
if (-not $PEFile) {
    $PEFile = Join-Path $env:SystemRoot 'System32\notepad.exe'
}

$work = Join-Path ([System.IO.Path]::GetTempPath()) ('recraft-verify-' + [guid]::NewGuid().ToString('N').Substring(0, 8))
New-Item -ItemType Directory -Path $work | Out-Null

function Assert-Contains([string]$Text, [string]$Needle, [string]$What) {
    if (-not $Text.Contains($Needle)) {
        throw "package verification failed: $What (missing '$Needle')"
    }
}

try {
    $dist = Split-Path -Parent $Zip
    $sumFile = Join-Path $dist 'SHA256SUMS.txt'
    if (-not (Test-Path -LiteralPath $sumFile)) { throw "missing $sumFile" }
    $zipHash = (Get-FileHash -LiteralPath $Zip -Algorithm SHA256).Hash
    $sumLine = Get-Content -LiteralPath $sumFile | Where-Object { $_ -match [regex]::Escape((Split-Path -Leaf $Zip)) }
    if (-not $sumLine) { throw 'zip hash not listed in SHA256SUMS.txt' }
    $expected = ($sumLine -split '\s+')[0]
    if ($expected -ne $zipHash) { throw "zip hash mismatch: $expected != $zipHash" }
    Write-Host "[verify] SHA256SUMS matches $(Split-Path -Leaf $Zip)"

    # Optional NSIS installer: when present it must be listed and hash-clean too.
    $setup = Get-ChildItem -LiteralPath $dist -Filter 'recraft-*-setup.exe' -File -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
    if ($setup) {
        $setupHash = (Get-FileHash -LiteralPath $setup.FullName -Algorithm SHA256).Hash
        $setupLine = Get-Content -LiteralPath $sumFile | Where-Object { $_ -match [regex]::Escape($setup.Name) }
        if (-not $setupLine) { throw 'installer hash not listed in SHA256SUMS.txt' }
        $expectedSetup = ($setupLine -split '\s+')[0]
        if ($expectedSetup -ne $setupHash) {
            throw "installer hash mismatch: $expectedSetup != $setupHash"
        }
        $magic = [System.IO.File]::ReadAllBytes($setup.FullName)[0..1]
        if ($magic[0] -ne 0x4D -or $magic[1] -ne 0x5A) { throw 'installer is not a PE executable' }
        Write-Host "[verify] installer SHA256 + PE header OK ($($setup.Name), $([math]::Round($setup.Length / 1MB, 1)) MiB)"
    } else {
        Write-Host '[verify] no installer in dist (zip-only build)'
    }

    $extract = Join-Path $work 'pkg'
    Expand-Archive -LiteralPath $Zip -DestinationPath $extract -Force
    $required = @(
        'recraft.exe', 'recraft-cli.exe', 'LICENSE', 'README.md', 'CHANGELOG.md', 'VERSION.txt',
        'docs\AI.md', 'docs\DESIGN.md', 'docs\FAQ.md', 'docs\ROADMAP.md', 'docs\M4.md'
    )
    foreach ($entry in $required) {
        if (-not (Test-Path -LiteralPath (Join-Path $extract $entry))) {
            throw "missing package entry: $entry"
        }
    }
    Write-Host "[verify] package entries present ($($required.Count) checked)"

    $exe = Join-Path $extract 'recraft-cli.exe'
    $info = & $exe info $PEFile
    if ($LASTEXITCODE -ne 0) { throw 'packaged recraft-cli info failed' }
    Assert-Contains ($info -join "`n") 'PE32' 'PE header output'

    $project = Join-Path $work 'verify.rcraft'
    & $exe import $PEFile -o $project | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'packaged import failed' }
    & $exe analyze $project | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'packaged analyze failed' }
    $pseudo = & $exe decompile $project --addr entry --l2
    if ($LASTEXITCODE -ne 0) { throw 'packaged decompile failed' }
    Assert-Contains ($pseudo -join "`n") '// L2 structure recovery' 'pseudocode output'

    $provider = & $exe ai provider
    Assert-Contains ($provider -join "`n") 'state=disabled' 'AI provider defaults offline'

    $metrics = & $exe metrics $project --json
    $parsed = ($metrics -join "`n") | ConvertFrom-Json
    if ($parsed.functions -lt 1) { throw 'metrics reported no functions' }
    Write-Host ("[verify] analyze/decompile/AI/metrics OK (functions={0})" -f $parsed.functions)

    Write-Host '[verify] PACKAGE PASS'
} finally {
    Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
}
