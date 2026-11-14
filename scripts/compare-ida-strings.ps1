# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 ReCraft contributors
<#
.SYNOPSIS
    Compare ReCraft's string list with IDA's for the same binary.

.DESCRIPTION
    Imports and analyzes the binary with `recraft-cli`, asks IDA (headless) for
    `idautils.Strings()`, and reports how the two lists differ. Both sides are reduced to
    `(address, text)` so IDA's terminator does not count as a difference.

    The default IDA path is the portable installation used during development; pass -Ida when
    it lives somewhere else. IDA writes a database next to the binary, so the binary is copied
    into a temporary directory first.

.PARAMETER Binary
    PE or ELF file to compare.

.PARAMETER Ida
    Path to ida64.exe (default: D:\sec_tools\IDA_Pro_v8.3_Portable\ida64.exe).

.PARAMETER Keep
    Keep the temporary directory (project, IDA database, JSON dumps) for inspection.

.EXAMPLE
    .\scripts\compare-ida-strings.ps1 -Binary D:\samples\crackme.exe
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Binary,
    [string]$Ida = 'D:\sec_tools\IDA_Pro_v8.3_Portable\ida64.exe',
    [switch]$Keep
)

$ErrorActionPreference = 'Continue'
$repo = Split-Path -Parent $PSScriptRoot

if (-not (Test-Path -LiteralPath $Binary)) { throw "binary not found: $Binary" }
if (-not (Test-Path -LiteralPath $Ida)) { throw "ida64 not found: $Ida (pass -Ida)" }

$cli = Join-Path $repo 'target\release\recraft-cli.exe'
if (-not (Test-Path -LiteralPath $cli)) {
    Push-Location $repo
    & cargo build --release -p recraft-cli --quiet 2>&1 | Out-Null
    Pop-Location
    if (-not (Test-Path -LiteralPath $cli)) { throw 'recraft-cli is not built' }
}

$work = Join-Path ([System.IO.Path]::GetTempPath()) ("recraft-ida-{0}" -f ([guid]::NewGuid().ToString('N').Substring(0, 8)))
New-Item -ItemType Directory -Path $work | Out-Null
$sample = Join-Path $work (Split-Path -Leaf $Binary)
Copy-Item -LiteralPath $Binary -Destination $sample

try {
    $project = Join-Path $work 'sample.rcraft'
    & $cli import $sample -o $project 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'recraft import failed' }
    & $cli analyze $project 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'recraft analyze failed' }

    $oursJson = Join-Path $work 'recraft-strings.json'
    & $cli strings $project --json 2>&1 | Set-Content -LiteralPath $oursJson -Encoding UTF8
    if ($LASTEXITCODE -ne 0) { throw 'recraft strings failed' }

    $idaJson = Join-Path $work 'ida-strings.json'
    $dump = Join-Path $repo 'scripts\ida-strings-dump.py'
    $log = Join-Path $work 'ida.log'
    & $Ida -A "-S`"$dump $idaJson`"" "-L$log" $sample 2>&1 | Out-Null
    if (-not (Test-Path -LiteralPath $idaJson)) { throw "IDA failed; see $log" }

    $compare = Join-Path $work 'compare.py'
    Set-Content -LiteralPath $compare -Encoding UTF8 -Value @'
import json
import sys

ours = json.load(open(sys.argv[1], encoding="utf-8-sig"))
ida = json.load(open(sys.argv[2], encoding="utf-8-sig"))


def normalise(text):
    return text.rstrip("\r\n")


def address_of(row):
    value = row["addr"]
    return int(value, 16) if isinstance(value, str) else int(value)


recraft = {address_of(row): (row["len"], normalise(row["text"])) for row in ours}
reference = {}
for row in ida:
    reference[address_of(row)] = (row["len"], normalise(row.get("text", "")))

both = sorted(set(recraft) & set(reference))
only_recraft = sorted(set(recraft) - set(reference))
only_ida = sorted(set(reference) - set(recraft))
mismatched = [addr for addr in both if recraft[addr][1] != reference[addr][1]]

print("recraft strings: %d" % len(recraft))
print("ida strings:     %d" % len(reference))
print("same address:    %d  (text equal in %d)" % (len(both), len(both) - len(mismatched)))
print("only recraft:    %d" % len(only_recraft))
print("only ida:        %d" % len(only_ida))
print("")
if mismatched:
    print("text differences at a shared address (first 10):")
    for addr in mismatched[:10]:
        print("  %#x" % addr)
        print("    recraft %r" % recraft[addr][1][:88])
        print("    ida     %r" % reference[addr][1][:88])
if only_recraft:
    print("in recraft but not ida (first 10):")
    for addr in only_recraft[:10]:
        print("  %#x %r" % (addr, recraft[addr][1][:88]))
if only_ida:
    print("in ida but not recraft (first 10):")
    for addr in only_ida[:10]:
        print("  %#x %r" % (addr, reference[addr][1][:88]))
'@

    $python = 'C:\Users\q\.dsh\dsh-runtimes\dsh-primary-runtime\dependencies\python\python.exe'
    if (-not (Test-Path -LiteralPath $python)) { $python = 'python' }
    & $python $compare $oursJson $idaJson
} finally {
    if ($Keep) {
        Write-Host "[compare] kept $work"
    } else {
        Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
    }
}
