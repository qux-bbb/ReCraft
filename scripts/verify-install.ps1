# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 ReCraft contributors
#
# Clean-machine install -> GUI -> CLI -> uninstall verification for the ReCraft MSI
# (DESIGN section 13). Unlike scripts/verify-msi.ps1, which drives silent installs and
# table/payload assertions, this one exercises the user-visible path on a fresh machine:
# install with the default options, check every artifact, run the installed CLI on
# notepad.exe, open the GUI (window + screenshot + WM_CLOSE), uninstall, and assert the
# machine is back to its pre-install state (files, shortcuts, PATH, ARP, App Paths,
# Capabilities, RegisteredApplications, Software\ReCraft).
#
# The MSI is per-machine, so run this from an elevated prompt. It writes its own log
# because VMware VIX guest runs cannot capture stdout; on a VM, drive it with
#   vmrun -T ws -gu <user> -gp <pass> runProgramInGuest <vmx> -interactive cmd.exe /c run.bat
#
#   powershell -NoProfile -ExecutionPolicy Bypass -File scripts/verify-install.ps1 `
#       -Msi dist\recraft-v0.1.0-windows-x64.msi
#   ... -OutDir .tmp\verify-install    # default: <repo>\.tmp\verify-install
#   ... -SkipGui                       # headless runs
#
# Exit code 0 = every check passed (the log ends with "RESULT PASS").

[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Msi,
    [string]$OutDir = '',
    [string]$ExpectedVersion = '',
    [switch]$SkipGui
)

$repo = Split-Path -Parent $PSScriptRoot
if (-not $OutDir) { $OutDir = Join-Path $repo '.tmp\verify-install' }
$Msi = (Resolve-Path -LiteralPath $Msi).Path
if (-not $ExpectedVersion) {
    # recraft-v<version>-windows-x64.msi
    $ExpectedVersion = ($Msi | Split-Path -Leaf) -replace '^recraft-v', '' -replace '-windows-x64\.msi$', ''
}

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

if (-not (Test-Path -LiteralPath $OutDir)) { New-Item -ItemType Directory -Force -Path $OutDir | Out-Null }
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$log = Join-Path $OutDir "verify-install-$stamp.log"
$script:failures = 0

function Write-Log([string]$Text) {
    $line = "[{0}] {1}" -f (Get-Date -Format 'HH:mm:ss'), $Text
    Write-Host $line
    Add-Content -LiteralPath $log -Value $line -Encoding UTF8
}

function Check([bool]$Condition, [string]$What) {
    if ($Condition) { Write-Log "ok   $What" } else { $script:failures++; Write-Log "FAIL $What" }
}

function Get-MachinePathRaw {
    $key = [Microsoft.Win32.Registry]::LocalMachine.OpenSubKey('SYSTEM\CurrentControlSet\Control\Session Manager\Environment', $false)
    if ($null -eq $key) { return '' }
    try { return [string]$key.GetValue('Path', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames) }
    finally { $key.Close() }
}

function Test-PathEntry([string]$PathValue, [string]$Folder) {
    $wanted = $Folder.TrimEnd('\')
    foreach ($entry in ($PathValue -split ';')) { if ($entry.TrimEnd('\') -ieq $wanted) { return $true } }
    return $false
}

function Find-ArpKey([string]$DisplayName) {
    foreach ($root in @('HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall',
                        'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall',
                        'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall')) {
        if (-not (Test-Path -LiteralPath $root)) { continue }
        foreach ($key in Get-ChildItem -LiteralPath $root -ErrorAction SilentlyContinue) {
            $props = Get-ItemProperty -LiteralPath $key.PSPath -ErrorAction SilentlyContinue
            if ($props.DisplayName -eq $DisplayName) { return $key }
        }
    }
    return $null
}

function Get-ReCraftFiles {
    $files = @(
        'recraft.exe', 'recraft-cli.exe', 'LICENSE', 'NOTICE.md', 'README.md', 'CHANGELOG.md', 'VERSION.txt',
        'docs\DESIGN.md', 'docs\FAQ.md', 'docs\RELEASE.md'
    )
    return $files
}

function Wait-For([scriptblock]$Condition, [int]$Seconds) {
    for ($i = 0; $i -lt ($Seconds * 2); $i++) {
        if (& $Condition) { return $true }
        Start-Sleep -Milliseconds 500
    }
    return (& $Condition)
}

# ---- Win32 helpers --------------------------------------------------------------------------
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class W32 {
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc cb, IntPtr lp);
    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr hWnd, StringBuilder s, int n);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdc, uint flags);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT r);
    [DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr hWnd, uint msg, IntPtr w, IntPtr l);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int cmd);
    public struct RECT { public int Left, Top, Right, Bottom; }
    public static System.Collections.Generic.List<IntPtr> WindowsOf(uint pid) {
        var list = new System.Collections.Generic.List<IntPtr>();
        EnumWindows((h, l) => {
            uint p; GetWindowThreadProcessId(h, out p);
            if (p == pid) list.Add(h);
            return true;
        }, IntPtr.Zero);
        return list;
    }
}
'@

