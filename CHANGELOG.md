# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- Decompiler structuring now duplicates terminal blocks that end in padding/external
  tail calls instead of leaving a jump to an unemitted label, drops straight-line code
  after an unconditional `return`, and folds jumps to return-only labels. Shared blocks
  with multiple predecessors get a label at their first emission, so no generated `goto`
  targets a missing label. When an earlier arm consumed the post-dominator join, short
  acyclic arms are now duplicated instead of falling back to a raw jump, and jumps to the
  last labelled short tail copy that tail plus the function epilogue. Tail scanning now
  recognizes `} else {` as a closing brace, and jumps to any label whose balanced tail
  ends in `return`/a tail call copy that tail too. Dead code after `return` now also drops
  orphan labels, exposing the rest of the unreachable tail. On the notepad corpus this
  raises structured functions from 432/502 (86.1%) to 465/502 (92.6%) while keeping
  instruction coverage at 99.4%; add regression tests for dead-tail, orphan-label,
  balanced-if and last-tail cleanup.

### Changed

- Removed the offline/Heuristic AI mode. `ProviderKind::Heuristic` and
  `HeuristicProvider` are gone; GUI AI actions (explain/suggest/query/polish) and the
  corresponding CLI commands now refuse to run until an LLM provider is configured. The
  provider settings panel only offers DeepSeek, OpenCode Go and a custom OpenAI-compatible
  endpoint, and unconfigured streaming returns a clear "no LLM provider configured" error.
- AI polish now streams OpenAI-compatible SSE responses: HTTPS providers emit
  `stream: true`, the parser handles `delta.content` (including chunks split across TCP
  reads and UTF-8 characters split across reads), and the AI result pane appends text live
  with auto-scroll. Local plain-HTTP transports and providers without SSE fall back to one
  final chunk. Other providers honor the configured completion budget; OpenCode Go polish
  disables thinking (`{"thinking":{"type":"disabled"}}`) and caps completion at 8k tokens,
  because the reasoning path delayed the first answer by 57–111 seconds. Reasoning-only
  OpenCode streams now produce an actionable error (reasoning character count +
  non-reasoning model suggestion) instead of a generic empty-stream failure. Repeated
  polish requests are cached per endpoint/model/budget; the AI header shows elapsed seconds
  and a first-token waiting status while a call is running.
- Decompiled call sites now show inferred arguments instead of bare `sub_xxx()`:
  callee prologues are scanned once per target (following import `jmp` thunks), argument
  setup writes captured by the call are elided, and calls without a recoverable signature
  fall back to the caller's computed slots. Consecutive duplicate writes, `int3`/`nop`
  padding, adjacent `goto`/label pairs and unreferenced labels are removed from L1/L2
  output; unused callee-saved spill pairs are hidden too.
  `call_sites_show_arguments_with_a_callee_prototype`,
  `tidy_labels_removes_adjacent_gotos_and_orphans` and the callee-save test cover the new
  behavior.
- The toolbar's Text/Graph pair is now a single target-mode toggle: in Graph mode it
  offers 文本/Text, in Text mode it offers 图形/Graph. The Space shortcut and the existing
  view behavior are unchanged. This removes the duplicate segmented control while keeping
  the mode switch discoverable.

### Fixed

- The Windows desktop binary is now linked with the GUI subsystem, so launching
  `recraft-ui.exe` no longer flashes/keeps a black console window. Unit tests keep the
  console so their output remains visible.
- Collapsing the AI sidebar leaves the active document untouched. The assistant is
  sidebar-only, so no dock tab is added or focused; regression test:
  `collapsing_ai_sidebar_keeps_the_active_document_tab`.
- Hex view now follows IDA's dump layout: 16-digit addresses without `0x`, tight
  two-digit bytes (uppercase), no per-byte offset header, and the ASCII tail directly after
  the bytes. Word mode keeps the same compact grid.
- Decompile view text now uses the active palette's foreground color instead of hard-coded
  `LIGHT_GRAY`, which was nearly invisible on the white light theme. Both the code text and
  the painter fallback follow `pal.text`; verified with a light-theme screenshot.
