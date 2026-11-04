# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 ReCraft contributors
#
# License/notice compliance check (DESIGN §12): every tracked source file must carry an
# SPDX header, the legal files must exist, and NOTICE must list the third-party licenses
# we rely on. Golden snapshots are generated output and are covered by the repository
# license instead of per-file headers.
#
#   pwsh -File scripts/check-license.ps1

[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
Set-Location $repo

$failures = New-Object System.Collections.Generic.List[string]

# 1. Required legal/notice files.
$required = @(
    'LICENSE',
    'CONTRIBUTING.md',
    'TRADEMARK.md',
    'SECURITY.md',
    'NOTICE.md',
    'deny.toml'
)
foreach ($file in $required) {
    if (-not (Test-Path -LiteralPath $file)) {
        $failures.Add("missing required file: $file")
    }
}

# 2. SPDX headers on hand-written source files (golden snapshots are generated).
$sourceExtensions = @('.rs', '.ps1', '.c', '.h', '.nsi')
$sourceFiles = Get-ChildItem -Path 'crates', 'scripts', 'corpus', 'tests', 'packaging' -Recurse -File -ErrorAction SilentlyContinue |
    Where-Object { $sourceExtensions -contains $_.Extension.ToLowerInvariant() } |
    Where-Object { $_.FullName -notmatch '[\\/]tests[\\/]golden[\\/]' }

$missing = 0
foreach ($file in $sourceFiles) {
    $head = Get-Content -LiteralPath $file.FullName -TotalCount 3 -ErrorAction SilentlyContinue
    if (-not (($head -join "`n") -match 'SPDX-License-Identifier:\s*AGPL-3\.0-only')) {
        $missing++
        $failures.Add("missing SPDX header: $($file.FullName.Replace($repo + [IO.Path]::DirectorySeparatorChar, ''))")
    }
}

# 3. NOTICE must mention the key third-party licenses used directly.
$notice = Get-Content -LiteralPath 'NOTICE.md' -Raw
foreach ($needle in @('egui', 'egui_dock', 'iced-x86', 'rusqlite', 'SQLite', 'serde', 'clap')) {
    if (-not $notice.Contains($needle)) {
        $failures.Add("NOTICE.md does not mention $needle")
    }
}

if ($failures.Count -gt 0) {
    Write-Host "[license] FAIL ($($failures.Count) problem(s))"
    foreach ($failure in $failures) {
        Write-Host "  - $failure"
    }
    exit 1
}

Write-Host "[license] OK: $($sourceFiles.Count) source file(s) with SPDX headers, $($required.Count) legal files present, NOTICE covers direct dependencies"
