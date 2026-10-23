# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 ReCraft contributors
#
# Build the Go evaluation corpus (keeps pclntab, which the runtime needs even when
# stripped, so `-s -w` is intentionally not used for the main artifact).
#
#   pwsh -File corpus/build-go.ps1
#
# Output: corpus/build/go/corpus-go.exe

[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
$source = Join-Path $PSScriptRoot 'go'
$outDir = Join-Path $PSScriptRoot 'build\go'
New-Item -ItemType Directory -Force -Path $outDir | Out-Null

$go = Get-Command go -ErrorAction SilentlyContinue
if (-not $go) {
    throw 'go toolchain not found on PATH'
}

Push-Location $source
try {
    # Disable inlining so every helper stays a real, named function in the binary.
    & go build -trimpath -gcflags=all=-l -o (Join-Path $outDir 'corpus-go.exe') .
    if ($LASTEXITCODE -ne 0) {
        throw "go build failed with exit code $LASTEXITCODE"
    }
} finally {
    Pop-Location
}

$exe = Join-Path $outDir 'corpus-go.exe'
if (-not (Test-Path $exe)) {
    throw 'corpus-go.exe was not produced'
}
Write-Host ("[corpus-go] built {0} ({1:N0} bytes)" -f $exe, (Get-Item $exe).Length)
