# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 ReCraft contributors
#
# Verifies the MSI built by scripts/package.ps1 (DESIGN section 13).
#
# Always:
#   - structure: per-machine summary bit, .rcraft association, Start-menu shortcut,
#     Apps & Features properties, MajorUpgrade rule, VC++ runtime launch condition;
#   - payload: an administrative install (`msiexec /a`) extracts the same binaries,
#     documents and docs/ tree as dist/ReCraft-<version>/ (byte-identical hashes).
#
# From an elevated prompt (the MSI is per-machine, so msiexec needs to elevate):
#   - silent install into a temporary folder -> files / Start-menu shortcut / HKLM
#     Apps & Features entry -> installed CLI -> silent uninstall -> full cleanup;
#   - silent install with the default folder -> %ProgramFiles%\ReCraft -> uninstall.
#
#   powershell -File scripts/verify-msi.ps1                 # full run (needs elevation)
#   powershell -File scripts/verify-msi.ps1 -SkipInstall     # structure + payload only
#   powershell -File scripts/verify-msi.ps1 -Msi dist\recraft-v0.1.0-windows-x64.msi

[CmdletBinding()]
param(
    [string]$Msi = '',
    [switch]$SkipInstall
)

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
if (-not $Msi) {
    $Msi = Get-ChildItem -Path (Join-Path $repo 'dist') -Filter 'recraft-v*-windows-x64.msi' -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1 -ExpandProperty FullName
}
if (-not $Msi -or -not (Test-Path -LiteralPath $Msi)) {
    throw 'no MSI found; run scripts/package.ps1 (WiX 5 required) first'
}
$Msi = (Resolve-Path -LiteralPath $Msi).Path
$version = ($Msi | Split-Path -Leaf) -replace '^recraft-v', '' -replace '-windows-x64\.msi$', ''
$stage = Join-Path $repo "dist\ReCraft-$version"

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$elevated = (New-Object Security.Principal.WindowsPrincipal $identity).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $elevated -and -not $SkipInstall) {
    throw 'the MSI is per-machine: run this from an elevated prompt, or pass -SkipInstall for the structure/payload checks only'
}

$shortcut = Join-Path $env:ProgramData 'Microsoft\Windows\Start Menu\Programs\ReCraft.lnk'
$defaultTarget = Join-Path $env:ProgramFiles 'ReCraft'
$work = Join-Path $env:TEMP ('recraft-msi-check-' + [guid]::NewGuid().ToString('N').Substring(0, 8))
New-Item -ItemType Directory -Path $work | Out-Null

function Assert-Condition([bool]$Condition, [string]$What) {
    if (-not $Condition) { throw "MSI verification failed: $What" }
}

function Get-SummaryWordCount([string]$Path) {
    $installer = New-Object -ComObject WindowsInstaller.Installer
    $summary = $installer.GetType().InvokeMember('SummaryInformation', 'GetProperty', $null, $installer, @($Path, 0))
    return $summary.GetType().InvokeMember('Property', 'GetProperty', $null, $summary, @(15))
}

function Find-ArpKey([string]$DisplayName) {
    foreach ($root in @(
            'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall',
            'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall')) {
        if (-not (Test-Path -LiteralPath $root)) { continue }
        foreach ($key in Get-ChildItem -LiteralPath $root -ErrorAction SilentlyContinue) {
            $props = Get-ItemProperty -LiteralPath $key.PSPath -ErrorAction SilentlyContinue
            if ($props.DisplayName -like "$DisplayName*") { return $key }
        }
    }
    return $null
}

function Install-Msi([string]$InstallFolder, [string]$LogPath) {
    $arguments = @('/i', $Msi, '/qn', '/l*v', $LogPath)
    if ($InstallFolder) { $arguments += "INSTALLFOLDER=$InstallFolder" }
    $process = Start-Process -FilePath 'msiexec.exe' -Wait -PassThru -ArgumentList $arguments
    if ($process.ExitCode -ne 0) {
        $tail = (Get-Content -LiteralPath $LogPath -Tail 25 -ErrorAction SilentlyContinue) -join "`n"
        throw "msiexec install exit code $($process.ExitCode)`n$tail"
    }
}

function Uninstall-Msi([string]$ProductCode, [string]$LogPath) {
    $process = Start-Process -FilePath 'msiexec.exe' -Wait -PassThru -ArgumentList @(
        '/x', $ProductCode, '/qn', '/l*v', $LogPath
    )
    if ($process.ExitCode -ne 0) {
        $tail = (Get-Content -LiteralPath $LogPath -Tail 25 -ErrorAction SilentlyContinue) -join "`n"
        throw "msiexec uninstall exit code $($process.ExitCode)`n$tail"
    }
}