function Get-WindowTitle([IntPtr]$hWnd) {
    $sb = New-Object System.Text.StringBuilder 512
    [void][W32]::GetWindowTextW($hWnd, $sb, $sb.Capacity)
    return $sb.ToString()
}

function Save-WindowShot([IntPtr]$hWnd, [string]$Path) {
    $rect = New-Object W32+RECT
    [void][W32]::GetWindowRect($hWnd, [ref]$rect)
    $w = $rect.Right - $rect.Left
    $h = $rect.Bottom - $rect.Top
    if ($w -le 0 -or $h -le 0) { throw "window has no size ($w x $h)" }
    $bmp = New-Object System.Drawing.Bitmap $w, $h
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $hdc = $g.GetHdc()
    $ok = [W32]::PrintWindow($hWnd, $hdc, 2)  # PW_RENDERFULLCONTENT
    $g.ReleaseHdc($hdc)
    $g.Dispose()
    $bmp.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    if (-not $ok) { throw 'PrintWindow failed' }
}

# =============================================================================================
Write-Log "=== ReCraft MSI clean-install verification ==="
Write-Log "host      $(Get-CimInstance Win32_OperatingSystem -ErrorAction SilentlyContinue | ForEach-Object { $_.Caption } ) $([Environment]::OSVersion.Version)"
Write-Log "user      $([Security.Principal.WindowsIdentity]::GetCurrent().Name)"
Write-Log "admin     $((New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator))"
Write-Log "msi       $Msi"
Write-Log "msi size  $([math]::Round((Get-Item -LiteralPath $Msi).Length / 1MB, 2)) MiB"
Write-Log "msi sha   $((Get-FileHash -LiteralPath $Msi -Algorithm SHA256).Hash)"

$installDir = Join-Path $env:ProgramFiles 'ReCraft'
$startMenu = Join-Path $env:ProgramData 'Microsoft\Windows\Start Menu\Programs'
$desktop = [Environment]::GetFolderPath('CommonDesktopDirectory')
$desktopShortcut = Join-Path $desktop 'ReCraft.lnk'
$machinePathBefore = Get-MachinePathRaw
$work = Join-Path $env:TEMP ('recraft-clean-' + [guid]::NewGuid().ToString('N').Substring(0, 8))
New-Item -ItemType Directory -Force -Path $work | Out-Null
$productCode = $null

