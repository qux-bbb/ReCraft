#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 ReCraft contributors
#
# Build the Linux ELF corpus: a shared object with every function exported, plus a
# stripped copy (no `.symtab`) that keeps only `.dynsym` and `.eh_frame`.
#
#   bash corpus/build-linux.sh                # host compiler
#   CC=clang bash corpus/build-linux.sh       # a specific compiler
#
# Output (build products, never committed):
#   corpus/build/linux/libcorpus.so
#   corpus/build/linux/libcorpus-stripped.so

set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
out="$repo/corpus/build/linux"
source="$repo/corpus/linux/functions.c"
cc="${CC:-gcc}"

mkdir -p "$out"

# Keep `.eh_frame` (the default on x86-64): stripped binaries rely on it for
# function boundaries. `-O2` matches the Windows corpus, so the metrics stay
# comparable.
"$cc" -O2 -fPIC -shared -o "$out/libcorpus.so" "$source"
cp "$out/libcorpus.so" "$out/libcorpus-stripped.so"
if command -v strip >/dev/null 2>&1; then
    strip --strip-all "$out/libcorpus-stripped.so"
else
    rm -f "$out/libcorpus-stripped.so"
fi

"$cc" --version | head -n 1
ls -l "$out/libcorpus.so" "$out/libcorpus-stripped.so" 2>/dev/null || true
echo "[linux] built $out/libcorpus.so"
