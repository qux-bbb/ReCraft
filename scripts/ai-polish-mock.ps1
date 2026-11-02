# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 ReCraft contributors
#
# End-to-end smoke test of `recraft ai polish` against a local OpenAI-compatible stub.
# The stub returns canned text; this verifies the prompt -> HTTP -> cache -> output path,
# not model quality. For a real model set RECRAFT_AI_* to Ollama/OpenAI and run
# `recraft ai polish <project> --addr 0x140001010` directly.

param(
    [string]$Binary = 'D:\files\projects\ctf-simple-crackme\challenge\crackme_win_x64.exe',
    [int]$Port = 18765
)

$ErrorActionPreference = 'Continue'
$root = Split-Path -Parent $PSScriptRoot
$cli = Join-Path $root 'target\debug\recraft-cli.exe'
$serverScript = Join-Path $PSScriptRoot 'ai_mock_server.py'
$project = Join-Path $env:TEMP ("recraft-ai-polish-{0}.rcraft" -f [guid]::NewGuid().ToString('N').Substring(0, 8))

if (-not (Test-Path -LiteralPath $Binary)) { throw "binary not found: $Binary" }
if (Test-Path -LiteralPath $project) { throw "project already exists: $project" }
if (-not (Test-Path -LiteralPath $cli)) {
    Push-Location $root
    try { cargo build -q -p recraft-cli } finally { Pop-Location }
}
$python = (Get-Command python -ErrorAction SilentlyContinue).Source
if (-not $python) { $python = (Get-Command py -ErrorAction SilentlyContinue).Source }
if (-not $python) { throw 'python is required for the mock server' }

& $cli import --out $project $Binary 2>$null
if ($LASTEXITCODE -ne 0) { throw 'import failed' }
& $cli analyze $project | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'analysis failed' }

$prototypes = [ordered]@{
    '0x140001010' = 'int main(void)'
    '0x140001170' = 'int puts(const char* text)'
    '0x140004580' = 'void* __acrt_iob_func(uint32 index)'
    '0x1400049f8' = 'int fflush(void* stream)'
    '0x140004bd4' = 'char* fgets(char* buf, int size, void* stream)'
    '0x140004cc4' = 'int printf(const char* text)'
    '0x140006fd0' = 'char* strchr(const char* text, int ch)'
    '0x140016b60' = 'int strcmp(const char* left, const char* right)'
    '0x140016be0' = 'uint64 strlen(const char* text)'
}
foreach ($entry in $prototypes.GetEnumerator()) {
    & $cli prototype --addr $entry.Key --text $entry.Value $project | Out-Null
}

$server = Start-Process -FilePath $python -ArgumentList @($serverScript, $Port) -PassThru -WindowStyle Hidden
try {
    Start-Sleep -Milliseconds 800
    $env:RECRAFT_AI_PROVIDER = 'openai'
    $env:RECRAFT_AI_ENDPOINT = "http://127.0.0.1:$Port"
    $env:RECRAFT_AI_MODEL = 'mock-polish'
    $env:RECRAFT_AI_API_KEY = 'mock-key'

    Write-Host '== ai provider ==' -ForegroundColor Cyan
    & $cli ai provider
    Write-Host '== ai polish (mock OpenAI-compatible endpoint) ==' -ForegroundColor Cyan
    & $cli ai polish --addr 0x140001010 --no-cache $project
    if ($LASTEXITCODE -ne 0) { throw 'ai polish failed' }
}
finally {
    Remove-Item Env:RECRAFT_AI_PROVIDER, Env:RECRAFT_AI_ENDPOINT, Env:RECRAFT_AI_MODEL, Env:RECRAFT_AI_API_KEY -ErrorAction SilentlyContinue
    if ($server -and -not $server.HasExited) { Stop-Process -Id $server.Id -Force -ErrorAction SilentlyContinue }
    Remove-Item -LiteralPath $project -Recurse -Force -ErrorAction SilentlyContinue
}
