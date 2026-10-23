# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 ReCraft contributors
#
# Build the M1 evaluation corpus with MSVC, without the CRT.
#
#   pwsh -File corpus/build.ps1                # x64 Release
#   pwsh -File corpus/build.ps1 -Arch x86      # also x86 (no .pdata ground truth)
#
# Output: corpus/build/<arch>/corpus.exe

[CmdletBinding()]
param(
    [ValidateSet('x64', 'x86')]
    [string]$Arch = 'x64',
    [string]$Configuration = 'Release'
)

$ErrorActionPreference = 'Stop'

$repo = Split-Path -Parent $PSScriptRoot
$source = Join-Path $PSScriptRoot 'c\functions.c'
$outDir = Join-Path $PSScriptRoot "build\$Arch"
New-Item -ItemType Directory -Force -Path $outDir | Out-Null

$exports = @(
    'add_leaf', 'branchy', 'loopy', 'switcher', 'switcher_big', 'caller', 'recursive',
    'indirect', 'nested_loops', 'tailish', 'float_math', 'bitops', 'bytes_op',
    'noinline_helper', 'uses_helper', 'corpus_banner', 'corpus_wide_banner',
    'corpus_counter_bump'
) | ForEach-Object { "/EXPORT:$_" }

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

$obj = Join-Path $outDir 'functions.obj'
$exe = Join-Path $outDir 'corpus.exe'
Remove-Item $obj, $exe, (Join-Path $outDir 'corpus.pdb') -Force -ErrorAction SilentlyContinue

$compile = 'cl /nologo /c /O2 /GS- /Zl /W3 "{0}" /Fo:"{1}"' -f $source, $obj
$link = 'link /nologo /SUBSYSTEM:CONSOLE /NODEFAULTLIB /ENTRY:corpus_main /OUT:"{0}" "{1}" {2}' -f $exe, $obj, ($exports -join ' ')
$command = 'call "{0}" >nul && {1} && {2}' -f $vcvars, $compile, $link

Write-Host "[corpus] $Arch $Configuration"
cmd /c $command
if ($LASTEXITCODE -ne 0) {
    throw "corpus build failed with exit code $LASTEXITCODE"
}
if (-not (Test-Path $exe)) {
    throw "corpus.exe was not produced"
}
$size = (Get-Item $exe).Length
Write-Host ("[corpus] built {0} ({1:N0} bytes)" -f $exe, $size)
