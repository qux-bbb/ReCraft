# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 ReCraft contributors
#
# Build the C++ demangling corpus with MSVC (no CRT).
#
#   pwsh -File corpus/build-cpp.ps1
#
# Output: corpus/build/cpp/corpus-cpp.dll

[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$source = Join-Path $PSScriptRoot 'cpp\functions.cpp'
$outDir = Join-Path $PSScriptRoot 'build\cpp'
New-Item -ItemType Directory -Force -Path $outDir | Out-Null

$scriptName = 'vcvars64.bat'
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
    throw 'MSVC environment script not found (tried vswhere and VS 2022 editions)'
}

$dll = Join-Path $outDir 'corpus-cpp.dll'
Remove-Item $dll, (Join-Path $outDir 'corpus-cpp.lib'), (Join-Path $outDir 'corpus-cpp.exp') -Force -ErrorAction SilentlyContinue

$compile = 'cl /nologo /LD /O2 /GS- /GR- /Zl /EHs-c- "{0}" /link /NODEFAULTLIB /NOENTRY /OUT:"{1}"' -f $source, $dll
$command = 'call "{0}" >nul && {1}' -f $vcvars, $compile

Write-Host '[corpus-cpp] x64 Release'
cmd /c $command
if ($LASTEXITCODE -ne 0) {
    throw "corpus-cpp build failed with exit code $LASTEXITCODE"
}
if (-not (Test-Path $dll)) {
    throw 'corpus-cpp.dll was not produced'
}
Write-Host ('[corpus-cpp] built {0} ({1:N0} bytes)' -f $dll, (Get-Item $dll).Length)