- L2 structuring no longer drops the rest of a loop body when several failure checks
  share one exit block. A shared exit is emitted as a labelled jump instead of being
  inlined at the first check. For nested loops whose inner exit leaves the parent loop
  (the crackme's `for`-style validation loop), the function now falls back to labelled
  flat output instead of putting the success path after the hint; a regression test
  covers `nested_loop_leaves_parent`.
- Polish now lets the model use canonical C runtime names when the call shape uniquely
  identifies them (`fgets`, `strcmp`, `strlen`, `strpbrk`, `printf`/`puts`, …), emits
  self-contained `extern` declarations for referenced tables, and strips Markdown code
  fences from provider replies. Previously it preserved every `sub_XXX` call even when the
  arguments made the CRT function obvious, so polished code looked less source-like.
- Polish now feeds the **authoritative disassembly** (addresses, bytes, mnemonics and
  branch targets) alongside L2 and instructs the model not to trust pseudo-code block
  order. This fixes outputs that hoisted the 32-round check outside the input loop or
  merged early exits with the success path. Re-running on the crackme now keeps the
  validation inside `while`, `wrong:` inside the loop and the hint after it.
- OpenCode Go / reasoning-model polish now works end-to-end: the prompt uses **L2** (locals,
  strings, call names) with an explicit source-reconstruction instruction, the polish
  budget defaults to **65536** tokens (reasoning consumes most of it before the answer;
  `deepseek-v4-pro` needs the full 64k), and an empty completion now reports an actionable
  "raise RECRAFT_AI_MAX_TOKENS" error.
  HTTPS errors include the response body (first 800 chars) instead of only the status code.
  Verified against a real OpenCode Go key with `deepseek-v4.1-flash`; the produced source is
  recorded in `docs/cases/crackme_win_x64.md` (section 4b).
- The AI panel is now vertically scrollable, shows the request status and polished
  result **above** the provider settings, and requests a repaint while a background
  polish job is running. Previously a slow/large response could be clipped below the
  panel or only appear after an unrelated mouse/keyboard event (reported as "Polish does
  nothing"). Adds `ai_job_result_updates_status_and_explanation`.

### Added

- Application icon: a geometric mint **R** inside a hexagon with circuit nodes, generated
  from `assets/generate_icon.py` (PNG + multi-size ICO + raw RGBA). The icon is embedded
  in `recraft-ui.exe` via the Windows SDK resource compiler, set as the eframe window icon,
  and used by the NSIS installer.
- Drag and drop a sample onto the window to open it: `.rcraft` projects load directly,
  PE files are imported to a sibling `.rcraft` project, and a centered drop overlay shows
  the accepted targets while dragging. A dropped file is processed before the next frame.
- AI is now a **sidebar-only** assistant: there is no AI dock tab. The right sidebar is
  shown by default, can be collapsed only via `×` or Options, and resized by dragging its
  inner edge (clamped to 300–620px) without ever turning into the button. When collapsed
  it leaves a **draggable floating AI pill** (rounded, shadowed, foreground overlay; does
  not consume layout width); drag it around and click to reopen. Its position is persisted
  in `layout.json`. `F10` and the AI menu entries open the sidebar. Provider settings live
  in the header ⚙ popup.
- Provider settings moved out of the inline panel into a **gear popup** in the AI header
  (also reachable from Options), so the main pane stays focused on actions/results.
- Redesigned the AI pane as a compact assistant dashboard: a title/status header, rounded
  action/result/query/suggestion cards, full-width readable result area with a copy button,
  a dedicated query row whose button stays visible, confidence badges and per-proposal
  evidence links, and a primary-styled Polish button. Provider settings remain a
  collapsible section at the bottom.
- UI localization migrated from inline `t(en, zh)` pairs to **Fluent**. `en-US.ftl` and
  `zh-CN.ftl` are embedded with `include_str!`; all 148 static UI pairs, menu bar, pane
  titles (including `Hex` → 十六进制), command-palette labels, graph legend/zoom controls
  and formatted status text now use stable message IDs, with `t_args` for formatted status.
  Tests enforce identical message IDs across locales and fallback behavior. The selected
  language is stored in `layout.json` and restored on startup.
- The AI settings API-key field has **Show/Hide** and **Copy** controls, so the saved
  (DPAPI-decrypted) key can be inspected or copied without editing `ai-settings.json`.
- AI provider settings now offer built-in **DeepSeek** and **OpenCode Go** modes plus
  `Offline` / `Custom`; selecting a mode fills its endpoint and model list, and the
  settings panel accepts an API key directly. The key is persisted **sealed for the current
  user** (Windows DPAPI; `api_key_sealed` in `ai-settings.json`) and prefilled on startup;
  persisted settings still store endpoint/model/key-env name. Provider requests
  send `User-Agent: recraft/<version>` and, for OpenCode Go, a stable
  `x-opencode-session` header.
- L2 call sites no longer print redundant argument-register assignments when the call
  already renders the captured value: `fgets((char*)(rsp + 0x20), 0x100, (void*)rax)` no
  longer repeats `rcx = …; edx = …;`. Register-valued arguments keep their assignment so
  semantics are unchanged; regression test updated (`recursive(rcx)`).
- AI polish plumbing is now smoke-tested end-to-end: `scripts/ai-polish-mock.ps1` starts a
  local OpenAI-compatible stub (`scripts/ai_mock_server.py`), configures
  `RECRAFT_AI_PROVIDER=openai`, and runs `recraft ai polish` on the crackme project
  (prompt → HTTP → cache → printed suggestion). The stub returns canned source, so it
  verifies transport/config, not model quality; real providers use the same path.
- Decompiler output is more source-like for string-heavy functions: address constants
  that name a stored string render as quoted C literals (including short `.rdata`
  literals such as `"\r\n"`), and a user prototype's function name is used at call sites
  (`printf`, `fgets`, `strcmp`, …) instead of `sub_*`. Adds `Session::string_at` and a
  C-escaping unit test.
- End-to-end CTF case study `docs/cases/crackme_win_x64.md` and the reproducer
  `scripts/case-crackme.ps1`: strings → xref → disassembly → L2 decompile → AI-polished
  source. The recovered flag is `flag{shuffl3d_x0r3d_4nd_sh1ft3d}`; the obvious
  `flag{strings_are_not_enough_you_know}` is a decoy rejected by `strcmp`.

- Navigating to a known string address now opens the Disassembly in **data view**: the
  string is rendered as `db '…',0` (or `dw` for UTF-16LE) with its resolved name and a
  `; DATA XREF: …` comment, and the following strings in the same region are listed as
  further data rows instead of being decoded as instructions. This matches IDA's
  Strings → double-click behaviour without showing garbage disassembly. Adds
  `string_address_renders_data_rows`.

- Strings view is now IDA-like: four columns **Address (segment:address) / Length (hex) /
  Type (C or C16) / String**, zebra rows and a persistent selection that follows the
  disassembly cursor. The full stored string is decoded according to its encoding
  (ASCII or UTF-16LE) instead of reading 64 bytes as lossy UTF-8, so wide strings no
  longer garble and long strings are no longer truncated. Single click selects and syncs
  the disassembly cursor, double-click navigates. A `Hide headers/section names` toggle
  (on by default) removes header/section-name noise while keeping the substring filter.
  Adds `strings_decode_encodings_and_filter_noise` and `docs/images/strings.png`.

- Default document tabs now follow the requested order: **Disassembly, Decompile, Hex,
  Strings, Local Types, Imports, Exports, AI**; **Names, Xrefs and Segments are hidden by
  default** and remain reachable from the Views menu. `Hex View-1` was renamed to **Hex**.
  The saved-layout version is bumped to 8 so existing layouts fall back to this new
  default; tests now assert the expected default tabs and the hidden panes.

- Renamed the two primary views to match their function: the merged view is now
  **Disassembly / 反汇编** (Graph and Text modes inside one tab) and the pseudocode pane is
  now **Decompile / 反编译**. Updated menus, tabs, welcome shortcuts (Space = Graph/Text),
  demo docs and screenshots (`docs/images/decompile.png` added). Pane ids are unchanged,
  only their display names.

- Single **Code View** pane (replaces the separate "IDA View-A" and "Text View" tabs):
  one dock tab with an internal **Graph/Text** mode stored as `view_mode`. The toolbar
  `Graph`/`Text` buttons and **Space** switch the mode of the same view instead of
  switching tabs; double-clicking a graph block switches the same view to Text and
  navigates. Navigation from lists/Hex focuses Code View in Text mode. Saved layouts now
  record one view plus the mode (`SavedLayout` v7); the pane enum is simplified to one
  `View` pane and the old Graph tab is gone.

- Multi-window evaluation (ADR-012): added `examples/viewport_spike.rs`, which opens a
  second **real OS window** via `Context::show_viewport_immediate` + `ViewportBuilder`.
  Verified on Windows 10 + the pinned Glow backend: two native top-level windows, child
  reports `ViewportClass::Immediate` (not the embedded fallback), and the detached window
  can be captured independently (`docs/images/detached-spike.png`). Decision: v0.1 keeps
  egui_dock's in-app floating tabs; promoting them to OS windows is scheduled after the
  preview with a concrete work list, while deferred `'static` viewports are out of scope
  for M5.

- Code-view zoom: **View → Zoom in / Zoom out / Reset zoom** (shortcuts `Ctrl+=`,
  `Ctrl+-`, `Ctrl+0`) scales the monospace font and row height of Text View, Pseudocode
  and Hex View (75 %–200 %) while leaving the surrounding chrome untouched; egui's global
  GUI zoom shortcut is disabled so the shortcut only affects code views. Columns follow
  the font size automatically, and the zoom factor is persisted with the layout
  (`SavedLayout` v6). Adds `zoom_actions_clamp_and_reset` and a persistence assertion;
  screenshot `docs/images/zoom.png`.

- Graph overview is now a real minimap: it reuses the graph view's CFG layout
  (`graph_layout`), draws scaled node rectangles with their edges, highlights the block
  containing the current address, and draws the graph view's visible viewport box when the
  graph pane has been laid out. Clicking a node jumps to that block. A unit test covers the
  shared layout geometry, and the screenshot is `docs/images/graph-overview.png`.

- Navigation band now reflects real content and matches the legend: executable segments
  are teal (instructions), readable data segments green, unexplored areas grey; discovered
  functions overlay the top half in the blue "Regular function" colour, import/IAT
  addresses appear as magenta "External symbol" ticks, the marker uses the current
  address-colour, and hovering shows address · segment · function. The legend reuses the
  same `NAV_*` constants, so band and legend can no longer drift apart.

- Text View operand syntax colours: registers (purple), numeric literals/immediates
  (teal), symbol names such as `sub_`/`loc_` (brown) and quoted strings (green) are
  coloured separately instead of the whole operand string using one colour; both themes
  have their own variants. Operand classification is unit-tested
  (`operand_tokens_are_classified_like_ida`). Screenshot: `docs/images/text-view.png`.

- IDA-layout step 4 (graph view + polish): **IDA View-A now opens in graph mode** like the
  reference (Text View is the text document tab). Graph nodes use IDA-style chrome — dark
  bodies, teal frames, light header bars with the block address and an `entry` marker —
  with per-instruction mnemonic/operand/comment colours; edges are curved, coloured by
  kind (fallthrough/jump teal, taken green, not-taken red, jump table violet) and end in
  arrowheads. Adds zoom controls (`Fit`, `-`, `+`, 50–200 %) in the graph toolbar and a
  graph status line (zoom/blocks/Hex sync); Output height is ~18 % like the reference and
  the Functions/Graph-overview split was retuned. Saved layout version bumped to 4.
  Screenshot refreshed at `docs/images/ida-layout.png`, demo/changelog updated.

- IDA-layout step 3 (disassembly rendering): drops the separate "Name" column and renders
  names as IDA-style inline labels (`start:`) on a dedicated label line above the
  instruction; columns are now Address / Bytes / Mnemonic / Operands / Comment with IDA
  colors (yellow addresses and labels, grey bytes, blue mnemonics, light operands, green
  comments), and the selected/current-row highlight (IDA blue) covers the label line as
  well. Rows keep the compact 15 px height while label lines add their own baseline.

- IDA-layout step 2 (theme + fonts): installs IDA-like typography on the first frame —
  Consolas for monospace text, Segoe UI for the UI and Microsoft YaHei as CJK fallback
  (system font files are optional; egui's defaults remain as fallback). Unifies the dark
  palette (panel `#1e1e1e`, view `#141414`, text `#d4d4d4`), IDA's blue current-line
  highlight (`#264f78`), subtler hover tint, compact 15 px code rows at 12.5 px Consolas,
  smaller UI text styles/button padding, and dock tab/chrome colors matching the new
  palette (including visible tab close buttons). Falls back cleanly when the font files are
  absent (e.g. CI), so headless tests keep passing (30 UI tests).

- IDA-layout step 1 (reference-screenshot alignment, not pixel-perfect): replaced the
  generic tool column with an IDA-style workspace — menu bar (File/Edit/Jump/Search/View/
  Options/Windows/Help), toolbar (back/forward, Text/Graph toggle, Analyze, AI, language,
  project path), global navigation band with an IDA-style legend, **Functions** window with
  Function name / Segment columns and function icons, **Graph overview** minimap below it,
  a document tab row (IDA View-A, Graph View, Hex View-1, Pseudocode, Imports, Exports,
  Strings, Names, Xrefs, Segments, Local Types, AI), an **Output** window with the status
  log, and an AU-style status bar with address/bytes/function/Hex-sync fields. Adds the
  Exports pane (PE exports), the view footer line, IDA-style window title
  (`ReCraft - <file>  <path>`), logs project-load/analysis messages to Output, and bumps the
  saved-layout version to 3. Screenshot at `docs/images/ida-layout.png`; demo walkthrough
  updated; 30 UI tests pass.

- Real docking, step 4 (IDA-like default layout and chrome): the default layout now
  mirrors IDA — tool column (Functions/…/Structures) on the left, Disassembly/Graph in the
  center, Pseudo-C docked on the right and Hex spanning the full width at the bottom;
  disassembly and pseudo-C panes scroll both axes when narrow, so columns are no longer
  cut off. egui_dock chrome is tuned toward IDA (dark tab bars, compact tabs, tinted active
  tab, subtle separators). Layout files now carry a version (`SavedLayout`, v2) so a stale
  layout falls back to the new default instead of resurrecting the old arrangement; the
  demo screenshot `docs/images/dock-step1.png` was refreshed with the new layout, and the
  layout round-trip test checks version rejection.

- Real docking, step 3 (persistence + focus shortcuts): dock layout is serialized with
  egui_dock's serde feature and auto-saved to `%LOCALAPPDATA%\ReCraft\layout.json`
  (split ratios, tab order, closed tabs and floating panes) after the layout settles,
  restored at startup; `Reset layout` clears the file and rebuilds the default. Saves are
  skipped until the dock has finite rectangles (egui serializes NaN as null). New pane
  focus shortcuts: `F2` Functions, `F3` Names, `F4` Strings, `F6` Imports, `F7` Xrefs,
  `F8` Segments, `F9` Hex, `F10` AI, `F11` Structures, with the bindings shown as tooltips
  in the Views menu. Tests cover the JSON round-trip (closed panes stay closed) and the
  `F2` focus shortcut (30 UI tests).

- Real docking, step 2 (navigation parity in the dock): every tool pane keeps
  double-click-to-follow — Functions/Strings/Imports/Xrefs/Names/Segments jump into the
  disassembly with the target line highlighted, Hex double-click and Graph block
  double-click now focus the Disassembly pane too, and navigation from any pane focuses
  the Disassembly dock tab (not just switches the logical view). `focus_pane` now also
  sets the focused node/surface so `Views` reliably activates the tab, re-docks closed
  panes, and the default layout is asserted to contain all 12 panes. Tests cover default
  layout completeness, navigation focus and re-opening a closed pane (28 UI tests).

- Real docking, step 1 (goal: IDA-style dockable workspace): added **egui_dock 0.21.1**
  (egui 0.36-compatible, MIT) and replaced the floating tool windows with a single
  dock-state workspace — center tabs (Disassembly/Pseudo-C/Graph), a left tool column
  (Functions/Strings/Imports/Xrefs/Names/Segments/AI/Structures) and a bottom-right Hex
  pane. Tabs can be dragged to split/merge panes, resized via separators and dragged out
  into floating windows; primary view tabs are not closeable while tool tabs are.
  `Views` menu focuses any pane (re-docking it if closed) and resets the layout; `Space`
  cycles the main views by focusing the corresponding dock tab. Layout state lives in
  `ReCraftApp` (`DockState<Pane>`), and a screenshot of the docked layout is checked in at
  `docs/images/dock-step1.png`. Tests updated for the pane/dock API (25 UI tests pass).

- IDA-style step 6 — data/type operations: `C` (code), `D` (data), `A` (ASCII data),
  `U` (undefine) and `P` (create function) on the current disassembly line, mirrored in
  the right-click menu together with remove-function, Edit prototype… and Structures…;
  all go through journaled commands (undoable) and refresh every view. New **Structures**
  window lists user layouts, defines/updates them from `0:tag:u32, 16:value:i32` field
  specs and binds a layout to the selected function's argument slot (reuses
  `DefineStruct`/`ApplyType`); prototype editing is available from the row menu via the
  existing edit dialog. `A` marks the range as data — string extraction remains derived,
  so it lists printable runs rather than storing a user string. Tests cover flag marks,
  struct definition/argument binding, prototype set/clear and the field parser.

- IDA-style step 5 — view linkage and appearance: the disassembly gains an IDA-style
  **navigation band** (segment map with a current-position marker, click to jump); the
  pseudo-C view highlights the line matching the current disassembly address, single-click
  syncs the disassembly cursor (`Action::SyncDisasm`, stays in pseudo-C) while
  double-click jumps to the disassembly tab; hex-byte clicks also sync the disassembly
  cursor; palette moved toward IDA colours (gold addresses, soft-blue mnemonics, pale-yellow
  names, green comments) and the status bar shows the selected name and enclosing function
  range. Test covers SyncDisasm semantics; demo walkthrough updated.

- IDA-style step 4 — detached tool windows: Functions/Strings/Imports/Xrefs/Names/Segments
  (and AI) are now independent movable/resizable/closable egui windows instead of tabs in
  one fixed left panel; the top bar gains a **Views** menu to toggle each window, `X` opens
  the Xrefs window, and every list supports double-click to jump into the disassembly with
  the target line highlighted (new **Names** window backed by `Session::names()`, filters
  for names and strings). Layout now gives the central disassembly/pseudo/graph view the
  full width; tests render all windows headlessly and cover the X shortcut.

- IDA-style step 1 — full-row disassembly interaction: the disassembly is now one
  full-width hit area per line (no grid cells), so clicking anywhere selects the row and
  the whole line is highlighted (IDA current-line colour plus hover tint); double-click
  anywhere follows the operand; right-click opens a context menu with Follow
  operand/Rename/Comment/Cross references/Copy address/Undo/Redo; the toolbar gains
  enabled ◀/▶ back/forward buttons (new `Action::Forward` and `forward()` reusing the
  existing history stack). Tests cover the back/forward round-trip; hint bar and demo
  walkthrough updated.

- Usability pass 2 — offline natural-language **Query box** in the AI panel: `urls`,
  `callers of <api/function>` and `text <substring>` intents reuse `answer_query`, results
  show kind/address/text/functions and clicking an item reverse-jumps into the
  disassembly; Clear resets the answer too. Keeps offline NL query available without CLI.

- Usability pass 2 — navigation enhancements: **Functions** side list gains a filter box
  (case-insensitive name/address substring) and shows resolved function names instead of
  bare ranges (hover keeps the exact range; double-click jumps to disassembly);
  **pseudo-C line click** now reverse-jumps into the disassembly view instead of
  re-centering the pseudo view; **Xrefs** list keeps showing the reference type
  (call/jump/data), gets a follow hint and double-click jumps to the referencing
  instruction in the disassembly tab. New tests cover the filter matcher, parallel
  function labels and the reverse-jump action.

- Usability pass 2 — decompiler visible-noise cleanup: suppress stack-pointer bookkeeping
  (`rsp = rsp ± N`) in pseudo-C, name `mov r11, rsp` frames as `frame`, fold algebraic
  noise (`xor reg, reg`, `and reg, 0`, `sub reg, reg`, `x ± 0`, shifts by 0), fix value
  casts that rendered as pointer casts (`(uint32*)(x)` → `(uint32)(x)`), and render tail
  calls as `return sub_…();` instead of a comment. Corpus golden snapshots regenerated
  (31/31 clean), notepad metrics unchanged (502/99.4%/86.1%); new golden noise-regression
  test plus docs/M2 notes on the remaining frame/var artifacts and the SSA/phi evaluation
  (deferred: cost outweighs preview benefit, keeping the SSA-lite register model).

- Usability pass 2 — AI panel usable without CLI setup: in-panel **Provider settings**
  (kind/endpoint/model/API-key env var/max tokens) persisted to
  `%LOCALAPPDATA%\ReCraft\ai-settings.json` (`ProviderSettings::load/save/load_from/save_to`;
  keys are never stored, `enabled` is recomputed from the environment), plus **Explain**
  (offline function summary/APIs/strings/calls rendered in the panel) and **Polish**
  (builds the polish prompt and calls the enabled provider on a background thread, so the
  UI never blocks). Clear resets explanation/status; docs AI/DEMO/README updated.

- Usability pass 2 — bottom **hex dump** panel (DESIGN §8 follow-up): follows the selected
  disassembly/pseudo address (16-byte rows, address gutter, ASCII column), byte/word
  display toggle (little-endian 16-bit words), click to select a byte/word, double-click
  to navigate the disassembly there, "Follow selection" button to re-anchor, and a top-bar
  **Hex** checkbox to show/hide it. Headless tests cover row alignment, selection sync and
  byte/word rendering.

- Usability pass 1 (post-preview feedback): a first-run **welcome panel** with quick
  actions (Open project / Import PE), persisted **recent projects** (up to 8, stored in
  `%LOCALAPPDATA%\ReCraft\recent.txt`, missing files pruned) and a shortcut cheat sheet;
  disassembly **follow** navigation via double-click or Enter (resolves the first mapped
  hex address / symbol in the operand text, falls back to a status hint when there is
  none, still recorded in the back history). Headless tests cover token extraction,
  recent dedupe/cap, follow navigation and welcome rendering.

- v0.1 previews ship **unsigned by decision** (code signing deferred): README gains an
  "Installing (unsigned preview builds)" section (SHA-256 verification, SmartScreen
  "More info → Run anyway", per-user installer), `docs/RELEASE.md` records the decision,
  free OSS paths (SignPath Foundation, Certum ≥1k-star policy) and the exact signing
  steps for when it resumes; M4/ROADMAP mark signing as deferred rather than blocking.

- NSIS installer generated and validated end to end (DESIGN §13): NSIS 3.13 was installed
  successfully in this session (`winget install NSIS.NSIS`), `scripts/package.ps1` now
  writes checksums after the installer so `SHA256SUMS.txt` covers `setup.exe`, and the
  installer writes `Publisher`/`DisplayIcon`/`InstallLocation`/`QuietUninstallString`/
  `NoModify`/`NoRepair` alongside the per-user install (no elevation). New
  `scripts/verify-installer.ps1` performs silent install into a temp directory, asserts
  files/Start-Menu shortcut/uninstall registry entry, runs the installed CLI (notepad:
  502 functions, 99.4% coverage), launches the GUI and checks its window, then silently
  uninstalls and verifies the directory/shortcut/registry are gone (with cleanup in a
  `finally`). Verified locally on Windows 10 both with a custom `/D=` directory and the
  default `%LOCALAPPDATA%\ReCraft`; `verify-package.ps1` also checks the installer hash
  and PE header, and CI gained an `installer` job (choco NSIS → -RequireInstaller →
  verify-package → verify-installer → artifact). ZIP is now 33 entries / 10.3 MiB,
  setup.exe 10.3 MiB.

- ADR records for every decision in DESIGN §16: `docs/adr/` now contains ADR-001 and
  ADR-003…ADR-011 (headless core, SQLite container/flags, user-vs-derived data, own
  decompiler ladder, no-IDAPython/own API, AGPL+commercial+CLA, MCP propose/apply,
  Windows-first/core-portable, dependency whitelist, 1 GB targets) in addition to the
  detailed ADR-002; docs/README indexes the directory and the M4 audit marks §16
  complete.

- Incremental range re-analysis (DESIGN §6.4): `AnalysisOptions.seed_range` restricts seed
  discovery to one range (the range start becomes an extra code seed) and
  `recraft_analysis::reanalyze_range` replaces only the auto functions/names/xrefs/API
  tags intersecting that range, leaving every other function byte-identical
  (`Session::replace_auto_*_in_range`). Exposed as `recraft reanalyze <project> --addr A
  --len N [--json]`; `mark` now triggers a range-scoped re-analysis automatically after a
  code/data change. Corpus unit test asserts no leakage outside the range and that
  unaffected functions keep their exact start/end; E2E covers the CLI. docs/M4 audit
  table and M2 gain the §6.4 evidence, ROADMAP drops the gap.

- License/notice compliance (DESIGN §12): added `NOTICE.md` (direct dependencies, SQLite
  public domain, Noto CJK OFL, sample policy), `scripts/check-license.ps1` (verifies
  required legal files, SPDX headers on all hand-written source files outside generated
  golden snapshots, and NOTICE coverage of direct dependencies) and wired it into the CI
  `licenses` job next to cargo-deny. docs/M4.md gains a DESIGN §0–§16 acceptance table
  with per-section evidence: v1 functionality is implemented; §6.4 function-level
  incremental re-analysis and the §8.4 i18n implementation choice remain documented
  deviations, while installer validation/code signing stay externally blocked.

- UI command palette (DESIGN §8.1 "menu / command palette"): `Ctrl+P` or `F1` opens a
  fuzzy-filterable list (open/import/analyze/undo/redo/view switch/goto/rename/comment/
  xrefs/AI suggestions/language) with arrow-key selection and Enter execution; the
  status-bar hint lists the shortcut in EN/zh-CN. Headless egui test drives the real
  `Ctrl+P` event (with `Event::ModifiersChanged`), filters to "rename", presses Enter and
  asserts the edit dialog opens.

- DESIGN §10 CLI convenience: read-only commands (`strings`, `imports`, `funcs`,
  `disasm`, `decompile`, `xrefs`, `cfg`, `metrics`, `golden`, `search`, `callgraph`)
  now accept a PE file path directly and auto-import+analyze it into a temporary
  project (SQLite projects are detected by their header and used as before);
  `recraft analyze <pe> --out project.rcraft` performs the one-shot import+analysis
  form from the design. E2E covers both paths; README documents them.

- UI keyboard flow completes DESIGN §8.2: `X` now opens the cross-reference panel for the
  current instruction (side tab `Xrefs` + refreshed xref list), joining the existing
  `Space` (view switch), `N` (rename), `;` (comment), `G` (goto), `Esc` (back) and
  `Ctrl+Z/Y` (undo/redo); the status-bar hint lists the new shortcut in EN/zh-CN. New
  headless egui test sends a real `Key::X` event and asserts the panel switch.

- AI natural-language queries and pseudo-code polish (DESIGN §9.2 completion):
  `recraft ai query <project> "<question>" [--json]` recognises offline intents
  (`urls`, `api_callers`, `function_callers`, `text`), answers from local tables
  (strings/xrefs/API tags/functions), extracts identifiers from free-form questions
  (`CreateFileW`, `Microsoft`) and supports zh-CN phrasing; `recraft ai polish <project>
  --addr <f>` sends the L1 output through the configured provider with `POLISH_SYSTEM`
  (semantics-preserving polish), prints the suggestion with a "not stored" banner,
  reuses the prompt cache and writes an audit record; offline it fails with a
  configuration hint. Adds `POLISH_SYSTEM`/`build_polish_prompt`, four query unit tests
  (URL/api-caller/unknown target/text), a polish-prompt test, E2E query assertions and
  docs/AI.md + ROADMAP updates (DESIGN §9.2 now fully covered).

- Release rebuild after the flag-semantics work: `package.ps1` (zip 10.2 MiB, 23
  entries) → `verify-package.ps1` PACKAGE PASS → `demo.ps1` DEMO PASS on the staged
  binary. The packaged exe reproduces the source metrics for notepad (502 functions,
  99.4% coverage, 432/502 = 86.1% structured) with 239 intrinsic instructions left;
  docs/RELEASE.md and docs/M4.md record the numbers.

- Flag-consuming instruction semantics: `bt` now records CF (= the tested bit) and the
  consuming branch rebuilds it as `((value >> bit) & 1) != 0` (only jb/jae are defined
  afterwards); `js`/`jns` become exact signed comparisons of the producing result with
  zero for `cmp`/`sub`/`test`/bitwise/`inc`/`dec`; `setcc` writes 0/1 from the tracked
  comparison without consuming it. Overflow/parity conditions now stay as assembly
  instead of printing a wrong placeholder. notepad: "unmodelled flags" branches drop
  from 270 to 23, semantic coverage rises 98.5% -> **99.4%** (239/37522 intrinsics) and
  structured rate 84.7% -> **86.1%** (432/502); corpus stays 100% structured / 97.4%
  coverage. Lift tests cover bt/jae, add/js and cmp/sete; docs/QUALITY, docs/M2,
  docs/M4 and CHANGELOG updated.

- Shared tail-chain duplication: when a branch targets an already-emitted straight-line
  chain (≤24 statements ending in `return`/tail call, no loop headers or switch
  dispatch), the structurer re-emits that chain inside the reaching arm instead of
  printing a `goto` to code emitted earlier. notepad structured rate rises from 81.5%
  to **84.7%** (425/502), residual `goto` lines drop from 1181 to 1088 and fallback
  functions from 93 to 77; coverage stays 98.5% (561/37522). corpus bytes_op golden
  snapshot now shows the duplicated early-return epilogue (semantics unchanged, reviewed);
  corpus remains 100% structured / 97.4% coverage. docs/QUALITY, docs/M2, docs/M4 updated.

- Function-range and structuring fixes for real samples: (1) analysis now clamps
  overlapping function ranges to the next known start (a merged `sub_*` range spanned
  dozens of real functions and made decompilation pick the wrong one), and (2) the
  structurer re-emits already-emitted *leaf* epilogues (≤12 statements ending in
  `return` or a tail call) inside the branch that reaches them instead of printing a
  `goto` to a shared epilogue. notepad: 502 non-overlapping functions, structured
  **81.5%** (409/502, up from 407), coverage **98.5%** (561/37522 intrinsics), residual
  `goto` count down to 1181; corpus stays 100% structured / 97.4% coverage with golden
  snapshots unchanged. docs/QUALITY, docs/M2, docs/M4 updated.

- Release rebuild after the CFG/semantic work: `package.ps1` (zip 10.2 MiB, 23 entries)
  → `verify-package.ps1` PACKAGE PASS (hash, required entries, packaged exe pipeline)
  → `demo.ps1` DEMO PASS on the staged binary. The packaged exe reproduces the source
  metrics for notepad (502 functions, 98.5% coverage, 81.1% structured); docs/RELEASE.md
  and docs/M4.md record the run.

- Indirect calls and IAT tail calls: `call qword ptr [IAT]`/`call reg` become
  `Stmt::CallIndirect` and print `(*{fnptr})()`; `jmp qword ptr [rip+disp]` is modelled
  as an indirect call followed by a return (tail call). Register/indexed indirect jumps
  stay untouched for jump-table detection. Physical effect on notepad: its 1571
  indirect-call intrinsics become zero, so semantic coverage rises from 94.2% to
  **98.5%** (571/37602 intrinsics) with structured rate unchanged at 81.1% (407/502);
  corpus stays 97.4% (10/391) with clean golden snapshots. New lift tests cover both
  indirect call forms; docs/QUALITY, docs/M2, docs/M4 and CHANGELOG record the numbers.

- Semantic coverage + honest metrics: unaligned/SSE moves (`movups`/`movdqu`/`movaps`/
  `movdqa` memory operands) now decode through `MemorySize::size()` (previously
  `Packed128_*` was unmapped and fell back to `__asm`), x86 rotates (`ror`/`rol`) are
  lifted and printed as `ror(a, b)`, and `metrics --json` gained an `intrinsic_top`
  histogram. `build_cfg` now drops blocks unreachable from the entry (alignment `int3`,
  jump-table bytes, gaps), and `measure` counts only CFG-reachable instructions: on
  notepad the reachable instruction count is 37602 with 2183 intrinsics, so semantic
  coverage is **94.2%** (was reported 84.9% with a linear sweep) and structured rate
  **81.1%** (407/502); `int3` intrinsics fell from 8547 to 33. corpus coverage is 97.4%
  (10/391) with golden `nested_loops` updated for the now-modeled `movdqa` load;
  docs/QUALITY, docs/M2, docs/M4 record the corrected metric definition.

- MCP HTTP transport in E2E: `scripts/e2e.ps1` now starts the real server in the
  background on a random port with a generated token, waits for `/health`, asserts a
  token-bearing `initialize` returns `serverInfo`, verifies a request without the token
  is rejected with 401, and always stops the process in a `finally` block. docs/MCP.md
  and docs/QUALITY.md record the coverage; complements the existing stdio step.

- Deterministic malformed-input regression for the PE loader (DESIGN §11 offline
  fuzz substitute): `crates/recraft-format/tests/malformed.rs` feeds 500 random blobs,
  400 truncations of notepad.exe plus DOS/NT boundary cuts, and 400 header/section-table
  bit-flip mutations; loading and every public accessor must never panic, and
  successfully loaded mutants are exercised through section lookup, RVA mapping,
  permissions, imports/exports/TLS/pdata. Runs in ~0.06 s as part of the workspace
  tests; cargo-fuzz remains a nightly-only optional extra. docs/QUALITY.md documents it.

- Release regression after the M2/MCP/CLI work: rebuilt `dist` (zip 10.2 MiB, 23 entries)
  and re-ran the full chain — `package.ps1` PASS, `verify-package.ps1` PACKAGE PASS
  (hash + entries + packaged exe info/import/analyze/decompile/AI/metrics, 502 functions),
  and `demo.ps1` on the staged release binary → DEMO PASS. docs/RELEASE.md and docs/M4.md
  record the run and the remaining installer blocker.

- CLI `batch` (DESIGN §10): `recraft batch <project> --script <file.json>
  [--dry-run] [--json]` executes a JSON list of edit commands through the journaled
  Command bus, in order, stopping at the first failure with its index/type. Supported
  commands: `rename`, `comment` (regular/repeatable/anterior), `prototype`,
  `define_struct` (object or `offset:name:kind` field strings) and `apply_type`
  (`struct` field avoids clashing with the `type` discriminator). Symbolic addresses
  resolve against both the project and earlier renames in the same script (so dry-run
  works without applying), a UTF-8 BOM is tolerated, and `--dry-run` validates without
  writing. E2E covers dry-run + apply + rename verification; README shows the command.

- MCP HTTP transport with bearer-token auth (DESIGN §9.1): `recraft serve-mcp <project>
  --http [--bind 127.0.0.1:8765] [--token <token>]` serves one JSON-RPC message per
  `POST /mcp` request, requires `Authorization: Bearer <token>` (401 otherwise),
  returns 202 for notifications, 400 for bad JSON, 404/405 for other paths/methods and
  exposes an unauthenticated `GET /health` probe; the token is generated and printed to
  stderr when omitted. Implementation uses std TCP with bounded headers/body and
  `Connection: close`. Unit test covers missing/wrong token, initialize, tools/call and
  health; CLI smoke verified 200/401. docs/MCP.md documents the transport.

- CLI `search` and `callgraph` (DESIGN §10 headless gaps): `recraft search <project>
  --bytes <hex>|--string <ascii> [--limit] [--json]` scans readable mapped segments
  (chunked) and labels hits with their containing function; `recraft callgraph <project>
  [--addr <filter>] [--limit] [--json]` prints function-level call edges built from CALL
  xrefs. Byte scanning moved into `Session::search_bytes` and the MCP `search_bytes`
  tool now delegates to it. E2E exercises both commands; README lists examples.

- Typed call arguments: when a callee has a user prototype, pointer parameters cast the
  rendered argument to the declared type (`str_probe((char*)rcx)` for
  `int str_probe(const char* text)`), while scalar parameters stay uncast for
  readability. New corpus probe `call_str` keeps a real direct call site (plus
  `noinline` on `str_probe`); corpus is now 31 functions, still 100% structured with
  97.2% coverage, golden snapshots regenerated from a pristine project (only the new
  call_str snapshot is added). Test asserts the pointer cast at the call site.

- x86 stack parameter recovery through prologue pushes: `recover_x86` now tracks the
  cumulative `esp` adjustment (push/pop/sub/add) and maps observed `[esp+N]` reads back
  to incoming argument offsets, inserting every observed offset for the same argument.
  Functions that save registers before reading their arguments now get real parameter
  names/types (e.g. `int32 __cdecl recursive(int32 n)`, `// L2 params: n=[esp+0x8]`)
  instead of empty signatures; unknown stack-pointer rewrites keep the conservative
  fallback. Test renamed/updated to assert two recovered arguments for a push-first
  function.

- x86 stack call arguments: the lifter records `push` values (newest push first) and
  appends them to `Stmt::Call` arguments, so cdecl/stdcall call sites render pushed
  arguments when the callee has a user prototype (`recursive(eax)` on Win32, x64
  arguments 5+ likewise); frame adjustments clear stale candidates and calls clear them
  after use. Tests: x86 callee-prototype argument rendering (plus the existing x64 IR
  call-capture test). Docs/M2 documents the remaining x86 prototype-signature caveat
  (automatic stack-slot recovery can be defeated by prologue pushes).

- Call-argument recovery + cross-function prototype propagation: the lifter tracks
  register values per block (`reg_env`) and records x64 call arguments on
  `Stmt::Call { args }`; when the callee has a user prototype, pseudocode renders the
  declared number of arguments (e.g. `recursive(rcx)` for `int recursive(int n)`) while
  calls without a prototype stay `name()` (golden snapshots unchanged). Self-referential
  argument expressions (`dec ecx` right before the call) fall back to the register name
  to keep semantics correct; x86 stack arguments are not rendered yet. Tests: IR call
  capture, decompiler prototype/argument rendering.

- Release artifact verification: `scripts/verify-package.ps1` checks the zip against
  SHA256SUMS.txt, asserts the required staged files (both exes, LICENSE, docs incl.
  FAQ/ROADMAP/M4/DEMO, VERSION.txt) and runs the **extracted** binaries through
  info/import/analyze/decompile/`ai provider`/metrics; wired into the CI package job.
  Rebuilt dist (10.1 MiB, 23 entries) and verified locally (502 functions).
  NSIS installer generation remains unverified: this session has no interactive UAC
  (`winget install NSIS.NSIS` and silent NSIS setup were cancelled at elevation), so the
  elevator-free green zip stays the official artifact; docs/RELEASE.md documents the
  exact VM steps for the installer.

- Complex loop headers (short-circuit guards at the top of a loop, where the header's
  condition does not cleanly split into stay/leave) now fall back to a structured
  `while (1) { <structured body> }` with break/continue carrying exits, instead of
  emitting gotos. corpus reaches **30/30 structured (100%)** with 97.2% coverage;
  notepad observation 78.7% (395/502). nested_loops golden regenerated (no goto);
  test asserts the fallback stays structured.

- Switch case inlining: jump tables whose targets are simple leaves (block ends in
  `return` or jumps to the switch join) now render as `switch (sel) { case N: { ... } }`
  instead of `case N: goto loc_X;` plus labelled bodies; grouped adjacent cases share one
  body and complex/shared case bodies keep the previous label/goto form. corpus
  `switcher_big` is now structured: 29/30 functions (96.7%) with coverage 97.2%; golden
  regenerated (30 snapshots); test asserts the inlined `case` form and no goto.

- do-while recognition: loops whose header is also the latch (single-block) now render
  as `do { ... } while (cond);` instead of `while (1) { ... if (cond) break; }`, and a
  latch-with-conditions-without-header-condition form is handled by a dedicated
  do-while path. Semantics are preserved (`while (1)` + `break` and `do..while` are
  equivalent here, verified in golden diffs). New corpus probe `count_bits`; corpus is
  now 30 functions / 93.3% structured / 97.2% coverage, notepad observation 77.7%
  (390/502). Golden snapshots regenerated (30); tests assert do-while output.

- Structurer break/continue: loop bodies now emit `break;` for branches that leave to
  the loop exit and `continue;` for branches back to the loop header (tracked via a loop
  context stack), and jumps to an enclosing region's join fall through instead of
  printing `goto` (while keeping explicit gotos for targets outside the loop body that
  are neither break nor continue). Notepad's structured rate rose from 62.4% to **77.3%**
  (388/502, above the v1 70% target) with 75 functions fixed and zero regressions when
  diffing the fallback set; corpus stays at 93.1% structured / 97.1% coverage. Golden
  snapshots unchanged; tests assert `break;` emission.

- M4 preview materials: README refreshed (current status, feature comparison, mermaid
  architecture, document index links), new docs/FAQ.md (ten questions incl. AI privacy,
  license and safety), docs/ROADMAP.md (milestone status + near-term priorities) and
  docs/README.md (documentation index). scripts/check-docs.ps1 validates every relative
  Markdown link (52 links across 18 files) and runs in CI. Also removed stray tracked
  MSVC linker artifacts (functions.obj/.exp/.lib) and ignored those extensions.

- Structurer quality: `test`/`cmp`-based signed conditional jumps (`jle/jg/jl/jge` and
  the matching `cmov`) are now modelled from the zero-only flag state, and if/else
  packaging accepts a join that already lies in the enclosing region's stop set, so
  multi-target early-exit chains and "guard then loop" shapes structure without `goto`;
  empty arms no longer emit empty `if` blocks. New corpus probe `early_chain`, golden
  snapshots regenerated (29 functions), corpus structured rate 93.1% / coverage 97.1%,
  and the notepad observation rose from 49.2% to 62.4% structured. Tests: lift-level
  signed-jcc, structure-level no-goto checks.

- Persistent AI response cache + audit log: `ai explain --llm` now consults
  `%LOCALAPPDATA%\ReCraft\ai-cache.json` (key = template + provider + model + prompt) and
  serves repeat explanations without network I/O (`cached=yes`; `--no-cache` bypasses
  reads), saving new responses best-effort. Every provider interaction appends a JSONL
  record to `ai-audit.jsonl` (ts/provider/model/template/function/tokens/cached).
  New `scripts/ai-cache-smoke.ps1` spins up a local mock Ollama over TCP and asserts
  miss → hit → two audit lines (wired into CI); unit tests cover cache roundtrip, key
  composition and audit appends.

- Prompt template versioning + offline prompt-quality baseline: `PROMPT_TEMPLATE_VERSION`
  (`rc-ai-2026-10-v1`) tags the exported system templates, and `eval.rs` defines
  `EVAL_VERSION` (`prompt-eval-2026-10-v1`) with five corpus cases that assemble real
  explanation prompts and assert they contain the function evidence (API/strings/
  pseudo-code), stay redacted (no raw URLs) and fit the token budget. Runs as
  `cargo test -p recraft-ai --test prompt_eval` and `recraft ai eval <project> [--json]`
  (5/5 passed; non-zero on failure). Model-answer quality scoring remains future work.

- HTTPS LLM transport: `UreqTransport` (ureq + rustls) serves `https://` endpoints while
  `TcpHttpTransport` keeps plain `http://` for local Ollama; `transport_for` picks by
  scheme and providers now use it. `TransportKind`/`transport_kind` are unit-tested and
  a network-ignored test verifies the TLS handshake against crates.io
  (`cargo test -p recraft-ai https_tls_handshake_to_crates_io -- --ignored`).
  OpenAI-compatible cloud endpoints no longer report "https unsupported".

- Demo package for the M4 preview: `scripts/demo.ps1` runs a nine-step CLI walkthrough
  (PE headers, import/analysis, function discovery, L2 pseudocode, rename+prototype,
  struct layout/apply_type, AI triage/propose/apply/undo on a real sample, MCP stdio
  handshake, quality metrics) and ends with `DEMO PASS`; `-Keep` leaves the project for
  the GUI. `docs/DEMO.md` adds the CLI script, GUI walkthrough, MCP client config, local
  Ollama setup and demo talk track; `docs/M4.md` tracks preview status.

- x86 prototypes now map declared parameters onto recovered stack slots in offset
  order (`[esp+N]`/`[ebp+N]`), including names and types, so a 32-bit function with
  `int __stdcall stdcall_add(int a, int b)` renders
  `int32 __stdcall stdcall_add(int32 a, int32 b)` with bodies using `a`/`b`.
  `ArgNames` gained `stack_types` (signature uses declared types, default int64) and
  `apply_prototype_x86`; new x86 corpus test.

- Array-subscript rendering: scaled addressing through pointer-typed arguments
  (`[rcx+rax*4]`, `[rcx+rax<<2]`, byte offsets on `char*`) prints as `arg1[index]`
  instead of raw pointer arithmetic; pointer inference now also learns element width
  from indexed accesses. New corpus probe `array_probe` (`p[i]` on `const int*`), new
  L2 test, golden snapshot (28 functions), metrics (89.3% structured, 96.5% coverage)
  and docs updated.

- Prototype declarations are now parsed and drive the decompiler signature: return
  type, `__cdecl`/`__stdcall`, parameter types (`int32`/`int64`/`char*`/`wchar_t*`/
  `int32*`/`int64*`/`void*`) and parameter names replace the inferred values on x64
  (up to four register arguments), and bodies use the declared names. Unknown or
  unsupported declarators (function pointers, arrays) fall back to the verbatim
  `// prototype:` comment. New `parse_prototype`/`ProtoSig`/`ProtoParam` API with
  parser tests and an override/undo decompiler test; E2E asserts the overridden
  signature.

- User struct layouts + `apply_type` (DESIGN §9.1): `recraft define-struct --name Probe
  --size 24 --field 0:tag:u32 --field 16:value:i32` stores a layout (schema v3:
  user_struct/user_struct_field/func_arg_type, idempotent migration), `recraft apply-type
  --addr <func> --slot 0 --type Probe` binds it to an argument slot, `recraft structs`
  lists layouts, and omitting `--type` clears the binding. All writes go through
  `Command::DefineStruct`/`ApplyType` (journaled, undo/redo, persistence; unknown types
  rejected). The decompiler renders named fields (`arg1->value`) instead of
  `arg1->field_10` for bound arguments. AI gains `ProposalKind::ApplyType`
  (value `"<slot>:<Type>"`). Tests: session undo/redo/persistence, decompiler
  named-field/clear, AI proposal apply; E2E defines/applies/undoes a struct.

- User prototypes (write tool, DESIGN §9.1): `recraft prototype <project> --addr <func>
  --text "int foo(int)"` stores a C declaration through `Command::SetPrototype`
  (journaled, undo/redo/persistence), and `recraft unprototype` deletes it; structured
  pseudocode shows it as `// prototype: ...` above the generated signature. Schema v2
  adds the `prototype` table (idempotent migration from v1). AI `ProposalKind::Prototype`
  can carry the same write through `ai apply`. New tests: command undo/redo/persistence,
  decompiler rendering, AI proposal apply; E2E sets/renders/undoes a prototype.

- Struct-field access rendering (M2 types): when a pointer-typed argument is dereferenced
  at a constant offset, pseudocode prints `arg1->field_10` (negative offsets
  `field_nXX`) for both integer and float loads/stores instead of
  `*(u32*)((arg1 + 0x10))`. New corpus probe `field_probe` (`p[4]` on `const int*`)
  exercises `[rcx+0x10]`; golden snapshots, metrics (27 functions, 88.9% structured,
  96.4% coverage) and docs updated.

- Packaging (M3): `scripts/package.ps1` builds release binaries, stages a green
  `dist/ReCraft-<version>` directory (exes, LICENSE/README/CHANGELOG, VERSION.txt, docs),
  produces `recraft-v<version>-windows-x64.zip`, writes `SHA256SUMS.txt`, and optionally
  builds the NSIS installer from `packaging/installer.nsi` when `makensis` is present.
  CI gained a `package` job that uploads the zip + checksums as an artifact.
  Docs: `docs/RELEASE.md`.

- LLM HTTP transport behind an injectable `Transport` trait: `TcpHttpTransport` speaks
  plain `http://` (enough for a local Ollama at 127.0.0.1:11434; `https://` reports a
  clear unsupported error until a TLS stack lands), `MockTransport` enables offline tests
  of request shape, auth headers, error statuses and response parsing.
  `OllamaProvider` (POST /api/chat, stream=false) and `OpenAiCompatibleProvider`
  (POST {endpoint}/v1/chat/completions with Bearer key) both return the model text;
  `provider_from_settings` keeps the heuristic provider unless the configuration is
  explicitly enabled. New `recraft ai explain --llm` sends the redacted,
  budget-fitted prompt and prints the answer (default config refuses with a clear error).

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