$installedCode = $null
try {
    # ---- 1. structure ----------------------------------------------------------------------------
    $wordCount = Get-SummaryWordCount $Msi
    Assert-Condition (($wordCount -band 8) -eq 0) 'the package is marked per-user; the MSI is authored per-machine'

    $wix = $null
    if (Get-Command dotnet -ErrorAction SilentlyContinue) {
        $probe = (& dotnet wix --version 2>$null) -join ''
        if ($LASTEXITCODE -eq 0 -and $probe -match '^5\.') { $wix = 'dotnet' }
    }
    if (-not $wix -and (Get-Command wix -ErrorAction SilentlyContinue)) {
        $probe = (& wix --version 2>$null) -join ''
        if ($LASTEXITCODE -eq 0 -and $probe -match '^5\.') { $wix = 'wix' }
    }
    if ($wix) {
        $decompiled = Join-Path $work 'decompiled.wxs'
        if ($wix -eq 'dotnet') {
            & dotnet wix msi decompile $Msi -o $decompiled 2>$null | Out-Null
        } else {
            & wix msi decompile $Msi -o $decompiled 2>$null | Out-Null
        }
        Assert-Condition ($LASTEXITCODE -eq 0) 'wix msi decompile failed'
        $tables = Get-Content -LiteralPath $decompiled -Raw
        $expected = @(
            @('<Shortcut Id="StartMenuShortcut"', 'Start-menu shortcut'),
            @('Key=".rcraft"', '.rcraft project association'),
            @('ReCraft.Project', '.rcraft ProgId'),
            @('<Property Id="ARPPRODUCTICON"', 'Apps & Features product icon'),
            @('<Directory Id="INSTALLFOLDER" Name="ReCraft"', 'install folder'),
            @('<Directory Id="DocsFolder" Name="docs"', 'docs folder'),
            @('<MajorUpgrade', 'major-upgrade rule'),
            @('vcruntime140.dll', 'VC++ runtime launch condition')
        )
        foreach ($check in $expected) {
            Assert-Condition ($tables -match [regex]::Escape($check[0])) "missing $($check[1])"
        }
        Write-Host "[msi] structure OK ($($expected.Count) table checks, per-machine)"
    } else {
        Write-Host '[msi] WiX 5 not available; skipped the decompiled-table checks'
    }

    # ---- 2. payload: administrative install extracts the staged tree ------------------------------
    Assert-Condition (Test-Path -LiteralPath $stage) "missing staged directory: $stage"
    $extract = Join-Path $work 'admin-install'
    New-Item -ItemType Directory -Force -Path $extract | Out-Null
    $adminLog = Join-Path $work 'admin-install.log'
    $admin = Start-Process -FilePath 'msiexec.exe' -Wait -PassThru -ArgumentList @(
        '/a', $Msi, '/qn', "TARGETDIR=$extract", '/l*v', $adminLog
    )
    if ($admin.ExitCode -ne 0) {
        $tail = (Get-Content -LiteralPath $adminLog -Tail 20 -ErrorAction SilentlyContinue) -join "`n"
        throw "administrative install exit code $($admin.ExitCode)`n$tail"
    }
    $root = Get-ChildItem -LiteralPath $extract -Recurse -Directory -Filter 'ReCraft' | Select-Object -First 1
    Assert-Condition ($null -ne $root) 'the administrative install did not contain a ReCraft folder'
    $root = $root.FullName

    $payload = @(
        'recraft.exe', 'recraft-cli.exe', 'LICENSE', 'NOTICE.md', 'README.md', 'CHANGELOG.md', 'VERSION.txt',
        'docs\DESIGN.md', 'docs\M3.md', 'docs\adr\ADR-001-headless-core-ui.md'
    )
    foreach ($entry in $payload) {
        $installed = Join-Path $root $entry
        Assert-Condition (Test-Path -LiteralPath $installed) "the MSI payload is missing $entry"
        $staged = Join-Path $stage $entry
        $expectedHash = (Get-FileHash -LiteralPath $staged -Algorithm SHA256).Hash
        $actualHash = (Get-FileHash -LiteralPath $installed -Algorithm SHA256).Hash
        Assert-Condition ($expectedHash -eq $actualHash) "the MSI payload differs from the staged $entry"
    }
    $stagedDocs = (Get-ChildItem -LiteralPath (Join-Path $stage 'docs') -Recurse -File -Filter '*.md').Count
    $installedDocs = (Get-ChildItem -LiteralPath (Join-Path $root 'docs') -Recurse -File -Filter '*.md').Count
    Assert-Condition ($stagedDocs -eq $installedDocs) "the MSI payload has $installedDocs docs, the staged tree has $stagedDocs"
    Write-Host "[msi] payload OK ($($payload.Count) files + $installedDocs docs, byte-identical to dist\ReCraft-$version)"

    if ($SkipInstall) {
        Write-Host '[msi] install/uninstall skipped (-SkipInstall); run from an elevated prompt for the full check'
        Write-Host '[msi] PASS (structure + payload)'
        return
    }

    # ---- 3. silent install into a temporary folder -----------------------------------------------
    # Native CLIs write progress to stderr; Windows PowerShell would turn that into a terminating
    # NativeCommandError under ErrorActionPreference=Stop.
    $ErrorActionPreference = 'Continue'
    $target = Join-Path $env:TEMP ('recraft-msi-install-' + [guid]::NewGuid().ToString('N').Substring(0, 8))
    Write-Host "[install] $Msi -> $target"
    Install-Msi $target (Join-Path $work 'install.log')

    foreach ($entry in $payload) {
        Assert-Condition (Test-Path -LiteralPath (Join-Path $target $entry)) "missing installed file: $entry"
    }
    Assert-Condition (Test-Path -LiteralPath $shortcut) 'Start Menu shortcut missing'

    $arp = Find-ArpKey 'ReCraft'
    Assert-Condition ($null -ne $arp) 'Apps & Features entry missing'
    Assert-Condition ($arp.PSPath -like 'Microsoft.PowerShell.Core\Registry::HKEY_LOCAL_MACHINE*') "unexpected install scope: $($arp.PSPath)"
    $arpProps = Get-ItemProperty -LiteralPath $arp.PSPath
    Assert-Condition ($arpProps.DisplayName -eq 'ReCraft') "unexpected DisplayName: $($arpProps.DisplayName)"
    Assert-Condition ($arpProps.DisplayVersion -eq $version) "unexpected DisplayVersion: $($arpProps.DisplayVersion)"
    $installedCode = Split-Path -Leaf $arp.PSPath
    Write-Host "[install] files, shortcut and machine-wide Apps & Features entry present ($($arpProps.DisplayName) $($arpProps.DisplayVersion), $installedCode)"

    $exe = Join-Path $target 'recraft-cli.exe'
    $info = & $exe info (Join-Path $env:SystemRoot 'System32\notepad.exe') 2>$null
    Assert-Condition (($info -join "`n") -match 'PE32') 'installed CLI info failed'
    $project = Join-Path $work 'installed.rcraft'
    & $exe import (Join-Path $env:SystemRoot 'System32\notepad.exe') -o $project 2>$null | Out-Null
    & $exe analyze $project 2>$null | Out-Null
    $metrics = (& $exe metrics $project --json 2>$null) | ConvertFrom-Json
    Assert-Condition ($metrics.functions -gt 100) "installed metrics returned $($metrics.functions) functions"
    Write-Host "[install] installed CLI OK ($($metrics.functions) functions, coverage $([math]::Round($metrics.coverage * 100, 1))%)"

    Write-Host '[uninstall] msiexec /x'
    Uninstall-Msi $installedCode (Join-Path $work 'uninstall.log')
    for ($attempt = 0; $attempt -lt 40; $attempt++) {
        if (-not (Test-Path -LiteralPath $target)) { break }
        Start-Sleep -Milliseconds 500
    }
    Assert-Condition (-not (Test-Path -LiteralPath $target)) 'install directory still exists after uninstall'
    Assert-Condition (-not (Test-Path -LiteralPath $shortcut)) 'Start Menu shortcut still exists after uninstall'
    Assert-Condition ($null -eq (Find-ArpKey 'ReCraft')) 'Apps & Features entry still exists after uninstall'
    Write-Host '[uninstall] directory, shortcut and Apps & Features entry removed'
    $installedCode = $null

    # ---- 4. default folder: %ProgramFiles%\ReCraft -----------------------------------------------
    if (Test-Path -LiteralPath $defaultTarget) {
        Write-Host "[install] skipping the default-folder check: $defaultTarget already exists"
    } else {
        Write-Host "[install] default folder -> $defaultTarget"
        Install-Msi '' (Join-Path $work 'install-default.log')
        $arp = Find-ArpKey 'ReCraft'
        Assert-Condition ($null -ne $arp) 'Apps & Features entry missing after the default install'
        $installedCode = Split-Path -Leaf $arp.PSPath
        Assert-Condition (Test-Path -LiteralPath (Join-Path $defaultTarget 'recraft.exe')) "the default install did not land in $defaultTarget"
        Assert-Condition (Test-Path -LiteralPath $shortcut) 'Start Menu shortcut missing after the default install'
        Write-Host "[install] default folder OK ($defaultTarget)"

        Uninstall-Msi $installedCode (Join-Path $work 'uninstall-default.log')
        for ($attempt = 0; $attempt -lt 40; $attempt++) {
            if (-not (Test-Path -LiteralPath $defaultTarget)) { break }
            Start-Sleep -Milliseconds 500
        }
        Assert-Condition (-not (Test-Path -LiteralPath $defaultTarget)) 'default install directory still exists after uninstall'
        Assert-Condition ($null -eq (Find-ArpKey 'ReCraft')) 'Apps & Features entry still exists after the default uninstall'
        Write-Host '[uninstall] default folder removed'
        $installedCode = $null
    }

    Write-Host '[msi] PASS'
} finally {
    if ($installedCode) {
        Start-Process -FilePath 'msiexec.exe' -Wait -ArgumentList @('/x', $installedCode, '/qn') -ErrorAction SilentlyContinue
    }
    Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
}
