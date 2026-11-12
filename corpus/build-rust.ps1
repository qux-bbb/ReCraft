# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 ReCraft contributors
#
# Build the Rust evaluation corpus as a Windows PE cdylib.
#
#   pwsh -File corpus/build-rust.ps1
#
# Output: corpus/build/rust/corpus-rust.dll
#
# The cdylib keeps its export table, which is what the PE analysis path names
# functions from (`rust_add_leaf`, `rust_entry`, ...). The Rust-internal
# `corpus_rust::…` symbols only survive in an ELF `.symtab`, so demangling is
# exercised by the Linux build (`corpus/build-linux.sh`), not this one.

[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$source = Join-Path $PSScriptRoot 'rust\lib.rs'
$outDir = Join-Path $PSScriptRoot 'build\rust'
New-Item -ItemType Directory -Force -Path $outDir | Out-Null

$rustc = Get-Command rustc -ErrorAction SilentlyContinue
if (-not $rustc) {
    $candidate = Join-Path $env:USERPROFILE '.cargo\bin\rustc.exe'
    if (Test-Path $candidate) {
        $rustc = Get-Item $candidate
    } else {
        throw 'rustc not found on PATH (install the Rust toolchain from https://rustup.rs)'
    }
}

$dll = Join-Path $outDir 'corpus-rust.dll'
Remove-Item $dll, "$dll.lib", "$dll.exp", "$dll.pdb" -Force -ErrorAction SilentlyContinue

# opt-level=2 mirrors the C corpus, `-C debuginfo=0` keeps the image small, and the
# panic strategy stays at the default (`unwind`) so the panic/unwinding paths and
# their `.pdata` entries exist like in a real Rust binary.
& $rustc.Source --edition 2021 --crate-name corpus_rust --crate-type cdylib `
    -C opt-level=2 -C debuginfo=0 -o $dll $source
if ($LASTEXITCODE -ne 0) {
    throw "rustc failed with exit code $LASTEXITCODE"
}
if (-not (Test-Path $dll)) {
    throw 'corpus-rust.dll was not produced'
}

Write-Host ('[corpus-rust] built {0} ({1:N0} bytes)' -f $dll, (Get-Item $dll).Length)
