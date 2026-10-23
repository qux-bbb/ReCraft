# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- M1 control-flow graphs: `recraft-analysis::build_cfg` splits a function into basic
  blocks (leaders = entry, branch targets, post-branch instructions) and connects them
  with fallthrough/taken/not-taken/jump edges, tracking external tail-call branches;
  `layers()` gives a BFS depth layout. New Graph tab in the UI draws nodes and bezier
  edges (click a node to jump), and `recraft cfg --addr` prints or emits the block table
  as JSON. Tests cover branches, loops (back edge), notepad's entry CFG and a headless
  graph render.

- M1 UI side panels: Segments / Functions / Strings / Imports / Xrefs tabs. Strings has a
  live substring filter (first 2000 matches rendered) and click-to-navigate; Imports shows
  `module!api [category] ×refs` and jumps to the first call site; Xrefs lists references to
  the currently selected line and follows them. Disassembly now shows `sub_...` and IAT
  annotations. New headless test renders all five tabs against a real analysed notepad
  project.

- M1 import naming + API tagging: IAT slots are named `MODULE.dll!Api` at load time,
  disassembly annotates direct memory operands and call targets with resolved names
  (`jmp qword ptr [IAT] ; COMCTL32.dll!#413`), and analysis writes `sub_<hex>` names for
  unnamed functions (auto kind, never overriding loader/user names). Ordered substring
  rules categorize imports (network/file/registry/process/thread/crypto/service/
  anti-analysis/ui/memory) into derived `api_tag` rows. New `recraft imports` command
  lists 340 imported APIs for `notepad.exe` with categories and reference counts; analyze
  emits 422 API tags and 442 auto names in 37 ms.

- M1 strings + xrefs: ASCII/UTF-16LE string extraction (chunked, cross-chunk run
  continuation, ASCII-tail false-positive guard) and xref building for direct calls,
  branches and data references (absolute / RIP-relative). Persisted as derived data into
  the `string` and `xref` tables. New `recraft strings` and `recraft xrefs --addr`
  commands; `analyze` reports all counts. `notepad.exe`: 3,578 strings, 618 call edges,
  1,533 branch edges, 1,348 data refs in 34.6 ms. Corpus test verifies the
  `lea rax,[rip+…]` data xref to the banner string.

- M1: automatic function discovery (`recraft-analysis`) — seeds from `.pdata`, exports,
  entry point and TLS callbacks, then recursive descent with tail-call detection. Exact
  `.pdata` boundaries; functions are persisted as derived data (auto flag) without
  clobbering user definitions.
- `corpus/`: CRT-free MSVC benchmark (14 exports) where exports + `.pdata` are ground
  truth; `corpus/build.ps1` locates MSVC via `vswhere`.
- `recraft eval` / `recraft analyze` CLI commands, plus an "Analyze functions" button
  (F5) in the UI. Baseline: `notepad.exe` recall 1.000 / 386-of-386 `.pdata` boundaries;
  `kernel32.dll` recall 1.000 over 2460 analysable functions, precision 0.955.
- `docs/M1.md`: M1 status, metrics and next steps.

- M0 complete: end-to-end acceptance (`scripts/e2e.ps1`) covering import → info/disasm →
  rename + CJK comment → L0 decompile → undo/redo → JSON → code/data marks, plus an
  optional 1 GiB overlay smoke (4.44 s import). Wired into CI.
- `docs/M0-REPORT.md`: delivered capabilities, measured metrics, known limitations and
  the M1 plan. README now marks M0 complete.
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
- `recraft-ui`: native egui 0.36 desktop shell on the **Glow** renderer (wgpu/DX12
  crashed on Intel drivers during the spike) — virtualized disassembly Grid with
  address/bytes/mnemonic/operands/name/comment columns, L0 pseudo-C pane with
  click-to-jump, segment/function sidebar, goto dialog, rename/comment dialogs,
  undo/redo, keyboard-first navigation (G/N/;/Space/Esc/Ctrl+Z), CJK font fallback and
  English/简体中文 switching. Includes a headless egui layout test that renders the real
  400-row grid without a window.
- `recraft-cli`: `recraft` command line — `info` (PE inspect without a project),
  `import`, `disasm`, `decompile`, `funcs`, `rename`/`unname`, `comment`/`uncomment`,
  `mark`, `undo`, `redo`, with `--json` output and symbol/`entry` address resolution.
  Verified end-to-end on `notepad.exe`: import → disasm at entry → rename/comment
  (CJK) → L0 pseudo-C → undo → reopen in a new process still shows the edits.
- `recraft-ir`: register views (al/ax/eax/rax as one base register), expression arena,
  straight-line statements, x86/x64 lifting for mov/movzx/movsx/lea/arith/inc/dec/
  shifts/cmp/test/call/jmp/jcc/ret, and lazy flags (only the comparison feeding the
  next branch is tracked). Unsupported instructions become intrinsics that keep the
  original assembly — lifting never fails. 10 unit tests.
- `recraft-decompile`: **L0 pseudo-C** renderer — flat, instruction-faithful C-like
  output with true/signed displacement formatting, local labels for in-range branches,
  tail-call rendering for out-of-range jumps, call/name resolution and per-line source
  addresses for cross-highlighting. 2 integration tests on `notepad.exe`.
- `recraft-core`: session over the project database — segment map from the PE loader,
  4-bit-per-byte flags with a 64 KiB page cache, names/comments/functions, page-cached
  image reads (VA → file offset), linear disassembly view, and the command bus with
  journaled inverse operations (undo/redo, redo-stack invalidation, atomic apply).
  PE metadata is persisted as JSON in the project so reopening does not re-read the
  image. 10 unit tests (rename/comment/code-data/function undo-redo persistence).

[Unreleased]: https://github.com/qux-bbb/ReCraft/commits/main
