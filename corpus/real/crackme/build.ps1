# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 ReCraft contributors
#
# Build the real-world crackme fixture ("Simple Crackme v2.0", C edition) with
# MSVC x64, and optionally refresh its snapshots.
#
#   powershell -NoProfile -ExecutionPolicy Bypass -File corpus/real/crackme/build.ps1
#   powershell -NoProfile -ExecutionPolicy Bypass -File corpus/real/crackme/build.ps1 -Snapshots
#
# Output: corpus/build/real/crackme/crackme.exe  (+ crackme.map)
# The exe, map, obj and .rcraft project are build products: never commit them.
# See README.md for provenance, the pinned SHA-256 and the snapshot policy.

[CmdletBinding()]
param(
    [string]$Configuration = 'Release',
    # The case binary this fixture mirrors (docs/cases/crackme_win_x64.md).
    [string]$ExpectedSha256 = '6C84C77BFFFF5AD56123E3C2DCED03CF07E7DEAAFE9A3527E21E30FA8A38C5BC',
    # Refresh tests/golden/crackme-real from the rebuilt exe (see README.md).
    [switch]$Snapshots,
    # Override the CLI used by -Snapshots (default: target/debug/recraft-cli.exe).
    [string]$RecraftCli = ''
)

$ErrorActionPreference = 'Stop'

$repo = Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $PSScriptRoot))
$source = Join-Path $PSScriptRoot 'src\crackme.c'
$outDir = Join-Path $repo 'corpus\build\real\crackme'
$objDir = Join-Path $outDir 'obj'
New-Item -ItemType Directory -Force -Path $objDir | Out-Null

function Find-VcVars64 {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (Test-Path $vswhere) {
        $inst = & $vswhere -latest -products * `
            -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
            -property installationPath 2>$null
        if ($inst) {
            $candidate = Join-Path $inst 'VC\Auxiliary\Build\vcvars64.bat'
            if (Test-Path $candidate) { return $candidate }
        }
    }
    foreach ($edition in 'Community', 'Professional', 'Enterprise', 'BuildTools') {
        $candidate = "C:\Program Files\Microsoft Visual Studio\2022\$edition\VC\Auxiliary\Build\vcvars64.bat"
        if (Test-Path $candidate) { return $candidate }
    }
    return $null
}

$vcvars = Find-VcVars64
if (-not $vcvars) {
    throw 'vcvars64.bat not found: install "Desktop development with C++" (MSVC x64 build tools)'
}

$exe = Join-Path $outDir 'crackme.exe'
$map = Join-Path $outDir 'crackme.map'
Remove-Item $exe, $map -Force -ErrorAction SilentlyContinue

# Same flags as the case build (tools/build.ps1 in ctf-simple-crackme): static CRT,
# /O2, /Brepro for a content hash instead of a timestamp. /MAP adds the link map
# (every function symbol, including static ones) without changing the image.
$clFlags = @('/nologo', '/W3', '/O2', '/MT', '/GS-', '/DNDEBUG', '/TC', '/Brepro')
$bat = Join-Path $objDir 'build.bat'
@"
@echo off
call "$vcvars" >nul 2>&1
cd /d "$PSScriptRoot\src"
cl $($clFlags -join ' ') /Fo:"$objDir\\" /Fe:"$exe" crackme.c /link /MAP:"$map"
exit /b %ERRORLEVEL%
"@ | Set-Content -Path $bat -Encoding ascii

Write-Host "[real/crackme] $Configuration"
cmd /c "`"$bat`""
if ($LASTEXITCODE -ne 0) { throw "cl failed with exit code $LASTEXITCODE" }
Remove-Item $bat -Force -ErrorAction SilentlyContinue
if (-not (Test-Path $exe)) { throw 'crackme.exe was not produced' }

$actual = (Get-FileHash $exe -Algorithm SHA256).Hash
$size = (Get-Item $exe).Length
Write-Host ("[real/crackme] built {0} ({1:N0} bytes)" -f $exe, $size)
Write-Host ("[real/crackme] sha256 {0}" -f $actual)
if ($actual -ne $ExpectedSha256.ToUpperInvariant()) {
    Write-Warning 'rebuilt binary differs from the pinned case binary (MSVC toolchain drift).'
    Write-Warning 'The shipped snapshots in tests/golden/crackme describe the pinned binary;'
    Write-Warning 'run this script with -Snapshots to record the rebuilt binary in tests/golden/crackme-real.'
}

if ($Snapshots) {
    $cli = if ($RecraftCli) { $RecraftCli } else { Join-Path $repo 'target\debug\recraft-cli.exe' }
    if (-not (Test-Path $cli)) {
        throw "recraft-cli not found at $cli; run 'cargo build -p recraft-cli' or pass -RecraftCli"
    }
    $project = Join-Path $outDir 'crackme.rcraft'
    $golden = Join-Path $repo 'tests\golden\crackme-real'
    Remove-Item $project -Force -ErrorAction SilentlyContinue
    & $cli import $exe -o $project
    if ($LASTEXITCODE -ne 0) { throw "import failed with exit code $LASTEXITCODE" }
    & $cli analyze $project
    if ($LASTEXITCODE -ne 0) { throw "analyze failed with exit code $LASTEXITCODE" }
    # 160 snapshots: same selection the case set uses (sorted by address), small enough
    # to review and large enough to cover main/verify plus the CRT around them.
    & $cli golden $project --dir $golden --limit 160
    if ($LASTEXITCODE -ne 0) { throw "golden failed with exit code $LASTEXITCODE" }
    Write-Host "[real/crackme] snapshots written to $golden"
}
