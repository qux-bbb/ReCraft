#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 ReCraft contributors
#
# Build the real-world crackme fixture as a Linux ELF executable. The same C
# source as the MSVC sample exercises the ELF front end: a dynamically linked
# PIE with glibc, `.symtab` kept so `main`/`verify` stay named.
#
#   bash corpus/real/crackme/build-linux.sh
#   CC=clang bash corpus/real/crackme/build-linux.sh
#   bash corpus/real/crackme/build-linux.sh --snapshots
#
# Output (build product, never committed):
#   corpus/build/real/crackme-linux/crackme.elf
#
# --snapshots needs a built `recraft-cli` (target/debug, or $RECRAFT_CLI) and
# writes `tests/golden/crackme-elf/`. Those snapshots depend on the host gcc and
# glibc, so they are only committed on purpose (see README.md).

set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
src="$repo/corpus/real/crackme/src/crackme.c"
out="$repo/corpus/build/real/crackme-linux"
cc="${CC:-gcc}"
snapshots=0

for arg in "$@"; do
    case "$arg" in
        --snapshots) snapshots=1 ;;
        -h | --help)
            sed -n '2,19p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        *)
            echo "unknown argument: $arg (try --help)" >&2
            exit 2
            ;;
    esac
done

mkdir -p "$out"

# -O2 matches the Windows case build and the ELF corpus, so the metrics stay
# comparable. No `-s`: the symbol table is the ground truth for the names.
"$cc" -O2 -o "$out/crackme.elf" "$src"

"$cc" --version | head -n 1
size=$(wc -c < "$out/crackme.elf")
digest=$(sha256sum "$out/crackme.elf" | cut -d' ' -f1)
echo "[real/crackme] built $out/crackme.elf ($size bytes)"
echo "[real/crackme] sha256 $digest"

if [ "$snapshots" = 1 ]; then
    cli="${RECRAFT_CLI:-$repo/target/debug/recraft-cli}"
    if [ ! -x "$cli" ]; then
        echo "recraft-cli not found at $cli; run 'cargo build -p recraft-cli' or set RECRAFT_CLI" >&2
        exit 1
    fi
    project="$out/crackme-elf.rcraft"
    dir="$repo/tests/golden/crackme-elf"
    rm -f "$project"
    "$cli" import "$out/crackme.elf" -o "$project"
    "$cli" analyze "$project"
    "$cli" golden "$project" --dir "$dir"
    echo "[real/crackme] snapshots written to $dir"
fi
