# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 ReCraft contributors
#
# Share of AArch64 instructions covered by the IR lifter's supported mnemonics,
# measured over the first N functions of a project's function list.
#
# Usage: scripts/measure-a64-lift.ps1 -Project .tmp/arm64/sieve.rcraft

param(
    [string]$Cli = '.\target\release\recraft-cli.exe',
    [string]$Project = '.tmp\arm64\sieve.rcraft',
    [int]$FunctionLimit = 100
)

$ErrorActionPreference = 'Continue'
$supported = @('mov', 'movz', 'movn', 'movk', 'add', 'sub', 'and', 'orr', 'eor', 'neg',
    'lsl', 'lsr', 'asr', 'ubfx', 'cset', 'csel', 'csinc', 'ldr', 'ldur', 'ldrb', 'ldrsb',
    'ldrh', 'ldurh', 'ldrsh', 'ldrsw', 'str', 'stur', 'strb', 'strh', 'sturh', 'ldp', 'stp',
    'cmp', 'cmn', 'tst', 'adr', 'adrp', 'b', 'bl', 'ret', 'cbz', 'cbnz', 'tbz', 'tbnz', 'nop')
$total = 0
$lifted = 0
$perMnemonic = @{}
$funcLines = & $Cli funcs $Project 2>$null
$starts = @()
foreach ($line in $funcLines) {
    if ($line -match '^(0x[0-9a-f]+)\s') { $starts += $matches[1] }
}
$checked = 0
foreach ($start in $starts) {
    if ($checked -ge $FunctionLimit) { break }
    $json = & $Cli disasm $Project --addr $start --count 4000 --json 2>$null | Out-String
    if (-not $json.Trim()) { continue }
    $parsed = $json | ConvertFrom-Json
    if (-not $parsed.lines) { continue }
    $checked++
    $previous = $null
    foreach ($insn in $parsed.lines) {
        $mnemonic = $insn.mnemonic
        # Stop at the function boundary: a `ret` or the udf padding.
        if ($mnemonic -eq 'udf') { break }
        $total++
        if ($supported -contains $mnemonic -or $mnemonic -like 'b.*') { $lifted++ }
        if ($perMnemonic.ContainsKey($mnemonic)) { $perMnemonic[$mnemonic]++ } else { $perMnemonic[$mnemonic] = 1 }
        if ($mnemonic -eq 'ret') { break }
        if ($mnemonic -eq 'b') { break }
    }
}
$ratio = if ($total -gt 0) { [math]::Round(100.0 * $lifted / $total, 1) } else { 0 }
Write-Output "functions=$checked instructions=$total lifted=$lifted ratio=$ratio%"
$perMnemonic.GetEnumerator() | Sort-Object Value -Descending | Select-Object -First 12 | ForEach-Object { "$($_.Value)`t$($_.Key)" }
