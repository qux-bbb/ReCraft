# Real-world fixture: Simple Crackme v2.0

`corpus/` above this directory is the **synthetic** corpus: tiny CRT-free probes whose export
table, `.pdata` and linker map are ground truth, gated at 31 functions / 100% structured /
97.4% coverage. This directory is the opposite end of the spectrum: a complete, shipped-style
program with the static CRT linked in, used as the real-world sample behind
`docs/cases/crackme_win_x64.md`.

Only the source and this build script are committed. The binary, map and `.rcraft` project are
build products (`.gitignore` covers `*.exe`, `*.rcraft` and `corpus/build/`), so the sample is
reproducible without vendoring a 146 KB executable. The upstream CTF project stays at
`D:\files\projects\ctf-simple-crackme`; `src/crackme.c` here is a frozen mirror of its source.

## Provenance

| | |
|---|---|
| Upstream source | `D:\files\projects\ctf-simple-crackme\src\crackme.c` |
| Source SHA-256 | `1AD09ADCC22E0E03E5E4BE9EDC004444D279C20CC54B1A29E63AB6761EEA28BF` |
| Binary SHA-256 | `6C84C77BFFFF5AD56123E3C2DCED03CF07E7DEAAFE9A3527E21E30FA8A38C5BC` |
| Binary | 146,432 bytes, x64, MSVC `/O2 /MT /GS- /DNDEBUG /TC /Brepro` |
| Functions | 495 (400 from `.pdata`, 1 entry, 85 call targets, 9 data pointers) |

The flags match the upstream build, so a rebuild is byte-identical (`/Brepro` stores a content
hash instead of a timestamp). The pinned SHA-256 therefore doubles as a toolchain check.

## Build

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File corpus/real/crackme/build.ps1
# corpus/build/real/crackme/crackme.exe  (+ crackme.map, obj/)
```

`build.ps1` prints the SHA-256 and warns when the rebuilt image no longer matches the pinned
one. That is MSVC drift, not a build failure: the code is the same, the machine code moved.
`crackme.map` (linker `/MAP`) lists every function symbol, including static ones, and is the
ground truth for `recraft-cli eval <exe> --map <map>`.

## Linux / ELF build

The same source also builds as a Linux ELF executable, which is what the ELF front end is
exercised against:

```bash
bash corpus/real/crackme/build-linux.sh              # corpus/build/real/crackme-linux/crackme.elf
CC=clang bash corpus/real/crackme/build-linux.sh     # any host compiler
bash corpus/real/crackme/build-linux.sh --snapshots  # + tests/golden/crackme-elf/
```

It is a dynamically linked PIE with glibc: `-O2` like the Windows build, unstripped so
`.symtab` still names `main` (at `-O2` GCC inlines `verify` into `main`, unlike MSVC). The
image embeds the host `crt1.o`/glibc startup code, so it is reproducible per toolchain, not
across them: the script prints its SHA-256, nothing is pinned, and no snapshot set is
committed. `--snapshots` writes `tests/golden/crackme-elf/` with the local `recraft-cli`
(`target/debug`, or `$RECRAFT_CLI`); the test checks that directory when it exists.

On Windows the script needs WSL or an Ubuntu VM; the test only looks for
`corpus/build/real/crackme-linux/crackme.elf`, so copy the file back when the VM does not
share this checkout. Ubuntu 24.04's gcc 13.3 yields 24 functions, 95.8% structured and 97.5%
coverage, which is what the test floors are set against.

## Snapshots and the gate

The committed snapshot set is `tests/golden/crackme/` — 160 files, written with
`--limit 160`, the same selection the case documentation uses. This fixture deliberately does
**not** duplicate it: while the rebuild is byte-identical there is exactly one set to review.

`tests/golden/crackme-real/` is the escape hatch for a divergent rebuild:

```powershell
... build.ps1 -Snapshots   # import + analyze + golden --limit 160 into tests/golden/crackme-real
```

As soon as that directory contains `.c` files, `the_real_crackme_fixture_matches_its_snapshots`
prefers it and `tests/golden/crackme` goes back to describing the original case binary.

The gate itself (skipped on a clean checkout, because the binary does not exist):

```powershell
cargo test -p recraft-decompile --lib real_crackme
```

It runs `lint` on both levels for the first 160 functions (the full 495 take ~20 s in a debug
build, too slow for every `cargo test`) plus `golden --check` against those 160 snapshots.
This is intentionally **not** wired into CI: the windows-latest MSVC version can differ from
the pinned one, which would make committed snapshots flaky — the same reason the other
`tests/golden/*` sets are checked by scripts rather than by CI.

The same filter also runs the ELF fixture tests (`the_real_crackme_elf_*`): full `lint` at
both levels on `crackme.elf`, a structured-rate/coverage floor, and the optional local
`tests/golden/crackme-elf/` check. The ELF build is wired into the Linux CI job, so this half
does run on every push; the PE half stays local.

A skip is a silent `ok`: both helpers `return` early when their binary is missing (clean
checkout, or the Windows CI job for the ELF case), so `cargo test` still reports success.
Run it with output to see which side ran:

```powershell
cargo test -p recraft-decompile --lib real_crackme -- --nocapture
# skipping: …\crackme.elf not found; run corpus/real/crackme/build-linux.sh on Linux
```

## Why not `corpus/`

- The synthetic corpus has complete ground truth (exports, `.pdata`, map) and is small enough
  to keep at 100% structured. This binary is a normal CRT program: no export table, 495
  functions, 257 inline-asm statements, 35 unstructured functions.
- Its snapshots describe one compiler build, so they belong next to the case they document,
  not in the 31-function CI gate.
- The upstream directory is a whole CTF project (IDA database, solve script, writeup). Only
  the single C file is mirrored here.

## Keeping it current

`src/crackme.c` is frozen. If the upstream source changes, copy it over, rebuild both images
(`build.ps1` and, on Linux, `build-linux.sh`), and update both SHA-256 pins in this file and
`docs/cases/crackme_win_x64.md`; then regenerate the 160 PE snapshots
(`golden ... --dir tests/golden/crackme --limit 160`), drop any stale
`tests/golden/crackme-elf/`, and review the diff.
