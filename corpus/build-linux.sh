#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 ReCraft contributors
#
# Build the Linux ELF corpus: a shared object with every function exported, plus a
# stripped copy (no `.symtab`) that keeps only `.dynsym` and `.eh_frame`, a C++
# object whose Itanium-mangled names exercise demangling, a Rust `cdylib` with the
# shapes only a Rust binary has (panic/unwind machinery, vtables, closures, `fmt!`),
# and a behaviour sample (open/write, socket/connect, ptrace, dlopen, `/proc` reads)
# for the Linux API classification table.
#
#   bash corpus/build-linux.sh                # host compilers
#   CC=clang CXX=clang++ bash corpus/build-linux.sh
#
# Output (build products, never committed):
#   corpus/build/linux/libcorpus.so
#   corpus/build/linux/libcorpus-stripped.so
#   corpus/build/linux/libcorpus-cpp.so
#   corpus/build/linux/libcorpus-rust.so
#   corpus/build/linux/libcorpus-behaviors.so

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

# A `-g` build so the DWARF line table is exercised end to end.
"$cc" -O2 -g -fPIC -shared -o "$out/libcorpus-debug.so" "$repo/corpus/linux/functions.c"

if command -v "$cxx" >/dev/null 2>&1; then
    "$cxx" -O2 -fPIC -shared -o "$out/libcorpus-cpp.so" "$repo/corpus/linux/functions.cpp"
else
    rm -f "$out/libcorpus-cpp.so"
fi

# Rust cdylib: the same shapes as `corpus/rust/lib.rs` documents (panic/unwind,
# vtables, closures, `format!`, monomorphizations) plus `.symtab`, so `corpus_rust::…`
# names reach the legacy Itanium demangler. CI installs a toolchain; a bare machine
# skips the sample instead of failing the whole corpus build.
if command -v rustc >/dev/null 2>&1; then
    rustc --edition 2021 --crate-name corpus_rust --crate-type cdylib \
        -C opt-level=2 -C debuginfo=0 \
        -o "$out/libcorpus-rust.so" "$repo/corpus/rust/lib.rs"
    echo "[linux] built $out/libcorpus-rust.so"
else
    echo "[linux] skipping libcorpus-rust.so (rustc not installed)"
fi

# The behaviour sample exercises the Linux API/path classification table. `-ldl` keeps
# `dlopen` linkable on glibc before 2.34 (where it is not part of libc).
"$cc" -O2 -fPIC -shared -o "$out/libcorpus-behaviors.so" \
    "$repo/corpus/linux/behaviors.c" -ldl

"$cc" --version | head -n 1
ls -l "$out"/libcorpus*.so 2>/dev/null || true
echo "[linux] built $out/libcorpus.so"

# 32-bit variants: the same sources with `-m32`, so the ELF32 loader and the i386
# decompiler path have a real-toolchain corpus. Skipped when the multilib headers are
# missing (`sudo apt-get install gcc-multilib` on Debian/Ubuntu).
if "$cc" -m32 -O2 -fPIC -shared -o "$out/libcorpus32.so" "$repo/corpus/linux/functions.c" 2>/dev/null; then
    cp "$out/libcorpus32.so" "$out/libcorpus32-stripped.so"
    if command -v strip >/dev/null 2>&1; then
        strip --strip-all "$out/libcorpus32-stripped.so"
    else
        rm -f "$out/libcorpus32-stripped.so"
    fi
    echo "[linux] built $out/libcorpus32.so"
else
    echo "[linux] skipping libcorpus32.so (no 32-bit libc; install gcc-multilib)"
fi
