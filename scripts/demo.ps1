# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 ReCraft contributors
#
# Ten-minute demo walkthrough (DESIGN M4): import -> analyze -> pseudocode ->
# rename / prototype / struct type -> AI propose/apply -> MCP -> metrics.
#
#   pwsh -File scripts/demo.ps1                 # corpus.exe if built, else notepad
#   pwsh -File scripts/demo.ps1 -PEFile C:\Windows\System32\notepad.exe
#   pwsh -File scripts/demo.ps1 -Keep           # keep the demo project for the GUI
#
# The script is read-only w.r.t. the repository: everything happens in a temp project.

[CmdletBinding()]
param(
    [string]$PEFile = '',
    [switch]$Keep,
    [string]$Recraft = ''
)

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
if (-not $Recraft) {
    $Recraft = Join-Path $repo 'target\debug\recraft.exe'
    if (-not (Test-Path -LiteralPath $Recraft)) {
        Push-Location $repo
        try { & cargo build -p recraft-cli --quiet } finally { Pop-Location }
    }
}
if (-not (Test-Path -LiteralPath $Recraft)) { throw "recraft binary not found: $Recraft" }

if (-not $PEFile) {
    $corpus = Join-Path $repo 'corpus\build\x64\corpus.exe'
    $PEFile = if (Test-Path -LiteralPath $corpus) { $corpus } else { Join-Path $env:SystemRoot 'System32\notepad.exe' }
}
if (-not (Test-Path -LiteralPath $PEFile)) { throw "PE file not found: $PEFile" }

$work = Join-Path ([System.IO.Path]::GetTempPath()) ('recraft-demo-' + [guid]::NewGuid().ToString('N').Substring(0, 8))
New-Item -ItemType Directory -Path $work | Out-Null
$project = Join-Path $work 'demo.rcraft'
$proposals = Join-Path $work 'proposals.json'

function Step([string]$title) {
    Write-Host ''
    Write-Host ('=' * 4 + ' ' + $title + ' ' + '=' * 4) -ForegroundColor Cyan
}

function Run-Recraft([string[]]$Arguments) {
    Write-Host ('$ recraft ' + ($Arguments -join ' ')) -ForegroundColor DarkGray
    # Progress lines from import/analyze go to stderr; capture them without letting the
    # script's $ErrorActionPreference='Stop' turn a stderr line into a terminating error.
    $previous = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        $output = & $Recraft @Arguments 2>&1
        $code = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $previous
    }
    if ($code -ne 0) { throw ("recraft {0} failed" -f ($Arguments -join ' ')) }
    return $output
}

function Assert-Contains([string]$Text, [string]$Needle, [string]$What) {
    if (-not $Text.Contains($Needle)) {
        throw "demo assertion failed: $What (missing '$Needle')"
    }
}

