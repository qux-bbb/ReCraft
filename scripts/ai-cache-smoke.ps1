# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 ReCraft contributors
#
# Cache/audit smoke test for `recraft ai explain --llm` (no external network).
#
# Starts a throw-away TCP HTTP server that answers like Ollama /api/chat, points the
# Ollama provider at it with an isolated LOCALAPPDATA, and runs explain twice:
#   - first call  -> cached=no  (server called)
#   - second call -> cached=yes (served from ai-cache.json)
# It then checks the JSONL audit log has two records.
#
#   pwsh -File scripts/ai-cache-smoke.ps1

[CmdletBinding()]
param(
    [string]$PEFile = '',
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

$work = Join-Path ([System.IO.Path]::GetTempPath()) ('recraft-ai-cache-' + [guid]::NewGuid().ToString('N').Substring(0, 8))
New-Item -ItemType Directory -Path $work | Out-Null
$project = Join-Path $work 'cache.rcraft'
$localAppData = Join-Path $work 'localappdata'
New-Item -ItemType Directory -Path $localAppData | Out-Null
$port = 18234
$serverJob = $null

function Run-Recraft([string[]]$Arguments) {
    $output = & $Recraft @Arguments
    if ($LASTEXITCODE -ne 0) { throw ("recraft {0} failed" -f ($Arguments -join ' ')) }
    return $output
}

try {
    Run-Recraft @('import', $PEFile, '-o', $project) | Out-Null
    Run-Recraft @('analyze', $project) | Out-Null

    $serverJob = Start-Job -ScriptBlock {
        param($Port)
        $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, $Port)
        $listener.Start()
        try {
            for ($i = 0; $i -lt 2; $i++) {
                $client = $listener.AcceptTcpClient()
                $stream = $client.GetStream()
                $buffer = [byte[]]::new(65536)
                $read = $stream.Read($buffer, 0, $buffer.Length)
                $body = '{"message":{"role":"assistant","content":"mock explanation"}}'
                $response = "HTTP/1.1 200 OK`r`nContent-Type: application/json`r`nContent-Length: $($body.Length)`r`nConnection: close`r`n`r`n$body"
                $bytes = [System.Text.Encoding]::UTF8.GetBytes($response)
                $stream.Write($bytes, 0, $bytes.Length)
                $stream.Flush()
                $client.Close()
            }
        } finally {
            $listener.Stop()
        }
    } -ArgumentList $port
    Start-Sleep -Milliseconds 300
    if ($serverJob.State -eq 'Failed') { throw "mock server failed to start: $($serverJob.ChildJobs[0].JobStateInfo.Reason)" }

    $env:LOCALAPPDATA = $localAppData
    $env:RECRAFT_AI_PROVIDER = 'ollama'
    $env:RECRAFT_AI_ENDPOINT = "http://127.0.0.1:$port"
    $env:RECRAFT_AI_MODEL = 'mock-model'
    $env:RECRAFT_AI_API_KEY = 'local'

    $first = Run-Recraft @('ai', 'explain', $project, '--addr', 'entry', '--llm')
    $second = Run-Recraft @('ai', 'explain', $project, '--addr', 'entry', '--llm')
    $firstText = $first -join "`n"
    $secondText = $second -join "`n"
    if (-not $firstText.Contains('cached=no')) { throw "first call should be a cache miss: $firstText" }
    if (-not $secondText.Contains('cached=yes')) { throw "second call should hit the cache: $secondText" }
    if (-not $secondText.Contains('mock explanation')) { throw "cached response missing: $secondText" }

    $cacheFile = Join-Path $localAppData 'ReCraft\ai-cache.json'
    $auditFile = Join-Path $localAppData 'ReCraft\ai-audit.jsonl'
    if (-not (Test-Path -LiteralPath $cacheFile)) { throw "cache file missing: $cacheFile" }
    if (-not (Test-Path -LiteralPath $auditFile)) { throw "audit file missing: $auditFile" }
    $auditLines = @(Get-Content -LiteralPath $auditFile)
    if ($auditLines.Count -ne 2) { throw "expected 2 audit lines, got $($auditLines.Count)" }
    $audit = $auditLines[1] | ConvertFrom-Json
    if ($audit.cached -ne $true) { throw 'second audit record should mark cached=true' }
    if ($audit.model -ne 'mock-model') { throw "audit model mismatch: $($audit.model)" }

    Write-Host '[ai-cache] first=cached=no second cached=yes; audit lines=2; PASS'
} finally {
    if ($serverJob) {
        Stop-Job -Job $serverJob -ErrorAction SilentlyContinue
        Remove-Job -Job $serverJob -Force -ErrorAction SilentlyContinue
    }
    Remove-Item Env:RECRAFT_AI_PROVIDER, Env:RECRAFT_AI_ENDPOINT, Env:RECRAFT_AI_MODEL, Env:RECRAFT_AI_API_KEY -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
}
