# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 ReCraft contributors
#
# 1 GiB performance gate (DESIGN §3.3 / §14 M3).
#
#   pwsh -File scripts/bench-large.ps1                     # 1 GiB overlay, default thresholds
#   pwsh -File scripts/bench-large.ps1 -OverlayMB 256      # faster smoke
#   pwsh -File scripts/bench-large.ps1 -Json               # machine-readable result
#
# Measures:
#   - import wall time (target < 60 s)
#   - analyze wall time (target < 5 s: interactive budget for a mapped-code sample)
#   - peak working set of the analyze process (target <= 512 MiB)
#
# Exits non-zero when a threshold is exceeded.

[CmdletBinding()]
param(
    [string]$PEFile = (Join-Path $env:SystemRoot 'System32\notepad.exe'),
    [int]$OverlayMB = 1024,
    [double]$ImportMaxSeconds = 60.0,
    [double]$AnalyzeMaxSeconds = 5.0,
    [int]$MemoryMaxMiB = 512,
    [string]$Recraft = '',
    [switch]$Json
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

$work = Join-Path ([System.IO.Path]::GetTempPath()) ('recraft-bench-' + [guid]::NewGuid().ToString('N').Substring(0, 8))
New-Item -ItemType Directory -Path $work | Out-Null

# Runs recraft and reports wall time plus the child process peak working set.
# Uses System.Diagnostics.Process directly: Windows PowerShell's Start-Process loses
# ExitCode without -Wait and PeakWorkingSet64 with -Wait.
function Quote-Arg([string]$value) {
    if ($value -match '[\s"]') { return '"' + ($value -replace '"', '\"') + '"' }
    return $value
}

function Invoke-Measured {
    param([string[]]$Arguments)
    $psi = [System.Diagnostics.ProcessStartInfo]::new()
    $psi.FileName = $Recraft
    $psi.Arguments = ($Arguments | ForEach-Object { Quote-Arg $_ }) -join ' '
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $process = [System.Diagnostics.Process]::Start($psi)
    $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
    # Drain both streams continuously so a chatty import cannot block on a full pipe.
    $stdoutTask = $process.StandardOutput.ReadToEndAsync()
    $stderrTask = $process.StandardError.ReadToEndAsync()
    $peak = 0L
    while (-not $process.HasExited) {
        try {
            $workingSet = $process.WorkingSet64
            if ($workingSet -gt $peak) { $peak = $workingSet }
        } catch { }
        Start-Sleep -Milliseconds 20
    }
    $process.WaitForExit()
    $stopwatch.Stop()
    $stdout = $stdoutTask.GetAwaiter().GetResult()
    $stderr = $stderrTask.GetAwaiter().GetResult()
    if ($process.ExitCode -ne 0) {
        throw ("recraft {0} failed ({1})`n{2}`n{3}" -f ($Arguments -join ' '), $process.ExitCode, $stdout, $stderr)
    }
    $process.Dispose()
    [pscustomobject]@{
        Seconds = $stopwatch.Elapsed.TotalSeconds
        PeakMiB = [math]::Round($peak / 1MB, 1)
        Output  = $stdout
    }
}

try {
    $sample = Join-Path $work 'large.exe'
    Copy-Item -LiteralPath $PEFile -Destination $sample
    $remaining = [int64]$OverlayMB * 1MB
    $buffer = [byte[]]::new(1MB)
    $stream = [System.IO.File]::Open($sample, [System.IO.FileMode]::Append)
    try {
        while ($remaining -gt 0) {
            $count = [Math]::Min([int64]$buffer.Length, $remaining)
            $stream.Write($buffer, 0, [int]$count)
            $remaining -= $count
        }
    } finally { $stream.Dispose() }
    $sizeMiB = [math]::Round((Get-Item -LiteralPath $sample).Length / 1MB, 1)

    $project = Join-Path $work 'large.rcraft'
    $import = Invoke-Measured @('import', $sample, '-o', $project)
    $analyze = Invoke-Measured @('analyze', $project)

    $failures = @()
    if ($import.Seconds -gt $ImportMaxSeconds) {
        $failures += ('import {0:N2}s > {1:N1}s' -f $import.Seconds, $ImportMaxSeconds)
    }
    if ($analyze.Seconds -gt $AnalyzeMaxSeconds) {
        $failures += ('analyze {0:N2}s > {1:N1}s' -f $analyze.Seconds, $AnalyzeMaxSeconds)
    }
    if ($analyze.PeakMiB -gt $MemoryMaxMiB) {
        $failures += ('analyze peak {0:N1} MiB > {1} MiB' -f $analyze.PeakMiB, $MemoryMaxMiB)
    }

    $result = [ordered]@{
        sample_mib        = $sizeMiB
        import_seconds    = [math]::Round($import.Seconds, 3)
        analyze_seconds   = [math]::Round($analyze.Seconds, 3)
        analyze_peak_mib  = $analyze.PeakMiB
        import_max_s      = $ImportMaxSeconds
        analyze_max_s     = $AnalyzeMaxSeconds
        memory_max_mib    = $MemoryMaxMiB
        failures          = $failures
        passed            = ($failures.Count -eq 0)
    }
    if ($Json) {
        $result | ConvertTo-Json -Compress
    } else {
        Write-Host ('[bench] sample: {0:N1} MiB' -f $sizeMiB)
        Write-Host ('[bench] import: {0:N2} s (limit {1:N1} s)' -f $import.Seconds, $ImportMaxSeconds)
        Write-Host ('[bench] analyze: {0:N2} s (limit {1:N1} s), peak {2:N1} MiB (limit {3} MiB)' -f `
            $analyze.Seconds, $AnalyzeMaxSeconds, $analyze.PeakMiB, $MemoryMaxMiB)
        if ($failures.Count -eq 0) {
            Write-Host '[bench] PASS'
        } else {
            foreach ($failure in $failures) { Write-Host "[bench] FAIL: $failure" }
        }
    }
    if ($failures.Count -gt 0) {
        throw ('performance gate failed: ' + ($failures -join '; '))
    }
} finally {
    Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
}