try {
    Step '1. PE headers'
    (Run-Recraft @('info', $PEFile) | Select-Object -First 10) | ForEach-Object { Write-Host $_ }

    Step '2. import + analysis'
    Run-Recraft @('import', $PEFile, '-o', $project) | Select-Object -Last 1 | ForEach-Object { Write-Host $_ }
    $analysis = Run-Recraft @('analyze', $project)
    $analysis | Select-Object -Last 1 | ForEach-Object { Write-Host $_ }

    Step '3. function discovery'
    $funcs = Run-Recraft @('funcs', $project)
    $funcs | Select-Object -First 8 | ForEach-Object { Write-Host $_ }
    $firstFunc = (($funcs -split "`r?`n" | Where-Object { $_ -match '^0x' } | Select-Object -First 1) -split '\s+')[0]

    Step '4. structured pseudocode (L2)'
    (Run-Recraft @('decompile', $project, '--addr', 'entry', '--l2') | Select-Object -First 14) | ForEach-Object { Write-Host $_ }

    Step '5. rename + user prototype (journaled, undoable)'
    Run-Recraft @('rename', $project, '--addr', $firstFunc, '--name', 'demo_entry') | Out-Null
    $protoText = 'int __stdcall demo_entry(const char* text, int count)'
    Run-Recraft @('prototype', $project, '--addr', $firstFunc, '--text', $protoText) | ForEach-Object { Write-Host $_ }
    $pseudo = Run-Recraft @('decompile', $project, '--addr', $firstFunc, '--l2')
    Assert-Contains ($pseudo -join "`n") '// prototype:' 'prototype comment'
    Assert-Contains ($pseudo -join "`n") 'demo_entry' 'renamed function'
    $pseudo | Select-Object -First 10 | ForEach-Object { Write-Host $_ }

    Step '6. struct layout + apply_type (named fields)'
    if (($funcs -join "`n").Contains('field_probe')) {
        Run-Recraft @('define-struct', $project, '--name', 'Probe', '--size', '24', '--field', '0:tag:u32', '--field', '16:value:i32') | ForEach-Object { Write-Host $_ }
        Run-Recraft @('apply-type', $project, '--addr', 'field_probe', '--slot', '0', '--type', 'Probe') | ForEach-Object { Write-Host $_ }
        $typed = Run-Recraft @('decompile', $project, '--addr', 'field_probe', '--l2')
        Assert-Contains ($typed -join "`n") 'arg1->value' 'struct field name in pseudocode'
        $typed | Select-Object -First 10 | ForEach-Object { Write-Host $_ }
    } else {
        Write-Host '[skip] sample has no field_probe; struct demo needs the corpus fixture'
    }

    Step '7. AI assist (requires a configured provider)'
    $providerStatus = (Run-Recraft @('ai', 'provider') -join "`n")
    if ($providerStatus -match 'state=disabled') {
        Write-Host '[skip] AI provider is disabled; configure it in the AI panel and re-run.'
    } else {
        # The corpus fixture has no imports/strings, so run the AI demo on a real sample.
        $realProject = $project
        $notepad = Join-Path $env:SystemRoot 'System32\notepad.exe'
        if (Test-Path -LiteralPath $notepad) {
            $realProject = Join-Path $work 'real.rcraft'
            Run-Recraft @('import', $notepad, '-o', $realProject) | Out-Null
            Run-Recraft @('analyze', $realProject) | Out-Null
            Write-Host "AI demo project: $notepad"
        }
        (Run-Recraft @('ai', 'triage', $realProject) | Select-Object -First 14) | ForEach-Object { Write-Host $_ }
        Run-Recraft @('ai', 'suggest', $realProject, '--limit', '3', '--out', $proposals) | ForEach-Object { Write-Host $_ }
        $lines = (Get-Content -Raw -LiteralPath $proposals) -split "`r?`n"
        ($lines | Select-Object -First 22) | ForEach-Object { Write-Host $_ }
        (Run-Recraft @('ai', 'apply', $realProject, $proposals, '--yes') | Select-Object -First 5) | ForEach-Object { Write-Host $_ }
        (Run-Recraft @('undo', $realProject)) | ForEach-Object { Write-Host ("undo: " + $_) }
    }

    Step '8. MCP server (newline-delimited JSON-RPC over stdio)'
    $mcpLines = @(
        '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}',
        '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"list_functions","arguments":{"limit":2}}}'
    )
    $mcp = ($mcpLines | & $Recraft serve-mcp $project | Out-String -Width 4096)
    Assert-Contains $mcp '"serverInfo"' 'MCP initialize'
    ($mcp -split "`r?`n" | Select-Object -First 3) | ForEach-Object { Write-Host $_ }

    Step '9. quality metrics'
    (Run-Recraft @('metrics', $project) | ForEach-Object { Write-Host $_ })

    Step 'done'
    if ($Keep) {
        Write-Host "project kept: $project"
        Write-Host "open the GUI: $((Split-Path -Parent $Recraft))\recraft-ui.exe $project"
    } else {
        Write-Host 'DEMO PASS (temporary project removed; pass -Keep to open it in the GUI)'
    }
} finally {
    if (-not $Keep) {
        Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
    }
}

