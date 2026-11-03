# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 ReCraft contributors
#
# Verifies a packaged release without needing the WiX toolchain:
#   - dist/*.zip exists and matches SHA256SUMS.txt
#   - the optional MSI is listed and hash-clean, with an OLE compound-file header
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

    # Optional MSI: same rule, plus the OLE compound-file signature a Windows Installer
    # package starts with (D0 CF 11 E0 A1 B1 1A E1).
    $msi = Get-ChildItem -LiteralPath $dist -Filter 'recraft-v*-windows-x64.msi' -File -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
    if ($msi) {
        $msiHash = (Get-FileHash -LiteralPath $msi.FullName -Algorithm SHA256).Hash
        $msiLine = Get-Content -LiteralPath $sumFile | Where-Object { $_ -match [regex]::Escape($msi.Name) }
        if (-not $msiLine) { throw 'MSI hash not listed in SHA256SUMS.txt' }
        $expectedMsi = ($msiLine -split '\s+')[0]
        if ($expectedMsi -ne $msiHash) { throw "MSI hash mismatch: $expectedMsi != $msiHash" }
        $magic = [System.IO.File]::ReadAllBytes($msi.FullName)[0..7]
        $ole = @(0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1)
        for ($i = 0; $i -lt $ole.Count; $i++) {
            if ($magic[$i] -ne $ole[$i]) { throw 'the MSI is not an OLE compound file' }
        }
        Write-Host "[verify] MSI SHA256 + compound-file header OK ($($msi.Name), $([math]::Round($msi.Length / 1MB, 1)) MiB)"
    } else {
        Write-Host '[verify] no MSI in dist (zip-only build)'
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
