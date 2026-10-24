# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 ReCraft contributors
#
# End-to-end installer verification (DESIGN §13): silent install into a temporary
# per-user directory → check files/shortcut/uninstall registry entry → run the installed
# CLI → launch the GUI and check its window → silent uninstall → verify full cleanup.
#
# The installer is per-user (RequestExecutionLevel user) and needs no elevation.
#
#   pwsh -File scripts/verify-installer.ps1
#   pwsh -File scripts/verify-installer.ps1 -Setup dist\recraft-0.1.0-setup.exe

[CmdletBinding()]
param(
    [string]$Setup = ''
)

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
if (-not $Setup) {
    $Setup = Get-ChildItem -Path (Join-Path $repo 'dist') -Filter 'recraft-*-setup.exe' -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1 -ExpandProperty FullName
}
if (-not $Setup -or -not (Test-Path -LiteralPath $Setup)) {
    throw 'no installer found; run scripts/package.ps1 (NSIS required) first'
}
$Setup = (Resolve-Path -LiteralPath $Setup).Path

$regKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\ReCraft'
$shortcut = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\ReCraft.lnk'
$target = Join-Path $env:TEMP ('recraft-install-' + [guid]::NewGuid().ToString('N').Substring(0, 8))
$work = Join-Path $env:TEMP ('recraft-install-check-' + [guid]::NewGuid().ToString('N').Substring(0, 8))
New-Item -ItemType Directory -Path $work | Out-Null

function Assert-Condition([bool]$Condition, [string]$What) {
    if (-not $Condition) { throw "installer verification failed: $What" }
}

if (Test-Path -LiteralPath $regKey) {
    throw 'an existing ReCraft install is registered; uninstall it before running this check'
}

$uiProcess = $null
$installed = $false
try {
    # Native CLIs write progress to stderr; Windows PowerShell would turn that into a
    # terminating NativeCommandError under ErrorActionPreference=Stop.
    $ErrorActionPreference = 'Continue'
    Write-Host "[install] $Setup -> $target"
    $install = Start-Process -FilePath $Setup -ArgumentList @('/S', "/D=$target") -PassThru -Wait
    Assert-Condition ($install.ExitCode -eq 0) "installer exit code $($install.ExitCode)"
    $installed = $true

    foreach ($entry in @('recraft.exe', 'recraft-ui.exe', 'uninstall.exe', 'LICENSE', 'VERSION.txt', 'docs\DESIGN.md')) {
        Assert-Condition (Test-Path -LiteralPath (Join-Path $target $entry)) "missing installed file: $entry"
    }
    Assert-Condition (Test-Path -LiteralPath $shortcut) 'Start Menu shortcut missing'
    $reg = Get-ItemProperty -LiteralPath $regKey -ErrorAction SilentlyContinue
    Assert-Condition ($null -ne $reg) 'uninstall registry entry missing'
    Assert-Condition ($reg.DisplayName -like 'ReCraft*') "unexpected DisplayName: $($reg.DisplayName)"
    Assert-Condition ($reg.InstallLocation -eq $target) "InstallLocation mismatch: $($reg.InstallLocation)"
    Write-Host '[install] files, shortcut and registry entry present'

    $info = & (Join-Path $target 'recraft.exe') info $env:SystemRoot\System32\notepad.exe 2>$null
    Assert-Condition (($info -join "`n") -match 'PE32') 'installed CLI info failed'
    $project = Join-Path $work 'installed.rcraft'
    & (Join-Path $target 'recraft.exe') import $env:SystemRoot\System32\notepad.exe -o $project 2>$null | Out-Null
    & (Join-Path $target 'recraft.exe') analyze $project 2>$null | Out-Null
    $metrics = (& (Join-Path $target 'recraft.exe') metrics $project --json 2>$null) | ConvertFrom-Json
    Assert-Condition ($metrics.functions -gt 100) "installed metrics returned $($metrics.functions) functions"
    Write-Host "[install] installed CLI OK ($($metrics.functions) functions, coverage $([math]::Round($metrics.coverage * 100, 1))%)"

    $uiProcess = Start-Process -FilePath (Join-Path $target 'recraft-ui.exe') -PassThru
    Start-Sleep -Seconds 4
    $uiProcess.Refresh()
    Assert-Condition (-not $uiProcess.HasExited) 'GUI exited during startup'
    Assert-Condition ($uiProcess.MainWindowTitle -like '*ReCraft*') "unexpected window title: '$($uiProcess.MainWindowTitle)'"
    Write-Host "[install] GUI launched (window: $($uiProcess.MainWindowTitle))"
    Stop-Process -Id $uiProcess.Id -Force
    $uiProcess = $null

    Write-Host '[uninstall] running silent uninstaller'
    $uninstall = Start-Process -FilePath (Join-Path $target 'uninstall.exe') -ArgumentList '/S' -PassThru -Wait
    Assert-Condition ($uninstall.ExitCode -eq 0) "uninstaller exit code $($uninstall.ExitCode)"
    for ($attempt = 0; $attempt -lt 40; $attempt++) {
        if (-not (Test-Path -LiteralPath $target)) { break }
        Start-Sleep -Milliseconds 500
    }
    Assert-Condition (-not (Test-Path -LiteralPath $target)) 'install directory still exists after uninstall'
    Assert-Condition (-not (Test-Path -LiteralPath $shortcut)) 'Start Menu shortcut still exists after uninstall'
    Assert-Condition (-not (Test-Path -LiteralPath $regKey)) 'uninstall registry entry still exists after uninstall'
    Write-Host '[uninstall] directory, shortcut and registry entry removed'
    Write-Host '[installer] PASS'
} finally {
    if ($uiProcess -and -not $uiProcess.HasExited) {
        Stop-Process -Id $uiProcess.Id -Force -ErrorAction SilentlyContinue
    }
    if ($installed -and (Test-Path -LiteralPath $target)) {
        $uninstaller = Join-Path $target 'uninstall.exe'
        if (Test-Path -LiteralPath $uninstaller) {
            Start-Process -FilePath $uninstaller -ArgumentList '/S' -Wait -ErrorAction SilentlyContinue
            for ($attempt = 0; $attempt -lt 20; $attempt++) {
                if (-not (Test-Path -LiteralPath $target)) { break }
                Start-Sleep -Milliseconds 250
            }
        }
        Remove-Item -LiteralPath $target -Recurse -Force -ErrorAction SilentlyContinue
    }
    Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
}
