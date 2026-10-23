# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Project design document v0.1 (`docs/DESIGN.md`).
- Cargo workspace skeleton with the planned crate graph.
- `recraft-common`: addresses, ranges, ids and the shared error type.
- M0 implementation plan (`docs/M0.md`).
- ADR-002: egui is viable for the native UI (spike passed: 200k-row grid p95 0.62 ms,
  500-node CFG p95 2.43 ms on CPU layout + tessellation).
- `recraft-storage`: single-file SQLite project container (schema v1), 256 KiB chunked
  image import with progress/cancel and SHA-256, undo journal (append/undo/redo/truncate),
  query-only reader connections. 1 GiB import measured at 4.6 s; reopen at 3.1 ms.
- `recraft-format`: PE32/PE32+ loader — headers, sections, imports (incl. delay imports),
  exports (incl. forwarders), TLS callbacks, x64 `.pdata`, overlay, RVA mapping, `Loader`
  trait. Malformed import/export data degrades to warnings instead of failing the load.
  Smoke: `notepad.exe` (8 sections, 340 imports, 386 `.pdata`) and `kernel32.dll`
  (1,292 imports, 1,697 exports, 1,530 `.pdata`) parse cleanly.
- `recraft-decode`: stateless x86/x64 decoding on iced-x86 1.21 — `Insn` wrapper with
  `FlowKind` classification, direct branch targets, memory-operand detection, Intel
  formatter (mnemonic/operands split for UIs) and a bounded decode cache. 10 unit tests;
  stateless throughput measured at ~46 M instructions/s.
- `recraft-cli`: `recraft` command line — `info` (PE inspect without a project),
  `import`, `disasm`, `funcs`, `rename`/`unname`, `comment`/`uncomment`, `mark`,
  `undo`, `redo`, with `--json` output and symbol/`entry` address resolution.
  First end-to-end flow verified on `notepad.exe`: import → disasm at entry →
  rename/comment (Chinese) → undo → reopen (separate process) still shows the edits.
- `recraft-core`: session over the project database — segment map from the PE loader,
  4-bit-per-byte flags with a 64 KiB page cache, names/comments/functions, page-cached
  image reads (VA → file offset), linear disassembly view, and the command bus with
  journaled inverse operations (undo/redo, redo-stack invalidation, atomic apply).
  PE metadata is persisted as JSON in the project so reopening does not re-read the
  image. 10 unit tests (rename/comment/code-data/function undo-redo persistence).

[Unreleased]: https://github.com/qux-bbb/ReCraft/commits/main
