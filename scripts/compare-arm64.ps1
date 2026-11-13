# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 ReCraft contributors
#
# Compare ReCraft's AArch64 disassembly with `go tool objdump` for the first N functions.
#
# Usage:
#   scripts/compare-arm64.ps1 -Project .tmp/arm64/sieve.rcraft -Binary .tmp/arm64/sieve-linux-arm64
#
# Both files come from a Go cross-compiled sample:
#   GOOS=linux GOARCH=arm64 CGO_ENABLED=0 go build -o sieve-linux-arm64 .
#   recraft-cli import sieve-linux-arm64 -o sieve.rcraft
#   recraft-cli analyze sieve.rcraft
#
# Exits non-zero when any function's instruction addresses differ from objdump's.

param(
    [Parameter(Mandatory = $true)][string]$Project,
    [Parameter(Mandatory = $true)][string]$Binary,
    [int]$Limit = 100,
    [string]$Cli = '.\target\release\recraft-cli.exe'
)

$ErrorActionPreference = 'Continue'

# Ground truth: `go tool objdump` groups instructions under `TEXT name(SB)` headers.
$dump = & go tool objdump $Binary 2>$null
$functions = [ordered]@{}
$current = $null
foreach ($line in $dump) {
    if ($line -match '^TEXT (\S+)\(SB\)') {
        $current = $matches[1]
        $functions[$current] = [System.Collections.Generic.List[string]]::new()
        continue
    }
    if ($null -ne $current -and $line -match '^\s+\S+:\d+\s+0x([0-9a-f]+)\s+[0-9a-f]+') {
        $functions[$current].Add($matches[1])
    }
}

$checked = 0
$mismatched = 0
foreach ($name in $functions.Keys) {
    if ($checked -ge $Limit) { break }
    $addresses = $functions[$name]
    if ($addresses.Count -eq 0) { continue }
    $start = $addresses[0]
    $json = & $Cli disasm $Project --addr "0x$start" --count $addresses.Count --json 2>$null | Out-String
    if (-not $json.Trim()) {
        $mismatched++
        Write-Output "no disassembly: $name"
        continue
    }
    $parsed = $json | ConvertFrom-Json
    $ours = @($parsed.lines | ForEach-Object { ($_.addr -replace '^0x0*', '0x') })
    $theirs = @($addresses | ForEach-Object { "0x$_" })
    $checked++
    if ($ours.Count -ne $theirs.Count) {
        $mismatched++
        Write-Output "count mismatch: $name ours=$($ours.Count) objdump=$($theirs.Count)"
        continue
    }
    for ($i = 0; $i -lt $ours.Count; $i++) {
        if ($ours[$i] -ne $theirs[$i]) {
            $mismatched++
            Write-Output "addr mismatch: $name index $i ours=$($ours[$i]) objdump=$($theirs[$i])"
            break
        }
    }
}

Write-Output "checked=$checked mismatched=$mismatched total=$($functions.Count)"
if ($mismatched -ne 0) { exit 1 }