try {
    # ---- pre-flight: the machine must be clean ------------------------------------------
    Write-Log "--- pre-flight (must be a clean machine) ---"
    Check (-not (Test-Path -LiteralPath $installDir)) "no previous install at $installDir"
    Check ($null -eq (Find-ArpKey 'ReCraft')) 'no ReCraft Apps & Features entry before install'
    Check (@(Get-ChildItem -LiteralPath $startMenu -Filter 'ReCraft*.lnk' -ErrorAction SilentlyContinue).Count -eq 0) 'no ReCraft Start-menu shortcuts before install'
    Check (-not (Test-Path -LiteralPath $desktopShortcut)) 'no desktop shortcut before install'
    Check (-not (Test-PathEntry $machinePathBefore $installDir)) 'install folder not on the machine PATH before install'
    if (Test-Path -LiteralPath 'HKLM:\Software\ReCraft') {
        $stale = Get-ItemProperty -LiteralPath 'HKLM:\Software\ReCraft' -ErrorAction SilentlyContinue
        Write-Log "note HKLM\Software\ReCraft exists before install (InstallDir=$($stale.InstallDir) PathEntry=$($stale.PathEntry))"
    }

    # ---- install -------------------------------------------------------------------------
    Write-Log "--- install (defaults: PATH on, desktop shortcut off) ---"
    $installLog = Join-Path $work 'install.log'
    $p = Start-Process -FilePath 'msiexec.exe' -Wait -PassThru -ArgumentList @(
        '/i', $Msi, '/qn', '/l*v', $installLog)
    Check ($p.ExitCode -eq 0) "msiexec /i exit code ($($p.ExitCode))"
    if ($p.ExitCode -ne 0) {
        Write-Log ((Get-Content -LiteralPath $installLog -Tail 30 -ErrorAction SilentlyContinue) -join "`n")
        throw 'install failed'
    }

    Check (Wait-For { Test-Path -LiteralPath (Join-Path $installDir 'recraft.exe') } 30) "installed recraft.exe at $installDir"
    $missing = @(Get-ReCraftFiles | Where-Object { -not (Test-Path -LiteralPath (Join-Path $installDir $_)) })
    Check ($missing.Count -eq 0) "installed payload complete ($(@(Get-ReCraftFiles).Count) files; missing: $($missing -join ', '))"

    $links = @(Get-ChildItem -LiteralPath $startMenu -Filter 'ReCraft*.lnk' -ErrorAction SilentlyContinue)
    Check ($links.Count -eq 2) "Start-menu shortcuts present ($($links.Name -join ', '))"
    Check (-not (Test-Path -LiteralPath $desktopShortcut)) 'no desktop shortcut by default (DESKTOPSHORTCUT unset)'

    $machinePath = Get-MachinePathRaw
    Check (Test-PathEntry $machinePath $installDir) 'install folder added to the machine PATH'

    $arp = Find-ArpKey 'ReCraft'
    Check ($null -ne $arp) 'Apps & Features entry created'
    if ($arp) {
        $props = Get-ItemProperty -LiteralPath $arp.PSPath
        $productCode = Split-Path -Leaf $arp.PSPath
        Write-Log "info DisplayName=$($props.DisplayName) DisplayVersion=$($props.DisplayVersion) ProductCode=$productCode"
        Check ($props.DisplayVersion -eq $ExpectedVersion) "Apps & Features DisplayVersion is $ExpectedVersion"
    }

    $appPath = Get-Item -LiteralPath 'HKLM:\Software\Microsoft\Windows\CurrentVersion\App Paths\recraft-cli.exe' -ErrorAction SilentlyContinue
    Check ($null -ne $appPath -and $appPath.GetValue('') -eq (Join-Path $installDir 'recraft-cli.exe')) 'App Paths\recraft-cli.exe points at the installed CLI'
    $cap = Get-ItemProperty -LiteralPath 'HKLM:\Software\ReCraft\Capabilities' -ErrorAction SilentlyContinue
    Check ($cap.ApplicationName -eq 'ReCraft') 'Capabilities\ApplicationName registered'
    $regApps = Get-ItemProperty -LiteralPath 'HKLM:\Software\RegisteredApplications' -ErrorAction SilentlyContinue
    Check ($regApps.ReCraft -eq 'Software\ReCraft\Capabilities') 'RegisteredApplications\ReCraft registered'

    # ---- CLI from the installed location ---------------------------------------------------
    Write-Log "--- CLI (installed) ---"
    $cli = Join-Path $installDir 'recraft-cli.exe'
    $ErrorActionPreference = 'Continue'
    $versionOut = (& $cli --version 2>$null) -join ' '
    Check ($versionOut -match '0\.1\.0') "recraft-cli --version ($versionOut)"
    $info = (& $cli info (Join-Path $env:SystemRoot 'System32\notepad.exe') 2>$null) -join "`n"
    Check ($info -match 'PE32') 'recraft-cli info notepad.exe reports a PE image'
    $project = Join-Path $work 'notepad.rcraft'
    & $cli import (Join-Path $env:SystemRoot 'System32\notepad.exe') -o $project 2>$null | Out-Null
    Check (Test-Path -LiteralPath $project) 'recraft-cli import created a project'
    & $cli analyze $project 2>$null | Out-Null
    $metrics = (& $cli metrics $project --json 2>$null) | ConvertFrom-Json
    Check ($metrics.functions -gt 100) "recraft-cli analyze/metrics (functions=$($metrics.functions), coverage=$([math]::Round($metrics.coverage * 100, 1))%)"
    $ErrorActionPreference = 'Stop'

    # ---- GUI ------------------------------------------------------------------------------
    if (-not $SkipGui) {
        Write-Log "--- GUI ---"
        $exe = Join-Path $installDir 'recraft.exe'
        $proc = Start-Process -FilePath $exe -PassThru
        $hwnd = [IntPtr]::Zero
        $title = ''
        $deadline = (Get-Date).AddSeconds(40)
        while ((Get-Date) -lt $deadline) {
            foreach ($h in [W32]::WindowsOf([uint32]$proc.Id)) {
                if ([W32]::IsWindowVisible($h)) {
                    $t = Get-WindowTitle $h
                    if ($t) { $hwnd = $h; $title = $t; break }
                }
            }
            if ($hwnd -ne [IntPtr]::Zero) { break }
            Start-Sleep -Milliseconds 500
        }
        Check ($hwnd -ne [IntPtr]::Zero) "main window appeared (title: '$title')"
        if ($hwnd -ne [IntPtr]::Zero) {
            [void][W32]::ShowWindow($hwnd, 9)  # SW_RESTORE
            [void][W32]::SetForegroundWindow($hwnd)
            Start-Sleep -Milliseconds 1500
            $shot = Join-Path $OutDir "gui-$stamp.png"
            try { Save-WindowShot $hwnd $shot; Check (Test-Path -LiteralPath $shot) "window screenshot saved ($shot, $([math]::Round((Get-Item $shot).Length/1KB,1)) KiB)" }
            catch { Check $false "window screenshot ($($_.Exception.Message))" }
            [void][W32]::PostMessageW($hwnd, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)  # WM_CLOSE
            $exited = $proc.WaitForExit(15000)
            Check ($exited) 'GUI closed cleanly on WM_CLOSE'
            if (-not $exited) { $proc.Kill() }
        }
    }

    # ---- uninstall ------------------------------------------------------------------------
    Write-Log "--- uninstall ---"
    $uninstallLog = Join-Path $work 'uninstall.log'
    $p = Start-Process -FilePath 'msiexec.exe' -Wait -PassThru -ArgumentList @(
        '/x', $productCode, '/qn', '/l*v', $uninstallLog)
    Check ($p.ExitCode -eq 0) "msiexec /x exit code ($($p.ExitCode))"

    Check (Wait-For { -not (Test-Path -LiteralPath $installDir) } 60) 'install directory removed'
    $links = @(Get-ChildItem -LiteralPath $startMenu -Filter 'ReCraft*.lnk' -ErrorAction SilentlyContinue)
    Check ($links.Count -eq 0) 'Start-menu shortcuts removed'
    Check (-not (Test-Path -LiteralPath $desktopShortcut)) 'desktop shortcut absent'
    $machinePath = Get-MachinePathRaw
    Check (-not (Test-PathEntry $machinePath $installDir)) 'install folder removed from the machine PATH'
    Check ($null -eq (Find-ArpKey 'ReCraft')) 'Apps & Features entry removed'
    $appPath = Get-Item -LiteralPath 'HKLM:\Software\Microsoft\Windows\CurrentVersion\App Paths\recraft-cli.exe' -ErrorAction SilentlyContinue
    Check ($null -eq $appPath -or -not $appPath.GetValue('')) 'App Paths value removed'
    $cap = Get-ItemProperty -LiteralPath 'HKLM:\Software\ReCraft\Capabilities' -ErrorAction SilentlyContinue
    Check ($null -eq $cap.ApplicationName) 'Capabilities values removed'
    $regApps = Get-ItemProperty -LiteralPath 'HKLM:\Software\RegisteredApplications' -ErrorAction SilentlyContinue
    Check ($null -eq $regApps.ReCraft) 'RegisteredApplications\ReCraft removed'
    $reCraft = Get-ItemProperty -LiteralPath 'HKLM:\Software\ReCraft' -ErrorAction SilentlyContinue
    Check ($null -eq $reCraft.InstallDir -and $null -eq $reCraft.PathEntry -and $null -eq $reCraft.DesktopShortcut) 'Software\ReCraft values removed'
    Check ($machinePath -eq $machinePathBefore) 'machine PATH restored to its pre-install value'
    $productCode = $null
}
catch {
    $script:failures++
    Write-Log "FAIL unexpected: $($_.Exception.Message)"
    Write-Log ($_.ScriptStackTrace -replace "`n", ' | ')
}
finally {
    if ($productCode) {
        Start-Process -FilePath 'msiexec.exe' -Wait -ArgumentList @('/x', $productCode, '/qn') -ErrorAction SilentlyContinue
    }
    Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
    if ($script:failures -eq 0) { Write-Log 'RESULT PASS' } else { Write-Log "RESULT FAIL ($($script:failures) checks failed)" }
    Write-Log "log: $log"
}

if ($script:failures -ne 0) { exit 1 }
