# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 ReCraft contributors
#
# Verifies the MSI built by scripts/package.ps1 (DESIGN section 13).
#
# Always:
#   - structure: per-machine summary bit, .rcraft association, Start-menu shortcuts,
#     CLI App Paths + Capabilities/RegisteredApplications, PATH/desktop-shortcut options,
#     Apps & Features properties, MajorUpgrade rule, VC++ runtime launch condition,
#     zh-CN wizard localization (2052/936);
#   - payload: an administrative install (`msiexec /a`) extracts the same binaries,
#     documents and docs/ tree as dist/ReCraft-<version>/ (byte-identical hashes).
#
# From an elevated prompt (the MSI is per-machine, so msiexec needs to elevate):
#   - silent install into a temporary folder with ADDTOPATH=0 DESKTOPSHORTCUT=1 ->
#     files / shortcuts / HKLM entries / desktop shortcut / installed CLI -> silent
#     uninstall -> full cleanup (directory, shortcuts, registry);
#   - silent install with the default folder and default options (PATH on, desktop off) ->
#     %ProgramFiles%\ReCraft + PATH entry, no desktop shortcut -> uninstall -> PATH removed.
#
#   powershell -File scripts/verify-msi.ps1                 # full run (needs elevation)
#   powershell -File scripts/verify-msi.ps1 -SkipInstall     # structure + payload only
#   powershell -File scripts/verify-msi.ps1 -Msi dist\recraft-v0.1.0-windows-x64.msi
#   powershell -File scripts/verify-msi.ps1 -Culture en-US   # English build

[CmdletBinding()]
param(
    [string]$Msi = '',
    [switch]$SkipInstall,
    [ValidateSet('zh-CN', 'en-US')]
    [string]$Culture = 'zh-CN'
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
if ($Culture -eq 'zh-CN') {
    $msiLanguage = 2052
    $msiCodepage = 936
} else {
    $msiLanguage = 1033
    $msiCodepage = 1252
}

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$elevated = (New-Object Security.Principal.WindowsPrincipal $identity).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $elevated -and -not $SkipInstall) {
    throw 'the MSI is per-machine: run this from an elevated prompt, or pass -SkipInstall for the structure/payload checks only'
}

$shortcut = Join-Path $env:ProgramData 'Microsoft\Windows\Start Menu\Programs\ReCraft.lnk'
$startMenu = Join-Path $env:ProgramData 'Microsoft\Windows\Start Menu\Programs'
$desktopShortcut = Join-Path ([Environment]::GetFolderPath('CommonDesktopDirectory')) 'ReCraft.lnk'
$machineEnvironmentKey = 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment'
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

