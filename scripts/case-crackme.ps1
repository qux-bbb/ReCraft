# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 ReCraft contributors
#
# End-to-end case: locate a flag-feature string, jump to its xref, decompile the
# containing function, and print the source-like L2 output.
#
# This script is sample-specific (the prototype table names the statically linked CRT
# helpers of crackme_win_x64.exe). It is a demo/reproduction helper, not a test.

param(
    [string]$Binary = 'D:\files\projects\ctf-simple-crackme\challenge\crackme_win_x64.exe',
    [string]$Project = (Join-Path $env:TEMP ("recraft-crackme-{0}.rcraft" -f [guid]::NewGuid().ToString('N').Substring(0, 8))),
    [switch]$KeepProject
)

$ErrorActionPreference = 'Continue'
$root = Split-Path -Parent $PSScriptRoot
$cli = Join-Path $root 'target\debug\recraft-cli.exe'

if (-not (Test-Path -LiteralPath $Binary)) {
    throw "binary not found: $Binary"
}
if (Test-Path -LiteralPath $Project) {
    throw "project already exists: $Project (choose a new -Project path)"
}
if (-not (Test-Path -LiteralPath $cli)) {
    Push-Location $root
    try { cargo build -q -p recraft-cli } finally { Pop-Location }
}

Write-Host '== import ==' -ForegroundColor Cyan
& $cli import --out $Project $Binary 2>$null
if ($LASTEXITCODE -ne 0) { throw 'import failed' }

Write-Host '== analyze ==' -ForegroundColor Cyan
& $cli analyze $Project | Select-Object -Last 6
if ($LASTEXITCODE -ne 0) { throw 'analysis failed' }

Write-Host '== flag-like strings ==' -ForegroundColor Cyan
$strings = & $cli strings $Project --json | ConvertFrom-Json
$candidates = $strings | Where-Object { $_.text -like '*flag*' } |
    Sort-Object { [UInt64]$_.addr }
$candidates | Select-Object addr, len, text | Format-Table -AutoSize

$flag = $candidates | Where-Object { $_.text -like 'flag{*' } | Select-Object -First 1
if (-not $flag) { throw 'no flag{...} string found' }
Write-Host ("string address: {0}" -f $flag.addr)

Write-Host '== xrefs of the flag string ==' -ForegroundColor Cyan
$xrefText = & $cli xrefs --addr $flag.addr $Project
$xrefText
$xrefAddr = [UInt64]0
foreach ($line in $xrefText) {
    if ($line -match '<-\s+0x([0-9a-fA-F]+)') {
        $xrefAddr = [Convert]::ToUInt64($Matches[1], 16)
        break
    }
}
if ($xrefAddr -eq 0) { throw 'flag string has no code xref' }

Write-Host '== containing function ==' -ForegroundColor Cyan
$start = [UInt64]0
foreach ($line in (& $cli funcs $Project)) {
    if ($line -match '^0x([0-9a-fA-F]+)\s+\.\.\s+0x([0-9a-fA-F]+)') {
        $lo = [Convert]::ToUInt64($Matches[1], 16)
        $hi = [Convert]::ToUInt64($Matches[2], 16)
        if ($xrefAddr -ge $lo -and $xrefAddr -lt $hi) { $start = $lo; break }
    }
}
if ($start -eq 0) { throw 'could not resolve the function containing the xref' }
Write-Host ("function start: 0x{0:x}" -f $start)

Write-Host '== disassembly (tail of the checker) ==' -ForegroundColor Cyan
& $cli disasm --addr ('0x{0:x}' -f $start) --count 80 $Project |
    Select-Object -Last 45

Write-Host '== name the statically linked CRT helpers (sample-specific) ==' -ForegroundColor Cyan
$prototypes = [ordered]@{
    '0x140001010' = 'int main(void)'
    '0x140001170' = 'int printf(const char* format, ...)'
    '0x140004580' = 'void* __acrt_iob_func(uint32 index)'
    '0x1400049f8' = 'int fflush(void* stream)'
    '0x140004bd4' = 'char* fgets(char* buf, int size, void* stream)'
    '0x140004cc4' = 'int puts(const char* text)'
    '0x140006fd0' = 'char* strpbrk(const char* text, const char* set)'
    '0x140016b60' = 'int strcmp(const char* left, const char* right)'
    '0x140016be0' = 'uint64 strlen(const char* text)'
}
foreach ($entry in $prototypes.GetEnumerator()) {
    & $cli prototype --addr $entry.Key --text $entry.Value $Project | Out-Null
}

Write-Host '== decompiled L2 (source-like) ==' -ForegroundColor Cyan
& $cli decompile --l2 --addr ('0x{0:x}' -f $start) $Project

if (-not $KeepProject) {
    Remove-Item -LiteralPath $Project -Recurse -Force
} else {
    Write-Host ("project kept at {0}" -f $Project)
}
