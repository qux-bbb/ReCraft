# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 ReCraft contributors
#
# End-to-end acceptance test for the M0 walking skeleton.
#
#   pwsh -File scripts/e2e.ps1                 # default: notepad.exe
#   pwsh -File scripts/e2e.ps1 -PEFile x.exe   # custom sample
#   pwsh -File scripts/e2e.ps1 -LargeOverlayMB 1024   # 1 GiB overlay smoke
#
# Exits non-zero on the first failed assertion.

[CmdletBinding()]
param(
    [string]$PEFile = (Join-Path $env:SystemRoot 'System32\notepad.exe'),
    [int]$LargeOverlayMB = 0,
    [string]$Recraft = ''
)

$ErrorActionPreference = 'Stop'

$repo = Split-Path -Parent $PSScriptRoot
if (-not $Recraft) {
    $Recraft = Join-Path $repo 'target\debug\recraft.exe'
    if (-not (Test-Path $Recraft)) {
        Write-Host '[build] cargo build -p recraft-cli'
        Push-Location $repo
        try { & cargo build -p recraft-cli --quiet } finally { Pop-Location }
    }
}
if (-not (Test-Path $Recraft)) { throw "recraft binary not found: $Recraft" }
if (-not (Test-Path $PEFile)) { throw "PE file not found: $PEFile" }

$work = Join-Path ([System.IO.Path]::GetTempPath()) ('recraft-e2e-' + [guid]::NewGuid().ToString('N').Substring(0, 8))
New-Item -ItemType Directory -Path $work | Out-Null

# CJK test string, base64-encoded to keep this script ASCII-only: Windows
# PowerShell 5.1 mis-decodes BOM-less UTF-8 source files.
$cjkComment = [System.Text.Encoding]::UTF8.GetString(
    [Convert]::FromBase64String('572R57uc5Zue6LCD5Yid5aeL5YyW'))

function Invoke-Recraft {
    param([string[]]$Arguments, [switch]$Json)
    # Windows PowerShell turns native stderr into ErrorRecords; the CLI writes
    # progress there, so relax ErrorActionPreference and capture stderr to a file so
    # failures can report the real diagnostic instead of swallowing it.
    $ErrorActionPreference = 'Continue'
    $errFile = [System.IO.Path]::GetTempFileName()
    $output = & $Recraft $Arguments 2>$errFile
    $code = $LASTEXITCODE
    $stderr = ''
    if (Test-Path $errFile) {
        $stderr = (Get-Content $errFile -Raw -ErrorAction SilentlyContinue)
        Remove-Item $errFile -Force -ErrorAction SilentlyContinue
    }
    $ErrorActionPreference = 'Stop'
    if ($code -ne 0) {
        throw ("recraft {0} failed with exit code {1}`n{2}`n--- stderr ---`n{3}" -f ($Arguments -join ' '), $code, ($output -join "`n"), $stderr)
    }
    return (($output | Out-String).Trim())
}

function Assert-Contains([string]$Text, [string]$Needle, [string]$What) {
    if (-not $Text.Contains($Needle)) {
        throw "assertion failed: $What (missing '$Needle')`n--- output ---`n$Text"
    }
}