# The machine PATH as stored, without expanding %VARS%, plus the registry value kind.
# Restoring it through [Environment]::SetEnvironmentVariable would rewrite REG_EXPAND_SZ
# as REG_SZ, so the finally block uses SetValue with the original kind instead.
function Get-MachinePathRaw {
    $key = [Microsoft.Win32.Registry]::LocalMachine.OpenSubKey($machineEnvironmentKey, $false)
    if ($null -eq $key) { return $null }
    try {
        return [pscustomobject]@{
            Value = [string]$key.GetValue('Path', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
            Kind  = $key.GetValueKind('Path')
        }
    } finally {
        $key.Close()
    }
}

function Install-Msi([string]$InstallFolder, [string]$LogPath, [string[]]$Properties = @()) {
    $arguments = @('/i', $Msi, '/qn', '/l*v', $LogPath)
    if ($InstallFolder) { $arguments += "INSTALLFOLDER=$InstallFolder" }
    foreach ($property in $Properties) { $arguments += $property }
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
$originalMachinePath = Get-MachinePathRaw
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
            @('Id="CliShortcut"', 'ReCraft CLI Start-menu shortcut'),
            @('Target="[SystemFolder]cmd.exe"', 'CLI shortcut target'),
            @('App Paths\recraft-cli.exe', 'CLI App Paths entry'),
            @('Software\RegisteredApplications', 'RegisteredApplications entry'),
            @('Software\ReCraft\Capabilities', 'Capabilities entry'),
            @('Id="DesktopFolder"', 'desktop-shortcut folder'),
            @('Id="RecraftPathEntry"', 'PATH option component'),
            @('Id="OptionsDlg"', 'install options page'),
            @('Name="PATH"', 'PATH environment entry'),
            @('Property="ADDTOPATH"', 'PATH checkbox'),
            @('Property="DESKTOPSHORTCUT"', 'desktop-shortcut checkbox'),
            @(('Codepage="{0}"' -f $msiCodepage), "$Culture string codepage"),
            @(('Language="{0}"' -f $msiLanguage), "$Culture package language"),
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

    # ---- 3. silent install into a temporary folder, opting out of PATH ---------------------------
    # Native CLIs write progress to stderr; Windows PowerShell would turn that into a terminating
    # NativeCommandError under ErrorActionPreference=Stop.
    $ErrorActionPreference = 'Continue'
    $target = Join-Path $env:TEMP ('recraft-msi-install-' + [guid]::NewGuid().ToString('N').Substring(0, 8))
    Write-Host "[install] $Msi -> $target (ADDTOPATH=0 DESKTOPSHORTCUT=1)"
    Install-Msi $target (Join-Path $work 'install.log') @('ADDTOPATH=0', 'DESKTOPSHORTCUT=1')

    foreach ($entry in $payload) {
        Assert-Condition (Test-Path -LiteralPath (Join-Path $target $entry)) "missing installed file: $entry"
    }
    $menuLinks = @(Get-ChildItem -LiteralPath $startMenu -Filter 'ReCraft*.lnk' -ErrorAction SilentlyContinue)
    Assert-Condition ($menuLinks.Count -eq 2) "expected the GUI and CLI Start-menu shortcuts, found $($menuLinks.Count)"
    Assert-Condition (Test-Path -LiteralPath $desktopShortcut) 'desktop shortcut missing although DESKTOPSHORTCUT=1'
    $machinePath = [Environment]::GetEnvironmentVariable('Path', 'Machine')
    Assert-Condition (-not (($machinePath -split ';') -contains $target)) 'ADDTOPATH=0 still added the install folder to the machine PATH'

    $arp = Find-ArpKey 'ReCraft'
    Assert-Condition ($null -ne $arp) 'Apps & Features entry missing'
    Assert-Condition ($arp.PSPath -like 'Microsoft.PowerShell.Core\Registry::HKEY_LOCAL_MACHINE*') "unexpected install scope: $($arp.PSPath)"
    $arpProps = Get-ItemProperty -LiteralPath $arp.PSPath
    Assert-Condition ($arpProps.DisplayName -eq 'ReCraft') "unexpected DisplayName: $($arpProps.DisplayName)"
    Assert-Condition ($arpProps.DisplayVersion -eq $version) "unexpected DisplayVersion: $($arpProps.DisplayVersion)"
    $installedCode = Split-Path -Leaf $arp.PSPath

    # CLI discovery and Settings > Default apps integration.
    $cliAppPathKey = 'HKLM:\Software\Microsoft\Windows\CurrentVersion\App Paths\recraft-cli.exe'
    $cliAppPath = Get-Item -LiteralPath $cliAppPathKey -ErrorAction SilentlyContinue
    Assert-Condition ($null -ne $cliAppPath -and $cliAppPath.GetValue('') -eq (Join-Path $target 'recraft-cli.exe')) 'CLI App Paths entry missing or pointing elsewhere'
    $capabilities = Get-ItemProperty -LiteralPath 'HKLM:\Software\ReCraft\Capabilities' -ErrorAction SilentlyContinue
    Assert-Condition ($capabilities.ApplicationName -eq 'ReCraft') 'Capabilities\ApplicationName missing'
    $fileAssociations = Get-ItemProperty -LiteralPath 'HKLM:\Software\ReCraft\Capabilities\FileAssociations' -ErrorAction SilentlyContinue
    Assert-Condition ($fileAssociations.'.rcraft' -eq 'ReCraft.Project') 'Capabilities\FileAssociations\.rcraft missing'
    $registeredApps = Get-ItemProperty -LiteralPath 'HKLM:\Software\RegisteredApplications' -ErrorAction SilentlyContinue
    Assert-Condition ($registeredApps.ReCraft -eq 'Software\ReCraft\Capabilities') 'RegisteredApplications\ReCraft missing'
    Write-Host "[install] files, GUI+CLI shortcuts, desktop shortcut, App Paths and Capabilities present ($($arpProps.DisplayName) $($arpProps.DisplayVersion), $installedCode)"

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
    Assert-Condition (@(Get-ChildItem -LiteralPath $startMenu -Filter 'ReCraft*.lnk' -ErrorAction SilentlyContinue).Count -eq 0) 'Start-menu shortcuts still exist after uninstall'
    Assert-Condition (-not (Test-Path -LiteralPath $desktopShortcut)) 'desktop shortcut still exists after uninstall'
    $machinePath = [Environment]::GetEnvironmentVariable('Path', 'Machine')
    Assert-Condition (-not (($machinePath -split ';') -contains $target)) 'install folder still on the machine PATH after uninstall'
    Assert-Condition ($null -eq (Find-ArpKey 'ReCraft')) 'Apps & Features entry still exists after uninstall'
    # MSI removes the values it wrote; the (now empty) keys may legitimately remain, so
    # assert on values rather than on key existence.
    $cliAppPath = Get-Item -LiteralPath $cliAppPathKey -ErrorAction SilentlyContinue
    Assert-Condition ($null -eq $cliAppPath -or -not $cliAppPath.GetValue('')) 'CLI App Paths value still exists after uninstall'
    $capabilities = Get-ItemProperty -LiteralPath 'HKLM:\Software\ReCraft\Capabilities' -ErrorAction SilentlyContinue
    Assert-Condition ($null -eq $capabilities.ApplicationName) 'Capabilities\ApplicationName still exists after uninstall'
    $registeredApps = Get-ItemProperty -LiteralPath 'HKLM:\Software\RegisteredApplications' -ErrorAction SilentlyContinue
    Assert-Condition ($null -eq $registeredApps.ReCraft) 'RegisteredApplications\ReCraft still exists after uninstall'
    $reCraftKey = Get-ItemProperty -LiteralPath 'HKLM:\Software\ReCraft' -ErrorAction SilentlyContinue
    Assert-Condition ($null -eq $reCraftKey.InstallDir -and $null -eq $reCraftKey.PathEntry -and $null -eq $reCraftKey.DesktopShortcut) 'Software\ReCraft values still exist after uninstall'
    Write-Host '[uninstall] directory, shortcuts, App Paths and Capabilities removed'
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
        Assert-Condition (-not (Test-Path -LiteralPath $desktopShortcut)) 'desktop shortcut created without DESKTOPSHORTCUT=1'
        $defaultMachinePath = [Environment]::GetEnvironmentVariable('Path', 'Machine')
        Assert-Condition (($defaultMachinePath -split ';') -contains $defaultTarget) 'the default install did not add the install folder to the machine PATH (ADDTOPATH defaults to 1)'
        Write-Host "[install] default folder OK ($defaultTarget, PATH added by default, no desktop shortcut)"

        Uninstall-Msi $installedCode (Join-Path $work 'uninstall-default.log')
        for ($attempt = 0; $attempt -lt 40; $attempt++) {
            if (-not (Test-Path -LiteralPath $defaultTarget)) { break }
            Start-Sleep -Milliseconds 500
        }
        Assert-Condition (-not (Test-Path -LiteralPath $defaultTarget)) 'default install directory still exists after uninstall'
        Assert-Condition ($null -eq (Find-ArpKey 'ReCraft')) 'Apps & Features entry still exists after the default uninstall'
        Assert-Condition (@(Get-ChildItem -LiteralPath $startMenu -Filter 'ReCraft*.lnk' -ErrorAction SilentlyContinue).Count -eq 0) 'Start-menu shortcuts still exist after the default uninstall'
        $defaultMachinePath = [Environment]::GetEnvironmentVariable('Path', 'Machine')
        Assert-Condition (-not (($defaultMachinePath -split ';') -contains $defaultTarget)) 'default install folder still on the machine PATH after uninstall'
        Write-Host '[uninstall] default folder removed'
        $installedCode = $null
    }

    Write-Host '[msi] PASS'
} finally {
    if ($installedCode) {
        Start-Process -FilePath 'msiexec.exe' -Wait -ArgumentList @('/x', $installedCode, '/qn') -ErrorAction SilentlyContinue
    }
    # The PATH install (on by default) extends the machine PATH; if the run failed before
    # the uninstall removed it, put the stored value (and its REG_EXPAND_SZ kind) back so a
    # rerun starts clean.
    if ($originalMachinePath) {
        $currentMachinePath = Get-MachinePathRaw
        if ($currentMachinePath -and $currentMachinePath.Value -ne $originalMachinePath.Value) {
            $key = [Microsoft.Win32.Registry]::LocalMachine.OpenSubKey($machineEnvironmentKey, $true)
            if ($key) {
                try {
                    $key.SetValue('Path', $originalMachinePath.Value, $originalMachinePath.Kind)
                    Write-Host '[uninstall] restored the machine PATH'
                } finally {
                    $key.Close()
                }
            }
        }
    }
    Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
}
