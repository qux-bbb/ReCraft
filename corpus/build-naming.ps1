# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 ReCraft contributors
#
# Build the naming-accuracy benchmark binary (see docs/AI.md).
#
#   pwsh -File corpus/build-naming.ps1            # x64
#   pwsh -File corpus/build-naming.ps1 -Arch x86  # also x86
#
# Unlike corpus/build.ps1 this binary links the CRT and real Win32 APIs: the benchmark
# measures naming from API tags, string literals and body shape, not from a CRT-free toy.
# Output: corpus/build/<arch>/naming.exe

[CmdletBinding()]
param(
    [ValidateSet('x64', 'x86')]
    [string]$Arch = 'x64'
)

$ErrorActionPreference = 'Stop'

$source = Join-Path $PSScriptRoot 'naming\naming.c'
$outDir = Join-Path $PSScriptRoot "build\$Arch"
New-Item -ItemType Directory -Force -Path $outDir | Out-Null

$scriptName = if ($Arch -eq 'x64') { 'vcvars64.bat' } else { 'vcvars32.bat' }
$vcvars = $null
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
if (Test-Path $vswhere) {
    $vsPath = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    if ($vsPath) {
        $candidate = Join-Path $vsPath "VC\Auxiliary\Build\$scriptName"
        if (Test-Path $candidate) { $vcvars = $candidate }
    }
}
if (-not $vcvars) {
    foreach ($edition in 'Community', 'Professional', 'Enterprise', 'BuildTools') {
        $candidate = "C:\Program Files\Microsoft Visual Studio\2022\$edition\VC\Auxiliary\Build\$scriptName"
        if (Test-Path $candidate) { $vcvars = $candidate; break }
    }
}
if (-not $vcvars) {
    throw "MSVC environment script not found (tried vswhere and VS 2022 editions)"
}

$obj = Join-Path $outDir 'naming.obj'
$exe = Join-Path $outDir 'naming.exe'
Remove-Item $obj, $exe, (Join-Path $outDir 'naming.pdb') -Force -ErrorAction SilentlyContinue

$compile = 'cl /nologo /c /O2 /MT /W3 "{0}" /Fo:"{1}"' -f $source, $obj
$link = 'link /nologo /SUBSYSTEM:CONSOLE /OUT:"{0}" "{1}"' -f $exe, $obj
$command = 'call "{0}" >nul && {1} && {2}' -f $vcvars, $compile, $link

Write-Host "[naming] $Arch"
cmd /c $command
if ($LASTEXITCODE -ne 0) {
    throw "naming benchmark build failed with exit code $LASTEXITCODE"
}
if (-not (Test-Path $exe)) {
    throw "naming.exe was not produced"
}
$size = (Get-Item $exe).Length
Write-Host ("[naming] built {0} ({1:N0} bytes)" -f $exe, $size)