try {
    $project = Join-Path $work 'sample.rcraft'

    Write-Host '[1/11] import'
    $out = Invoke-Recraft @('import', $PEFile, '-o', $project)
    Assert-Contains $out 'OK:' 'import output'
    if (-not (Test-Path $project)) { throw 'project file was not created' }

    Write-Host '[2/11] info --json'
    $info = Invoke-Recraft @('info', $PEFile, '--json') -Json | ConvertFrom-Json
    if (-not $info.entry) { throw 'info: missing entry point' }
    if ($info.sections.Count -lt 1) { throw 'info: no sections' }
    if ($info.imports.Count -lt 1) { throw 'info: no imports' }

    Write-Host '[3/11] disasm'
    $disasm = Invoke-Recraft @('disasm', $project, '--addr', 'entry', '--count', '20')
    Assert-Contains $disasm 'start' 'entry name shown'
    Assert-Contains $disasm 'sub' 'disassembly mnemonic'

    Write-Host '[4/11] rename + CJK comment'
    Invoke-Recraft @('rename', $project, '--addr', 'entry', '--name', 'my_main') | Out-Null
    Invoke-Recraft @('comment', $project, '--addr', 'entry', '--text', $cjkComment) | Out-Null
    $disasm = Invoke-Recraft @('disasm', $project, '--addr', 'entry', '--count', '1')
    Assert-Contains $disasm 'my_main' 'renamed entry'
    Assert-Contains $disasm $cjkComment 'CJK comment'

    Write-Host '[5/11] decompile L0'
    $pseudo = Invoke-Recraft @('decompile', $project, '--addr', 'entry', '--max-insns', '64')
    Assert-Contains $pseudo 'return' 'pseudo-C control flow'

    Write-Host '[6/11] undo removes the comment'
    Invoke-Recraft @('undo', $project) | Out-Null
    $disasm = Invoke-Recraft @('disasm', $project, '--addr', 'entry', '--count', '1')
    if ($disasm.Contains($cjkComment)) { throw 'undo did not remove the comment' }
    Assert-Contains $disasm 'my_main' 'rename survives undoing the comment'

    Write-Host '[7/11] redo, and persistence after a fresh process'
    Invoke-Recraft @('redo', $project) | Out-Null
    $disasm = Invoke-Recraft @('disasm', $project, '--addr', 'entry', '--count', '1')
    Assert-Contains $disasm $cjkComment 'redo restores the comment'

    Write-Host '[8/11] JSON output'
    $lines = (Invoke-Recraft @('disasm', $project, '--count', '5', '--json') -Json | ConvertFrom-Json).lines
    if ($lines.Count -ne 5) { throw "expected 5 JSON lines, got $($lines.Count)" }

    Write-Host '[9/11] mark code/data'
    Invoke-Recraft @('mark', $project, '--addr', 'entry', '--len', '16', '--as', 'code') | Out-Null
    Invoke-Recraft @('undo', $project) | Out-Null

    Write-Host '[10/11] analyze functions'
    $summary = Invoke-Recraft @('analyze', $project)
    Assert-Contains $summary 'functions:' 'analysis summary'
    Assert-Contains $summary 'strings:' 'string extraction summary'
    $funcs = Invoke-Recraft @('funcs', $project)
    Assert-Contains $funcs '0x' 'function list after analysis'
    $strings = Invoke-Recraft @('strings', $project)
    Assert-Contains $strings 'strings' 'string table'
    $xrefs = Invoke-Recraft @('xrefs', $project, '--addr', 'entry')
    Assert-Contains $xrefs 'xrefs at' 'xref query'
    $imports = Invoke-Recraft @('imports', $project)
    Assert-Contains $imports 'imported APIs' 'import table with categories'
    $cfg = Invoke-Recraft @('cfg', $project, '--addr', 'entry')
    Assert-Contains $cfg 'blocks' 'control-flow graph'

    Write-Host '[+] byte search and call graph'
    $search = Invoke-Recraft @('search', $project, '--bytes', '4d5a', '--limit', '5')
    Assert-Contains $search 'hit(s) for 2 byte pattern' 'byte search reports hits'
    $searchJson = Invoke-Recraft @('search', $project, '--bytes', '4d5a', '--limit', '1', '--json')
    Assert-Contains $searchJson '"count"' 'byte search JSON'
    $graph = Invoke-Recraft @('callgraph', $project, '--limit', '10')
    Assert-Contains $graph 'call graph:' 'call graph summary'
    $graphJson = Invoke-Recraft @('callgraph', $project, '--limit', '5', '--json')
    Assert-Contains $graphJson '"edges"' 'call graph JSON'

    Write-Host '[+] user prototype (set, render, undo)'
    $firstFuncAddr = (($funcs -split "`r?`n" | Where-Object { $_ -match '^0x' } | Select-Object -First 1) -split '\s+')[0]
    Invoke-Recraft @('prototype', $project, '--addr', $firstFuncAddr, '--text', 'int entry_proto(void)') | Out-Null
    $pseudoProto = Invoke-Recraft @('decompile', $project, '--addr', $firstFuncAddr, '--l2')
    Assert-Contains $pseudoProto '// prototype: int entry_proto(void)' 'prototype rendered above signature'
    Assert-Contains $pseudoProto 'int32 entry_proto(void)' 'prototype types override the signature'
    Invoke-Recraft @('undo', $project) | Out-Null
    $pseudoProto = Invoke-Recraft @('decompile', $project, '--addr', $firstFuncAddr, '--l2')
    if ($pseudoProto.Contains('// prototype:')) { throw 'undo did not remove the prototype' }

    Write-Host '[+] user struct layout bound to an argument'
    Invoke-Recraft @('define-struct', $project, '--name', 'Demo', '--size', '4', '--field', '0:first:u32') | Out-Null
    $structsOut = Invoke-Recraft @('structs', $project)
    Assert-Contains $structsOut 'struct Demo' 'struct layout listed'
    $appliedType = Invoke-Recraft @('apply-type', $project, '--addr', $firstFuncAddr, '--slot', '0', '--type', 'Demo')
    Assert-Contains $appliedType 'applied Demo' 'struct type bound to argument'
    Invoke-Recraft @('undo', $project) | Out-Null
    Invoke-Recraft @('undo', $project) | Out-Null
    $structsOut = Invoke-Recraft @('structs', $project)
    if ($structsOut.Contains('struct Demo')) { throw 'undo did not remove the struct definition' }

    Write-Host '[+] batch script (dry run + apply)'
    $batchScript = Join-Path $work 'batch.json'
    $batchJson = @'
{"commands":[
 {"type":"rename","addr":"entry","name":"batch_entry"},
 {"type":"comment","addr":"batch_entry","text":"from batch"},
 {"type":"define_struct","name":"BatchDemo","size":4,"fields":[{"offset":0,"name":"tag","kind":"u32"}]},
 {"type":"apply_type","addr":"batch_entry","slot":0,"struct":"BatchDemo"}
]}
'@
    [System.IO.File]::WriteAllText($batchScript, $batchJson)
    $dry = Invoke-Recraft @('batch', $project, '--script', $batchScript, '--dry-run')
    Assert-Contains $dry 'validated 4 command(s)' 'batch dry run validates commands'
    $applied = Invoke-Recraft @('batch', $project, '--script', $batchScript)
    Assert-Contains $applied 'applied 4 command(s)' 'batch applies commands'
    $funcsAfterBatch = Invoke-Recraft @('funcs', $project)
    Assert-Contains $funcsAfterBatch 'batch_entry' 'batch rename took effect'

    Write-Host '[11/11] MCP stdio'
    $mcpLines = @(
        '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}',
        '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"list_functions","arguments":{"limit":3}}}',
        '{"jsonrpc":"2.0","id":3,"method":"resources/read","params":{"uri":"recraft://project/sample/functions"}}'
    )
    $mcp = ($mcpLines | & $Recraft serve-mcp $project | Out-String -Width 4096)
    Assert-Contains $mcp '"serverInfo"' 'MCP initialize'
    Assert-Contains $mcp 'recraft' 'MCP server name'
    Assert-Contains $mcp 'recraft://project/sample/functions' 'MCP functions resource'

    Write-Host '[+] MCP HTTP transport (bearer token)'
    # The HTTP server is killed at the end of the step; run it against a copy so a
    # hard kill can never leave the main project's WAL/SHM in a state that makes the
    # following AI step fail with a transient SQLite I/O error.
    $httpProject = Join-Path $work 'sample-http.rcraft'
    Copy-Item -LiteralPath $project -Destination $httpProject -Force
    foreach ($suffix in @('-wal', '-shm')) {
        $source = "$project$suffix"
        if (Test-Path -LiteralPath $source) {
            Copy-Item -LiteralPath $source -Destination "$httpProject$suffix" -Force
        }
    }
    $port = Get-Random -Minimum 20000 -Maximum 40000
    $mcpToken = 'e2e-token'
    $mcpOut = Join-Path $work 'mcp-http.out.log'
    $mcpErr = Join-Path $work 'mcp-http.err.log'
    $mcpProc = Start-Process -FilePath $Recraft `
        -ArgumentList @('serve-mcp', $httpProject, '--http', '--bind', "127.0.0.1:$port", '--token', $mcpToken) `
        -RedirectStandardOutput $mcpOut -RedirectStandardError $mcpErr -PassThru -WindowStyle Hidden
    try {
        $ready = $false
        for ($attempt = 0; $attempt -lt 50; $attempt++) {
            Start-Sleep -Milliseconds 100
            try {
                $null = Invoke-WebRequest -Uri "http://127.0.0.1:$port/health" -UseBasicParsing -TimeoutSec 1
                $ready = $true
                break
            } catch {
                if ($mcpProc.HasExited) { break }
            }
        }
        if (-not $ready) {
            $log = Get-Content $mcpErr -Raw -ErrorAction SilentlyContinue
            throw "MCP HTTP server did not start: $log"
        }
        $initBody = '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}'
        $httpResp = Invoke-WebRequest -Uri "http://127.0.0.1:$port/mcp" -Method Post -Body $initBody `
            -ContentType 'application/json' -Headers @{ Authorization = "Bearer $mcpToken" } -UseBasicParsing
        Assert-Contains $httpResp.Content '"serverInfo"' 'MCP HTTP initialize'
        $unauthorized = $false
        try {
            $null = Invoke-WebRequest -Uri "http://127.0.0.1:$port/mcp" -Method Post -Body $initBody -ContentType 'application/json' -UseBasicParsing
        } catch {
            $unauthorized = ([int]$_.Exception.Response.StatusCode -eq 401)
        }
        if (-not $unauthorized) { throw 'MCP HTTP accepted a request without a bearer token' }
    } finally {
        if (-not $mcpProc.HasExited) {
            Stop-Process -Id $mcpProc.Id -Force -ErrorAction SilentlyContinue
        }
    }

    Write-Host '[+] AI assist (offline explain/suggest/triage + propose-apply)'
    $aiTriage = Invoke-Recraft @('ai', 'triage', $project, '--json')
    Assert-Contains $aiTriage 'apis_by_category' 'AI triage categories'
    $proposalFile = Join-Path $work 'proposals.json'
    Invoke-Recraft @('ai', 'suggest', $project, '--limit', '3', '--out', $proposalFile) | Out-Null
    Assert-Contains (Get-Content -Raw -LiteralPath $proposalFile) 'confidence' 'AI proposals JSON'
    $aiApplied = Invoke-Recraft @('ai', 'apply', $project, $proposalFile, '--yes')
    Assert-Contains $aiApplied 'applied' 'AI proposal apply'
    $provider = Invoke-Recraft @('ai', 'provider')
    Assert-Contains $provider 'state=disabled' 'AI provider defaults offline'
    $aiPrompt = Invoke-Recraft @('ai', 'prompt', $project, '--addr', 'entry', '--json')
    Assert-Contains $aiPrompt 'estimated_tokens' 'AI prompt budget'
    $aiQuery = Invoke-Recraft @('ai', 'query', $project, 'find every function that references a URL')
    Assert-Contains $aiQuery 'intent: urls' 'AI natural-language query (URLs)'
    $aiQueryJson = Invoke-Recraft @('ai', 'query', $project, 'which functions call CreateFileW', '--json')
    Assert-Contains $aiQueryJson '"intent"' 'AI natural-language query JSON'

    if ($LargeOverlayMB -gt 0) {
        Write-Host "[+] large-sample smoke: $LargeOverlayMB MiB overlay"
        $big = Join-Path $work 'big.exe'
        Copy-Item $PEFile $big
        $buffer = [byte[]]::new(1MB)
        $remaining = [int64]$LargeOverlayMB * 1MB
        $stream = [System.IO.File]::Open($big, [System.IO.FileMode]::Append)
        try {
            while ($remaining -gt 0) {
                $count = [Math]::Min([int64]$buffer.Length, $remaining)
                $stream.Write($buffer, 0, [int]$count)
                $remaining -= $count
            }
        } finally { $stream.Dispose() }

        $bigProject = Join-Path $work 'big.rcraft'
        $timer = [System.Diagnostics.Stopwatch]::StartNew()
        Invoke-Recraft @('import', $big, '-o', $bigProject) | Out-Null
        $timer.Stop()
        $disasm = Invoke-Recraft @('disasm', $bigProject, '--addr', 'entry', '--count', '5')
        Assert-Contains $disasm 'sub' 'large overlay disassembly'
        $sizeMiB = (Get-Item $big).Length / 1MB
        Write-Host ('[+] imported {0:N0} MiB in {1:N2} s' -f $sizeMiB, $timer.Elapsed.TotalSeconds)
        Remove-Item $big, $bigProject, "$bigProject-wal", "$bigProject-shm" -Force -ErrorAction SilentlyContinue
    }

    Write-Host 'E2E PASS'
} finally {
    Remove-Item $work -Recurse -Force -ErrorAction SilentlyContinue
}
