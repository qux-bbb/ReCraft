#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 ReCraft contributors
#
# Build the Linux ELF corpus: a shared object with every function exported, plus a
# stripped copy (no `.symtab`) that keeps only `.dynsym` and `.eh_frame`, and a C++
# object whose Itanium-mangled names exercise demangling.
#
#   bash corpus/build-linux.sh                # host compilers
#   CC=clang CXX=clang++ bash corpus/build-linux.sh
#
# Output (build products, never committed):
#   corpus/build/linux/libcorpus.so
#   corpus/build/linux/libcorpus-stripped.so
#   corpus/build/linux/libcorpus-cpp.so

set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
out="$repo/corpus/build/linux"
cc="${CC:-gcc}"
cxx="${CXX:-g++}"

mkdir -p "$out"

# Keep `.eh_frame` (the default on x86-64): stripped binaries rely on it for
# function boundaries. `-O2` matches the Windows corpus, so the metrics stay
# comparable.
"$cc" -O2 -fPIC -shared -o "$out/libcorpus.so" "$repo/corpus/linux/functions.c"
cp "$out/libcorpus.so" "$out/libcorpus-stripped.so"
if command -v strip >/dev/null 2>&1; then
    strip --strip-all "$out/libcorpus-stripped.so"
else
    rm -f "$out/libcorpus-stripped.so"
fi

if command -v "$cxx" >/dev/null 2>&1; then
    "$cxx" -O2 -fPIC -shared -o "$out/libcorpus-cpp.so" "$repo/corpus/linux/functions.cpp"
else
    rm -f "$out/libcorpus-cpp.so"
fi

"$cc" --version | head -n 1
ls -l "$out"/libcorpus*.so 2>/dev/null || true
echo "[linux] built $out/libcorpus.so"
