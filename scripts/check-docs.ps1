# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 ReCraft contributors
#
# Checks that relative Markdown links in README.md and docs/**.md resolve to real files.
# External links (http/https/mailto) and in-page anchors are skipped.

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$files = @(Join-Path $root 'README.md')
$files += Get-ChildItem -Path (Join-Path $root 'docs') -Filter *.md -Recurse |
    ForEach-Object { $_.FullName }

$linkPattern = '\[[^\]]*\]\(([^)]+)\)'
$checked = 0
$failures = @()

foreach ($file in $files) {
    $text = Get-Content -Raw -LiteralPath $file
    foreach ($match in [regex]::Matches($text, $linkPattern)) {
        $target = $match.Groups[1].Value.Trim()
        if ($target -match '^(https?://|mailto:|#)') { continue }
        $path = $target.Split('#')[0].Trim()
        if (-not $path) { continue }
        $path = [uri]::UnescapeDataString($path)
        $resolved = if ([System.IO.Path]::IsPathRooted($path)) {
            $path
        } else {
            Join-Path (Split-Path -Parent $file) $path
        }
        $checked++
        if (-not (Test-Path -LiteralPath $resolved)) {
            $relative = [System.IO.Path]::GetRelativePath($root, $file)
            $failures += "$relative -> $target"
        }
    }
}

if ($failures.Count -gt 0) {
    Write-Host "[docs] $($failures.Count) broken link(s):"
    foreach ($failure in $failures) { Write-Host "  $failure" }
    exit 1
}
Write-Host "[docs] $checked relative link(s) OK across $($files.Count) file(s)"
