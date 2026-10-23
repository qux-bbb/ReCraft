# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- LLM provider contract + prompt safety layer (no network I/O yet): `ProviderSettings` is
  offline by default and only enables an HTTP provider when kind, endpoint and the API
  key env var are all present; `redact` masks URLs/IPs/e-mails; `TokenBudget` enforces a
  ~4-chars/token budget with truncation; `PromptCache` and `AuditLog` provide cache/audit
  bookkeeping; `build_explain_prompt`/`build_rename_prompt` assemble minimal context from
  the explanation (APIs, strings, L2 pseudo-code). New CLI: `recraft ai provider` shows
  resolved settings (never the key) and `recraft ai prompt` prints the redacted,
  budget-fitted prompt for review.

- 1 GiB performance gate: `scripts/bench-large.ps1` builds a 1 GiB overlay sample,
  measures import/analyze wall time and peak working set (polling the child process every
  20 ms, since PowerShell's Start-Process loses the values we need), and fails on
  DESIGN thresholds (import < 60 s, analyze < 5 s, peak <= 512 MiB). Wired into CI.
  Measured (debug): import 4.03 s, analyze 0.32 s, peak 5.6 MiB. Docs: `docs/PERF.md`.

- UI AI proposal panel: a new left-side "AI" tab lists rename proposals with confidence,
  rationale and evidence links; selecting one and pressing Apply sends it through the
  same journaled command bus (undoable), while nothing is ever applied implicitly.
  `suggest_names` preloads the string/xref tables once, so generating suggestions no
  longer scales quadratically with function count. Headless UI test renders the panel
  and applies a proposal end to end.

- M3 AI layer (offline half + write protocol): `recraft ai explain` builds a function
  summary from API tags, referenced strings and call edges; `recraft ai suggest`
  produces rename Proposals for `sub_*` functions with evidence addresses, confidence
  and rationale; `recraft ai triage` summarises API categories, weighted suspicious
  functions and URLs. `recraft ai apply <file> --yes` is the only mutation path and goes
  through the journaled/undoable command bus; without `--yes` it refuses. New
  `Session::all_api_tags` backs the aggregation. Docs: `docs/AI.md`, `docs/M3.md`.

- Pointer parameter types: a register argument used as a direct memory base is typed
  from the unique access width (`char*` for 1-byte, `wchar_t*` for 2, `int32*`/`int64*`
  for 4/8, `void*` for mixed, `int64` when never used as a base). New corpus probes
  (`str_probe`, `wide_probe`) validate `char*`/`wchar_t*`; L2 renders
  `int32 str_probe(char* arg1)`. Golden snapshot filenames are now symbol-based instead
  of index-prefixed, so inserting corpus functions no longer renames existing files.

- SSA-lite argument tracking: a may-write forward dataflow over the CFG (entry set
  empty, join = union of predecessor out-sets) seeds a second lifting pass, so `argN`
  is only printed for incoming values that are intact on every path; clobbered values
  at joins fall back to the concrete register view. Unit test covers the dataflow on a
  diamond with an argument write on one arm.

- Basic float typing: SSE scalar memory operands (`movss/movsd` and float arithmetic)
  are typed loads/stores (`f32`/`f64`) instead of raw `u32`/`u64`, float arithmetic uses
  dedicated `BinOp::FAdd/FSub/FMul/FDiv`, and int<->float conversions pick the typed
  source operand. 16-byte vector accesses print as `__m128`. Golden snapshot for
  `float_math` regenerated.

- L2 signatures: return type inference (`int64` when the function writes full `rax`,
  `int32` for `eax`-width results, `int64` for value-forwarding calls/tail calls,
  otherwise `void`) and calling-convention detection (`ret imm16` -> `__stdcall`,
  `ret` -> `__cdecl` on x86; Windows x64 fastcall stays implicit). New corpus stdcall
  probe validates both paths on x86. L2 output example: `int32 add_leaf(int64 arg1,
  int64 arg2)`.

- Semantic lifting batch 1: `push`/`pop` (stack-pointer side effects and the memory
  access, ordered so the printed expression sees the old `rsp`), `imul` (2/3-operand),
  `cmovcc` (new `Expr::Select` keeping lazy flags valid), and an SSE/float subset
  (XMM register file: `movd/movq/movdqa/movdqu/movaps/movups/movss/movsd`,
  `padd*/psub*/pmulld/pand/por/pxor`, `psrldq/pslldq`, scalar `addss/addsd/.../divsd`,
  `cvttss2si/cvtsi2ss`). Corpus semantic coverage 80.6% -> **96.2%** (v1 target 95%);
  CI coverage gate raised to 0.9. Golden snapshots regenerated.

- Quality gates (DESIGN 7.4): `recraft metrics` reports instruction semantic coverage
  (intrinsic fallbacks) and structured success rate, with `--min-structured` /
  `--min-coverage` turning it into a failing gate; `recraft golden` writes or checks
  normalised L1 snapshots (`0x...`/`loc_...` replaced by placeholders). Committed
  corpus snapshots under `tests/golden/corpus`; CI runs the corpus gate plus a golden
  roundtrip. Baseline: 24 functions, structured 87.5%, coverage 80.6% (documented gap to
  the 95% v1 target). Docs: `docs/QUALITY.md`.

- M2 MCP read-only server: `recraft serve-mcp <project>` speaks newline-delimited
  JSON-RPC 2.0 over stdio with the DESIGN read tools (`list_functions`,
  `get_disassembly`, `get_pseudocode` L0/L1/L2, `xrefs_to/from`, `list_strings`,
  `list_imports`, `callgraph`, `search_bytes`, `function_summary`) and project
  resources (`functions`, `strings`, `imports`, `pseudocode/{func}`). Write tools stay
  in M3 with the propose/apply protocol. New `Session::all_xrefs` backs the call graph.
  Docs: `docs/MCP.md`; E2E now includes an MCP stdio smoke step.

- M2 switch selector recovery: jump tables carry the traced selector register
  (`mov eax, ecx; ...; mov ecx, [base+rax*4+tbl]; jmp rcx` -> `switch (ecx)`), and L2
  renders it as `argN` when the selector is still the intact argument. Case bodies are
  still labelled regions. Tests: L1 selector and L2 argument selector.

- M2 L2 Win32 x86 argument recovery: with a frame pointer, `[ebp+8..]` stack slots become
  `arg1..`; without one, `[esp+4..]` do (until a `push`/`sub esp` makes offsets unstable,
  in which case the heuristic stays conservative). Stack stores never take argument
  names. `int64` remains a placeholder type. Tests: x86 add_leaf args recovered,
  shifted-stack helper falls back to `var_XX` slots.

- M2 L2 variable recovery (first cut): argument registers (Windows x64 rcx/rdx/r8/r9)
  are detected by read-before-write and rendered as `arg1..arg4` while intact; stack
  slots become `var_XX`; the signature lists the recovered parameters. Exposed as
  `recraft decompile --l2` and an "L2 (vars)" toggle in the UI pseudo-C tab. No SSA yet
  (cross-block clobber tracking and types are documented follow-ups).

- M2 switch structuring: the CFG now carries its jump tables (dispatch address ->
  targets in table order), and the L1 structurer emits `switch (index) { case N: goto
  loc_...; }` with case bodies appended as labelled regions. The selector register is
  still a placeholder (documented in `docs/M2.md`).

- M2 L1 structure recovery: a conservative structurer turns the CFG into `if`/`else`
  diamonds, early-exit `if`s and natural `while` loops using dominators/post-dominators
  and back-edge loop detection; unsupported shapes fall back to labelled `goto` regions
  without dropping blocks. Exposed as `recraft decompile --structured` and an
  "L1 (structured)" toggle in the UI pseudo-C tab. Docs: `docs/M2.md`.

- M1 background analysis: the UI Analyze action runs on a worker thread with its own
  SQLite connection; the top bar shows elapsed time and a Cancel button. Cancellation is
  cooperative (`AnalysisOptions { cancel }`, checked between functions and every 1024
  instructions) and returns `Error::Cancelled`; completed runs refresh every panel via
  the derived-data tables. Benchmarks (debug build): `kernel32.dll` 292 ms / 79,797
  instructions including strings; 1 GiB overlay sample analysis 317 ms (overlay is not
  mapped, so it is not scanned), import 3.88 s.

- M1 function boundaries: MSVC cold `.pdata` fragments (hot/cold splits) are merged back
  into their owning function when they are fallthrough-contiguous or targeted by a
  conditional branch; x64 corpus precision against the linker map goes 0.857 -> 1.000.
- Function-pointer table discovery: aligned pointers in readable data segments that land
  in executable memory become `DataPointer` seeds, recovering dispatch handlers that are
  never called directly (corpus recall 0.833 -> 1.000). `recraft eval --map` uses the
  MSVC `/MAP` file as ground truth for every function symbol, and the corpus build now
  emits `corpus.map`.

- M1 symbol demangling: MSVC C++ names (`?name@scope@@...`, constructors, overloads —
  parameter types kept, return type dropped) and Rust legacy/v0 names via
  `msvc-demangler` + `rustc-demangle` (legacy hash suffix stripped). Applied to export
  and import names at load time, so disassembly, symbol lookup and `funcs` show readable
  names. New `recraft demangle <name>` command and a CRT-free C++ corpus with namespaces,
  class methods and overloads.

- M1 Go runtime recovery: scans readable segments for the `pcHeader` magic (Go 1.16 /
  1.18 / 1.20+ layouts, including Go 1.26 where `textStart` was removed), validates
  candidates, then walks `functab` → `_func` → NUL-terminated `funcnametab` names.
  Recovered functions become high-priority seeds and their real names (`main.main`,
  `runtime.*`) are persisted as auto names. New Go corpus (built with `-gcflags=all=-l`):
  2,997 pclntab functions, 3,088 discovered, 147k instructions in 295 ms; `recraft funcs`
  now shows function names.

- M1 jump tables: CFG recognizes absolute pointer tables (`jmp [index*scale+table]`) and
  MSVC x64 image-relative tables (`lea base,[image_base]; mov dst32,[base+index*4+disp];
  add dst,base; jmp dst`), validating at least two in-function targets. Table targets
  become leaders and edges of a new `JumpTable` kind (purple in the Graph tab). Corpus
  gains `switcher_big` (23 blocks, 20 table edges).

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
