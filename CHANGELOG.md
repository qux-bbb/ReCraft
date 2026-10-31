# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- The AI panel has a **Thinking mode** toggle. With it on, polish keeps the provider's
  reasoning pass and the streamed `reasoning_content` is shown in a collapsible
  “Thinking” section that expands while the model reasons and folds automatically when the
  answer completes. `Provider::complete_stream_with_reasoning` forwards reasoning deltas
  (the plain streaming entry point ignores them), and the UI receives them as
  `AiMessage::Reasoning`.

### Changed

- The whole Go sample now passes the output invariants: `recraft lint .tmp/go.rcraft --level l1`
  checks all **3261** functions and reports **0** findings (every `goto` labelled, labels defined
  once, braces balanced outside strings, no empty condition, signatures declaring what they use,
  no label above a signature). Together with Notepad (502 functions, L1+L2), crackme (493) and
  both corpora (63) that covers every sample in the workspace.
- `recraft lint --level l1|l2` checks one level instead of both, which halves the work on a
  large project (the 3261-function Go sample does not finish with both levels in a reasonable
  time; `--level l1` over its first 300 functions takes ~14 s). The summary line names the level
  that was checked.
- `recraft lint` takes the signature's parameter list from the **last** `(...)` of the header
  line, so a Go or C++ symbol carrying parentheses in its own name
  (`int64 internal/abi.(*RegArgs).IntRegArgAddr(int64 arg1, …)`) is parsed correctly instead of
  reporting `arg4 > 0`. The first 300 Go functions (600 texts) lint clean; the 33 "undeclared
  argument" findings it reported before were that parse artefact.
- **Fixed**: a jump table arm printed as `case 0: goto loc_140007014;` kept its `goto` but the
  target label was never emitted, so the output referenced an undefined label — 180 such jumps in
  crackme.exe (notepad was unaffected because its switches take a different shape). The jump
  target parser only recognised `goto loc_…` at the start of a line or after a condition, not
  after a `case` label; it now finds the keyword anywhere in the statement. `recraft lint`
  reports crackme clean again (493 functions, 986 texts, 0 findings) and Notepad is unchanged.
- `recraft lint --json` prints the report as one JSON object
  (`{"functions":…,"checked":…,"findings":…,"items":[…]}`) for CI and UI use; the exit code is
  the same as the text mode.
- `recraft lint <project> [--limit N]` checks the output invariants over every function and both
  levels, with a non-zero exit on a violation: every `goto` has its label, labels are defined
  once, braces balance outside string literals, no `if ()`/`while ()`, the signature declares
  every `argN` the body uses, and no `loc_` label sits above the signature. It reports a summary
  plus the first offenders. Corpus: 62 texts, no findings; Notepad (all 502 functions, 1004
  texts) reports none either.
- `merge_identical_if_arms` recounted the braces of every line while scanning a candidate block,
  which made the pass quadratic on deeply nested code. The per-line net brace counts are now
  computed once (and recomputed after the rare merge) — the arithmetic is unchanged, so the
  output is byte-identical: the 45 KB function's text passes went from ~262 ms to ~225 ms.
- Two more output invariants are now unit tests over the corpus: **every argument the body uses is
  declared in the signature** (a gap would print `arg5` without `arg4`) and **no `loc_` label is
  emitted above the signature line**. Both are checked for L2 (and the label rule for L1 as well),
  so the round-35 stack-argument widening and the round-41 label placement cannot regress
  silently.
- Golden snapshots handle the Go sample: `recraft golden` failed on it (`os error 123`) because
  Go names such as `internal/abi.(*RegArgs).IntRegArgAddr` and very long ones do not fit a file
  name. Names are sanitised and truncated to 80 characters (the dedupe pass still appends the
  address on collisions), and `recraft golden --limit N` writes/checks only the first N
  functions so a 3 261-function project gets a manageable sample. `tests/golden/go/` holds the
  first 40 Go snapshots and passes `--check`; the corpus and 32-bit snapshots are unchanged.
- A jump back to a function's first address used to label the header comment above the function
  (`loc_140001ba0:` printed before `// L2 structure recovery`), which also hid the loop the
  jump belonged to. Labels are now placed on the first statement of the body, after the header
  and any comment attached to that address, and comment lines never take a label. On the Go
  sample this recovered 24 functions: goto fallback functions 220 → **196**, labels
  1165 → **1059** (coverage 98.7% unchanged). Notepad, crackme and both corpora are unchanged.
- `fold_sibling_tail_gotos` looked its target label up with a full scan of the function for every
  candidate jump; it now builds a label index once and shifts it when a splice moves the tail
  (splices are rare). The 45 KB function's text passes went from ~313 ms to ~265 ms with
  byte-identical output across all 502 functions.
- A stack slot that only ever receives one argument is that argument, not a local: a slot that
  is written exactly once by `var_40 = arg4;` and never has its address taken has its reads
  renamed to `arg4` and the spill removed (`qword_140034e40 = arg4;` instead of
  `var_40 = arg1; … = var_40;`). Notepad: 72 of 88 such spills folded across 53 functions, L2
  lines 30 174 → 30 102. Slots written more than once, or whose address is passed to a call,
  keep their names.
- The L2 header shows the stack frame the prologue reserves:
  `// L2 structure recovery (11 blocks, frame 0x48)` — 101 of Notepad's 502 functions have a
  constant-size, push-free prologue and now report it, so a reader sees how much local state a
  function keeps without walking its prologue. Two stack parameters are pointer-typed as well
  (`int64* arg5`, `void* arg7`).
- An x64 stack parameter is typed by the access the pseudo-C makes through its value: when the
  slot is copied to a register and that register is used as a memory base, the parameter
  becomes `char*`/`wchar_t*`/`int32*`/`int64*` (`void*` when the widths disagree) instead of
  `int64`. Notepad's stack arguments are all plain values, so the output is unchanged there;
  a unit test covers a slot that is written through as a 64-bit pointer.
- 32-bit stack arguments are typed by how they are used: when the loaded value becomes the base
  of another memory access the parameter is a pointer, and the access widths pick
  `char*`/`wchar_t*`/`int32*` (`void*` when they disagree). The x86 corpus now reads
  `str_probe(char* arg1)`, `wide_probe(wchar_t* arg1)`, `array_probe(int32* arg1, int32 arg2)`
  and `field_probe(int32* arg1)`, matching what the x64 path infers from register width.
- An x86 function that re-reads an argument after the prologue moved `esp` used to get two
  parameters with the same name (`nested_loops(int32 arg1, int32 arg1)`); parameters are now
  deduplicated by name, giving `nested_loops(int32 arg1)` as on x64.
- 32-bit stack parameters are typed by their slot width: an x86 argument whose own type is
  unknown is `int32`, not `int64`, so `int32 __cdecl add_leaf(int32 arg1, int32 arg2)` replaces
  `(int64 arg1, int64 arg2)` across the x86 corpus (32 signatures). A user prototype still
  overrides it. The 32-bit corpus also gets committed snapshots (`tests/golden/corpus86/`,
  32 files) so the x86 path has the same regression coverage as the x64 one;
  `recraft golden <project> --dir tests/golden/corpus86 --check` passes.
- An x64 function that reads its caller's stack arguments shows them as parameters: when the
  prologue is `sub rsp, const` (no pushes), a read at `frame + 0x28` and beyond is `arg5`,
  `arg6`, … instead of a `var_` slot, the signature is widened to cover them and the parameter
  comment says `arg5+=stack`. Notepad: 9 functions, e.g. a tail call that used to forward
  `var_80, var_88, var_90` now forwards `arg5, arg6, arg7`. Slots the function writes stay
  slots (they are its own outgoing arguments), and functions whose prologue pushes registers
  are left alone because their frame offsets are not relative to the value the scan finds.
- The text passes are no faster than they need to be on the largest function: `hide_callee_saves`
  scanned every line once per saved slot (quadratic) and `propagate_argument_copies` rewrote
  every line for each argument copy. The first now builds a slot → line index in one pass, the
  second skips lines that do not mention the register. The 45 KB window procedure went from
  **~1.6 s to ~1.3 s** per decompile, of which the text passes dropped from ~800 ms to
  ~310 ms; the emitted text is byte-identical. `RECRAFT_PROFILE=1 recraft decompile …` prints
  the phase timings (lift, analysis, emit, text passes) for future work.
- `recraft metrics` reports two more counters from the same L1 pass: `labels` (the `loc_`
  definitions the structurer had to emit) and `inline asm statements` (the `__asm("…")`
  lines left in the pseudo-C). Notepad: 267 labels and 138 inline asm across 502 functions;
  the corpus reports 0 and 9. Both were previously counted by hand in these notes, so the
  numbers in `docs/QUALITY.md` can now be reproduced with one command.
- A constant loaded into a register for one comparison moves into it: `eax = 0x111; …
  if (arg2 > eax)` reads `if (arg2 > 0x111)`, and the assignment is dropped when the register
  is written again before any other read (so the value cannot be live). Notepad: 35 functions,
  L2 lines 30 208 → 30 174; the message dispatch of the window procedure now reads
  `if (arg2 > 0x111) { if (ebp > 0x233) {`, and the security-cookie check reads
  `if (rax == 0x2b992ddfa232)`. A comparison between two known constants keeps its register
  form — evaluating it would print a constant condition.
- The handle table covers more APIs: the registry functions name their key slot `hKey`
  (`RegOpenKeyExW`/`RegQueryValueExW`/`RegSetValueExW`/`RegEnumKeyExW` now match
  `RegCloseKey`), `DestroyIcon`/`DestroyCursor` give `hIcon`/`hCursor`,
  `UnhookWindowsHookEx` gives `hHook`, and the window APIs (`ShowWindow`, `SendMessageW`,
  `PostMessageW`, `SetWindowPos`, `InvalidateRect`, `RedrawWindow`, `SetWindowTextW`,
  `GetWindowLongPtrW`) name their first argument `hwnd`. Notepad: 137 named handle uses
  (from 120), e.g. `RegCreateKeyExW(…, &hKey, …)` and `SendMessageW(hwnd, …)`.
- Address constants now name what they point at: `rdx = 0x14002b210;` reads
  `rdx = off_14002b210;` for an address inside a data segment, a constant equal to a function
  start becomes that function (`return sub_140001c64(sub_140027590);`), and a constant that
  matches a string literal becomes the string. Numbers that match none of these stay as they
  are (`rbx = 0x2b992ddfa232;`). Notepad: 358 `off_` names and 5 function pointers across 168
  functions, bare address assignments 511 → 337 (the rest are image-base/code addresses and
  plain constants). The golden normaliser masks the `off_` digits too, and `dispatch.c` was
  regenerated.
- Global data reads as a named word instead of a cast expression: `*(u64*)(0x140036648)`
  becomes `qword_140036648`, `*(u32*)(0x140034460)` `dword_140034460`, and `u8`/`u16`/`f32`/
  `f64`/`__m128` give `byte_`/`word_`/`flt_`/`dbl_`/`xmmword_` (the width stays visible, and a
  computed address such as `*(u32*)(0x140034460 + 4)` keeps its cast). Notepad: **1725**
  global reads across 189 functions, no absolute-address cast left; `qword_140036648 = rax;`
  reads like the global store it is. The golden normaliser masks the digits
  (`qword_ADDR`) so snapshots stay address-independent; three corpus snapshots regenerated
  (`dword_ADDR`, `flt_ADDR`).
- A pointer argument that type inference left as `void*` now shows field accesses when the
  offset is unambiguous: `*(u32*)((arg1 + 0x10))` reads `arg1->field_10`, and a bare
  `*(u8*)(arg1)` reads `arg1->field_0`. The pass only rewrites an offset whose accesses all
  use the same width — a field name cannot say which width it means, so `arg2 + 0x18` read as
  both `u32` and `u8` keeps its casts. Notepad: `argN->field_XX` accesses 266 → **777**
  (+511, 97 functions), raw `*(…*)((argN + …))` casts 1418 → **18**, lines unchanged at
  30 208.
- A slot that a handle API receives gets a handle name: `CloseHandle(var_88)` reads
  `CloseHandle(hObject)`, `RegCloseKey(var_38)` `RegCloseKey(hKey)`, `ReleaseMutex`
  `hMutex`, `SetEvent`/`ResetEvent` `hEvent`, `DestroyWindow` `hwnd`, `WindowsDeleteString`
  `hstring`, `FreeLibrary` `hModule`, `FindClose` `hFind`, `DestroyMenu` `hMenu`,
  `ImageList_Destroy` `himl`, and `ReleaseDC`'s second argument `hdc`. A slot that several
  APIs receive takes the most telling kind (a mutex handle stays `hMutex` even when it is
  later closed). Notepad: 120 named handle uses across 9 functions. Slots only ever freed
  with `CoTaskMemFree` keep their number: a generic `pBuffer` was not clearly better.
- A structure that an API fills gets a real name: `GetClientRect(hwnd, &var_30)` turns the
  slot into `rect`, so the code reads
  `rect = 0; GetClientRect(hwnd, &rect); edx = rect.bottom; edx -= rect.top;` instead of
  `var_30`/`var_34`/`var_38`/`var_3c`. `MSG` slots become `msg` and `POINT` slots `pt`; a name
  already used by the function gets a numeric suffix (`rect2`). Notepad: 8 functions,
  54 named accesses. A small step toward variable naming, driven by the same known-structure
  table.
- Statements that do nothing are dropped: `rax = rax;` (and a slot self assignment), an `if`
  whose body is empty, and a label that nothing jumps to when it only introduces the closing
  brace. Notepad: 3 sites, L2 lines 30 212 → **30 208**.
- The analysis pass no longer formats the statements it discards. `lift_block` gained a
  `render` flag: the first pass only needs the block's effects (writes, calls, stack use,
  argument copies), so it skips ~37 000 statement renderings for Notepad's biggest function.
  The emitted text is byte-identical; wall-clock change is within run-to-run noise on this
  machine (1.6 s either way for the 45 KB function), so treat it as work removed rather than
  a measured speedup.
- A direct call that passes an argument register by name now shows the value that was put
  there. `edx = 0; sub_1400029dc(&var_20, rdx, …)` printed the register even though the value
  was known; it now reads `sub_1400029dc(&var_20, 0, …)`. Only plain values (constants, stack
  slots, `&var_XX` addresses) are substituted — a load or an arithmetic expression would be
  evaluated twice in the argument list, so those keep the register name. Notepad: 175 call
  arguments improved across 77 functions, lines unchanged. Conditional expressions stay
  named temporaries in the adjacent-temporary fold (`r9 = ((al != 0) ? rax : 0);` reads
  better than inlining it), and the `RECT` API list grew to the other rectangle
  producers/consumers (`SetRect`, `InflateRect`, `OffsetRect`, `CopyRect`, `InvalidateRect`,
  `PtInRect`, `GetUpdateRect`, `AdjustWindowRectEx`).
- A function that never returns a value now says so: `int64 sub_140001440(void) { … return; }`
  became `void sub_140001440(void)`, because every exit is a bare `return;`. A body that
  ends with a call keeps its integer type — `f(); ret` may be `return f();` in the source, and
  the register-width signature is the honest reading there. Notepad: 29 signatures corrected,
  L2 text otherwise unchanged; the sibling-arity fill (below) also learned to read `ApiName(…)`
  call sites, though notepad's remaining empty calls are all zero-argument APIs from the
  signature table.
- A direct call whose arguments were set up in another block prints its arity. Such a call
  records no arguments, so it printed `sub_X()` even when a sibling call to the same target
  in the same function shows three (`sub_X(a, b, c)`); the widest sibling now supplies the
  count and the site prints that many register placeholders. Calls with more than four
  arguments are left alone (a stack argument's position cannot be placed from register
  names). Notepad: empty direct calls 426 → **291**, 49 functions affected, lines unchanged.
  `POINT` also joins the known-structure table (`GetCursorPos`/`ClientToScreen`/
  `ScreenToClient` → `var_XX.x`/`.y`); notepad has no such call, so that part has no
  notepad delta but covers GUI binaries that do.
- A register temporary that is written once and read once by the very next statement is
  inlined into it, the way a source-level local would be:
  `eax = *(u64*)(rbx); sub_140001000(eax);` becomes
  `sub_140001000(*(u64*)(rbx));`, and `ecx = var_a0; var_50 = ecx;` becomes
  `var_50 = var_a0;`. Only register temporaries are folded — a stack slot is a variable, so
  its address (`&var_30`) is never replaced by the value that was stored into it, and an
  expression with a top-level operator is parenthesised on the way in. Notepad: L2 lines
  30 256 → **30 212**, 41 functions affected, `goto` count and coverage unchanged; one
  corpus golden (`array_probe`) updated for the new text.
- Well-known imports now print the right number of arguments. The decompiler had no parameter
  count for IAT calls, so a call whose registers were set up in an earlier block printed
  `CoTaskMemFree()`, while a zero-argument API with a leftover register printed a phantom
  argument (`GetCurrentThreadId((ecx + 1))`). A table of parameter counts for 66 common Win32
  APIs bounds the list: the call prints exactly that many slots, filling the unresolved ones
  with the ABI register name, and a zero-argument API prints no arguments at all. Notepad:
  `CoTaskMemFree(rcx)`, `WaitForSingleObject(rcx, 0)`, `CloseHandle(*(u64*)((rcx + 8)))`,
  `GetCurrentThreadId()` (false argument gone); empty-argument API calls 305 → **~256**,
  lines and gotos unchanged (30 256 / 452). An unknown import keeps the old
  register-setup rule.
- `MSG` joins `RECT` in the known-structure table. `GetMessageW`/`PeekMessageW`/
  `TranslateMessage`/`DispatchMessageW` take a `MSG*`, so later member accesses print
  `var_40.hwnd`, `var_40.message`, `var_40.wParam`, `var_40.lParam`, `var_40.time`,
  `var_40.pt.x`/`.pt.y` instead of the bare slots (`if (var_40.message == 0x50)`,
  `return var_40.wParam;`). The rewriter is now idempotent — a second message-loop call in
  the same function no longer re-prefixes names that are already member paths
  (`var_40.hwnd.wParam`). Notepad: 3 message fields across 1 function, 30 struct field
  accesses in total, lines and gotos unchanged.
- A `RECT` filled by `GetClientRect`/`GetWindowRect` now has field names. Those two APIs
  take a `RECT*`, so the four 32-bit slots at the passed `&var_XX` are the structure's
  fields; later reads and stores print `var_30.left`, `var_30.top`, `var_30.right`,
  `var_30.bottom` instead of `var_30`, `var_34`, `var_38`, `var_3c`. The address (`&var_30`)
  and any pointer base (`(var_3c + 8)`) keep the plain name, as does a wide clear before the
  call (`var_30 = xmm0;`), so only real member accesses are renamed. Notepad: 27 field
  accesses across 6 functions (`edi = var_30.bottom; edi -= var_30.top;`), lines unchanged.
  This is the first step of structure-aware naming; `POINT`, `MSG` and friends can follow
  the same table.
- Longer forward skip regions are folded into `if` blocks. `fold_short_forward_gotos` only
  handled guards up to 24 lines, so large dispatch/cleanup diamonds stayed as
  `if (c) goto L; … goto M; loc_L:`; the limit is now 64 (the pass still rejects regions
  with unbalanced braces or bound `break`/`continue`). Notepad: `goto loc_` 475 → **452**
  (the 45 KB function 391 → 369, e.g. a `GetProcAddress` loop body now sits inside
  `if (r15d != 0) { … }` instead of after a jump), L2 lines 30 254 → 30 256 (+2 braces),
  structured functions and coverage unchanged.
- Carried call arguments now survive across blocks even when they name a register. The
  cross-block value analysis used to keep only register-free values (constants, strings,
  stack slots), so `CloseHandle()` stayed empty when the handle came from
  `rcx = *(u64*)(rbx)`. Each carried value now remembers the registers it reads and the
  dataflow drops it only when one of them is overwritten between the definition and the
  call, which is exactly when the printed text would be stale. Notepad: empty Win32 API
  calls 337 → **305** (e.g. `CloseHandle(*(u64*)(arg1))`,
  `ReleaseSemaphore(arg1, 1, &var_48)`, `SetThreadpoolTimer(*(u64*)(rsi), &var_30, 0, 0x124f8)`),
  empty `sub_` calls 437 → 426, L2 lines unchanged at 30 254.
- Tail calls keep their arguments. A function ending in `rcx = 0x140027590; return sub_140001c64();`
  used to drop the value because the structured tail-call emitter only printed the target;
  the setup lines immediately above the jump are now consumed in ABI order (bounded by the
  callee's recovered arity, so an unknown callee only takes `rcx`) and printed as the call's
  arguments. Notepad: empty tail calls 74 → **63**, 48 tail calls now show arguments
  (`return sub_140023d8c(*(u64*)(0x140036648))`), L2 lines 30 265 → 30 254.
- The structurer's no-condition loop is printed as `while (1) { ... }` instead of
  `do { ... } while (1);`. Both are infinite loops, but the `while` form is what a compiler
  emits and what source code reads; the rewrite also visits nested loops (a `do` inside a
  converted one used to be skipped). Notepad: 12 `do { } while (1);` → 0, `while (1)`
  loops 48 → 60, lines and `goto` count unchanged (30 265 / 475). `matching_brace` now
  ignores braces inside string literals, so a format string containing `{`/`}` no longer
  hides a loop body from the text passes.
- A frame-pointer setup that nothing reads is no longer printed. `rbp = &var_100;` exists
  only so slot names can be resolved to the entry frame; when the register is never named
  again the line is noise, so the final pass drops it (any later read keeps it). Notepad:
  7 such lines across 7 functions (`sub_14000e950` and the other WndProc helpers start
  directly with their first real statement), L2 lines 30 272 → **30 265**; unit test covers
  both the dropped and the kept case.
- The pseudo-C header names the level that produced it: the decompile view used to claim
  "L1 structure recovery" even for the L2 signature view. The first comment is now
  `// L1 …` or `// L2 …` depending on the entry point; L1 text and every golden snapshot are
  unchanged. The analyses added this session also got unit tests: the argument-alias
  dataflow (a copy holds only when every path agrees, and an overwrite in one arm clears it)
  and the stack layout of a register-based frame restore (`lea r11, [rsp+0x50]; mov rsp,
  r11`).
- Epilogues that restore the frame pointer through a register no longer lose the stack
  displacement. A tail such as `lea r11, [rsp+0x50]; mov rsp, r11; pop r14; pop rdi; pop rbp`
  used to make the whole block's `rsp` layout unknown, so every slot in the function fell
  back to a raw offset (`var_n48`); a register already known to hold a stack address now
  resolves the update, and the printer restores the displacement through the same value.
  Notepad: `var_nXXXX` names 47 → **18**, two single-block functions change
  (`sub_14001f420` now reads `var_20`/`var_24` instead of `var_n48`/`var_n44`), L2 lines
  unchanged at 30 272, `goto loc_` unchanged at 475.
- Frames built through the saved-`rsp` alias keep their slot names. A prologue such as
  `mov rax, rsp; push ...; lea rbp, [rax-0x148]` was not resolved: naming a register from a
  stack address only understood `rsp` itself, so `[rbp-0x30]` printed as `var_n30` instead of
  `var_d0` while other accesses to the same slot used the normalised name. The printer now
  resolves an address from any register already known to hold one (`rsp - 8`,
  `rax - 0x148`), so the frame base propagates through aliases. Notepad: `var_nXXXX` names
  115 → **47**, 10 functions change, L2 lines 30 275 → 30 272, `goto loc_` unchanged at 475.
- Stack slots keep their frame base in functions that allocate the frame with `sub rsp, rax`.
  MSVC loads the frame size into `eax`, calls `__chkstk` (which preserves `rax`) and subtracts
  `rax`; the lifter treated `rax` as clobbered by the call, so the stack displacement of the
  whole function became unknown and every slot name fell back to its raw offset
  (`var_n1008` instead of `var_c0`). The constant is now carried across the probe call, so
  `sub rsp, <size>` is a normal step: notepad `var_nXXXX` names drop **2090 → 115** (the
  45 KB function 1891 → 1, its frame reads `rbp = &var_100`), six functions change, L2 lines
  30 277 → 30 275, `goto loc_` unchanged at 475.
- Calls whose argument setup was split across a block boundary now print their arguments.
  The printer's argument-register values are carried with the same must-analysis as the
  aliases (intersection over the predecessors, cleared by calls), so a call in a later block
  prints what an earlier block loaded. Only values that name no live register are carried —
  constants, string literals and stack slots — because a stored string like
  `*(u64*)((rcx + 8))` would read the register's *new* value in another block. Notepad:
  empty Win32 API calls 433 → **337** (41 functions), e.g. `ReleaseSRWLockExclusive()` now
  prints its lock argument, L2 lines 30 276 → 30 277, `goto loc_` unchanged at 475.
- Pointer-typed arguments now spell field accesses in every block, not just in the block
  that copies them. `r14 = arg4` followed by `mov ecx, [r14+0xC]` used to print
  `*(u32*)((r14 + 0xc))` because the alias was only tracked inside one basic block; the
  printer now gets a must-alias map computed over the CFG (`entry_alias_map`: a register
  carries a parameter to a block only when every path agrees — intersections of the
  predecessors' maps, with the block's own copies applied). Field spelling is only used when
  the access width equals the pointer's element size, so a `u8` load through a `void*`
  keeps its explicit cast. Notepad: `argN->field_XX` accesses 137 → **266** across 121
  functions (`sub_14000e950` now reads `arg4->field_c` / `field_4` / `field_8` instead of
  raw casts), L2 lines 30 272 → 30 276, `goto loc_` unchanged at 475, corpus golden 31/31.
- MSVC's `__chkstk` stack probe is no longer printed. A frame larger than one page is
  allocated by loading the size into `eax` and calling the probe helper, which touches the
  guard pages before the real prologue; it has no source-level effect, so the call and its
  size load are dropped. The helper is identified by shape (`sub rsp, imm` followed within
  six instructions by `mov [rsp], r10`/`mov [rsp+8], r11`), not by address, and the call
  site must be a bare no-argument call. Notepad: the six functions with a 4 KB+ frame
  (`sub_140013154`, `sub_140007570`, …) lose their `eax = 0x10c8; sub_140027440();` opening
  (notepad L2 30 291 → 30 272 lines).
- A function is no longer emitted as flat labels as soon as one inner loop leaves its parent
  loop. Only the offending top-level loop is emitted as labelled blocks (raw `loc_`/`goto`
  for its body, every edge preserved) and the rest of the function keeps its `while`/`if`
  structure. The validator functions still get the labels they need, but the surrounding
  code no longer loses its shape: the 45 KB notepad function drops from 687 `goto loc_`
  lines to **402** (10070 → 10080 lines), and notepad's total falls from 771 to **475**.
  The Go sample gains one structured function (1779 → 1780/2000); corpus golden and the
  structured rates rise to 487/502 (97.0%) and 453/493 (91.9%).
- Decompiling the largest notepad function got **8.5× faster**: the post-processing passes
  used to rescan the whole function for every candidate (token counts for the single-use
  temporary fold, label-reference counts for the label tidy pass). Token frequencies are
  now collected once and maintained as folds happen, jump targets are collected in a
  single pass, and the IAT name map is built once per function instead of once per block.
  The 45 KB / 1 620-block function drops from 12.2 s to **1.4 s** (debug build) while every
  one of the 502 notepad functions renders byte-identical L2 output — the change is a pure
  refactor.
- Labelled fallback text now recovers structure from its own jumps. A label whose region
  ends by jumping back to it becomes `do { ... } while (cond);` (an unconditional
  back-edge becomes `while (1)`), and a forward `if (cond) goto L;` whose skipped region
  is short (≤ 24 lines) and brace-balanced becomes `if (!cond) { ... }`. Both rewrites move
  no statement: labels stay valid (C labels are function scoped), jumps into the region
  still work, and a loop region that uses `break`/`continue` keeps its jumps so the
  binding to the enclosing loop cannot change. Longer skips keep the jump — wrapping
  hundreds of lines would indent the rest of the function past the screen. Notepad drops
  from 1 235 to **771** `goto loc_` lines (242 recovered `do` loops); the structured rate
  goes 486 → 487/502 (97.0%), the crackme case 450 → 452/493 (91.7%), and the Go sample
  1 767 → 1 779/2 000 (88.9%).
- The frame pointer established by the prologue (`rbp = rsp - N`) now names `[rbp + disp]`
  in every block. A block that never reloads rbp used to name the same slot from the raw
  rbp offset while the prologue named it from `rsp`, so one location appeared as two names
  (`var_18` in the prologue, `var_n10` in a branch — and `*(u64*)((rbp + 0x200))` raw in
  L1). `lift_block` now reports the stack-address registers it sees, the entry block
  supplies the frame pointer, and every block starts from that base: 58 notepad functions
  get one consistent name per slot (`var_48`/`var_40` instead of `var_18`/`var_n10`), and
  no raw frame expression remains in L1 either. Corpus golden is unchanged.
- MSVC `/GS` stack-cookie bookkeeping is no longer printed. The prologue stores  `__security_cookie ^ rsp` in a frame slot and every return path folds it back and calls
  `__security_check_cookie`; both are compiler-generated, and printing the call as the
  function's result (`return sub_1400019f0(rcx);`) hid the value actually returned. The
  check sites now read `return 0;` (and the real value is folded from `eax`), and the
  setup is dropped once no check reads its slot. Recognition is by shape — a global mixed
  with `rsp`, stored in a slot — so the API name is not assumed, and a function without
  such a setup keeps its calls. Notepad: 126 check sites in 44 functions, 155 → 2
  occurrences (the remaining two are the checker itself and one unresolved call), and the
  frame-cookie loads/folds 74 → 7. L1 output (which does not seed the register-rebuild set)
  keeps the check's load as a raw stack expression (`*(u64*)((rbp + 0x200))`) instead of a
  `var_XX` name; the matcher accepts both forms. Before this, L1 dropped the setup but kept
  the checks, leaving a comparison against a slot nothing wrote — L1 hides the same checks
  as L2 now (2 occurrences left).
- A call split from its argument setup by a block boundary now prints its arguments.
  Blocks end at a jump target, so `mov rcx, [global]` and the `call` it feeds can land in
  different blocks (and the call block is emitted with fresh value tracking); the arguments
  are recovered from the straight-line run above the call, stopping at labels, braces and
  calls that clobber the argument registers. Only a callee with a known non-empty arity is
  filled from the whole run; any other direct `sub_...` call is filled only when its first
  argument was written on the line directly above, so a stray register value cannot become
  an argument of an argument-less call. Corpus `recursive` reads `recursive(ecx)` instead
  of `recursive()`; notepad gains 99 calls (`sub_14000b320(eax)` from the `ecx = eax;`
  line above, …). This also fills calls duplicated as a shared tail, since each copy is
  rewritten from its own preceding lines.
- A `goto` to a label whose block is just the return path folds into `return <value>;`:
  `if (eax == 0) goto loc_140013a2d;` reads `if (eax == 0) return 0;` once the target's
  `eax = 0; return;` is the whole block (including labels shared by several exits). The
  label disappears when no jump references it; 29 jumps fold in notepad.
- Cross references are IDA's `xrefs to …` **dialog** instead of a docked tab: `X` (only —
  `F7` is retired) opens a window titled after the selected item, with `Direction`
  (`Up`/`Down`), `Type` (`c` call, `j` jump, `o` offset, `r` read, `w` write), `Address`
  (`main:loc_1400010b9`) and `Text` columns. Each row shows the instruction at the other
  end, with the referenced string appended as `lea rdx, [0x…]  ; 'flag{…}'`. `Down` rows
  are new: selecting an instruction also lists what it references. `OK`/Enter follows the
  selected row, `Cancel`/Esc closes, `Search` filters the columns and `Help` shows the
  type-letter legend; arrow keys move the cursor and the footer reports `Line N of M`.
- The frame-alias line is no longer printed: `mov rbp, rsp` (or `mov r11, rsp` in an exception
  frame) only sets up the alias used to name frame-relative slots, and the alias is never read
  back in pseudo-C. The bookkeeping still runs, so `var_nXX`/`frame_XX` naming is unchanged;
  96 lines in notepad.
- A loop-invariant setup at the top of a `while (1)` body hoists above the loop, so the
  sentinel test becomes a real condition: `while (1) { rax = &var_30; if (rcx <= rax) break;
  ... }` now reads `rax = &var_30; while (rcx > rax) { ... }`. The setup is only moved when
  no call, assembly or write to that register appears in the body.
- A register write whose register is never mentioned again in the whole function is dropped
  (`rbp = &var_b1;` in a stub, setup registers from an inlined call). All spellings count;
  `rax`/`eax` is kept because L1's bare `return;` still returns it, argument registers
  (`rcx`/`rdx`/`r8`/`r9`) are kept because a call prints only the arguments it recovered,
  and a function containing assembly is skipped since `rep movsb`-style text reads registers
  without naming them. About 40 lines in notepad.
- A temporary that only feeds one store folds into it:
  `ecx = (uint32)(*(u16*)(rax)); *(u32*)((r8 + 8)) = ecx;` becomes
  `*(u32*)((r8 + 8)) = (uint32)(*(u16*)(rax));`, and the same holds for a named stack slot
  (`ecx = 0; var_8 = ecx;` becomes `var_8 = 0;`). The copy stays when the register is still
  live afterwards, and calls/branches count as reads.
- A temporary built only for the return folds away: `ebx = 0x80004002; return ebx;` reads
  `return 0x80004002;`, and `rdx = (c ? a : b); return rdx;` reads `return (c ? a : b);`
  (29 sites in notepad).
- A compiled `bool` return (`al = 1; return eax;`) folds into `return 1;`: the constant moves
  onto the returning line when it is immediately followed by the `eax` return. Non-constant
  byte values keep their explicit write so the width stays visible. 33 such returns in
  notepad, 6 left that read a byte register.
- Ordinal-only imports (COMCTL32 `#345`, shcore-path `#170`, ...) now resolve to a call
  name too: `(**(u64*)(0x14002a420))()` reads `ord_170()`. Notepad's unresolved
  through-IAT pointer calls drop 10 → 2.
- A register write whose next mention in the same straight-line run is a full 32/64-bit
  redefinition is dropped: `rax = var_b8; r9d = 0; r8d = 0; rax = var_b0;` loses the first
  load. Calls, branches, labels and assembly stop the scan, a redefinition that reads the
  register does not count, and an 8/16-bit write (`al = 0`) leaves the wider value alive.
  The corpus `start` snapshot loses a dead spill and a dead argument copy; notepad's listing
  drops 76 such lines.
- A conditional branch does not modify the flags, so a block that starts with another test
  after its predecessor's branch re-lifts that predecessor's pure `cmp`/`test`/`bt` in its
  own expression arena and prints a real condition instead of
  `__asm("if (je ... /* unmodelled flags */) goto ...")`. `lock cmpxchg` also records the
  comparison it performs, so the retry test after a compare-and-swap reads
  `if (arg1->field_2c != eax) continue;`. Notepad's unmodelled branches drop 44 → 21 and the
  intrinsic count 172 → 170; one large function now needs a `goto` for a shared tail
  (structured 487 → 486).
- `sbb x, x` is lifted as the carry mask it is (`x = cond ? -1 : 0`) instead of assembly, and
  its usual consumers fold into one statement: `x = -(x)` becomes the boolean
  `x = (cond ? 1 : 0)` and `x &= y` the conditional mask `x = (cond ? y : 0)`. The carry
  flag is derived from the right producer: a `cmp`/`sub` gives an unsigned borrow, a `neg`
  gives "operand was non-zero" (`jb`/`jae` after a `neg` now read as `!= 0`/`== 0`), and a
  `bt` gives the tested bit. Notepad's 54 `sbb` assembly lines disappear and the intrinsic
  count drops 219 → 172 (coverage 99.5%).
- Calls with more than four arguments now print the stack-passed ones: stores into the
  outgoing argument area (`[rsp+0x20]`, `[rsp+0x28]`, ...) are collected until the call and
  appended in slot order, resolving copies to the value they carry. A format-string call
  reads `sub_1400083f8(rbx, 0x25, "%08lX-%04X-...", var_2f, (uint32)(var_33), ...)` and
  `RtlVirtualUnwind` shows all eight arguments; notepad has 149 such calls. Only a call that
  already shows its four register arguments is extended, so an unknown arity never places a
  stack slot in the wrong position.
- Slot names now follow stack-pointer changes inside a block as well: the displacement is
  updated before each statement is printed, so a `push`/`sub rsp` early in a block no longer
  leaves later accesses named as if the stack pointer had not moved (`&var_98` → `&var_50` in
  notepad's unwind helper).
- Scalar floating-point parameters and returns are typed: `xmm0`..`xmm3` read before being
  written become `f32`/`f64` parameters (the memory operand's width distinguishes `float`
  from `double`), and a function that leaves its result in `xmm0` without writing `eax` is
  declared `f32`/`f64`. Corpus `float_math` now reads `int32 float_math(f32 arg1)` with
  `arg1=xmm0` instead of an argument-less signature. Mixed integer/float parameter order is
  not recoverable without a prototype, so those `xmmN` registers stay unnamed — except for
  user prototypes, which map float parameters to `xmm0`..`xmm3` and keep their declared
  order in the signature (`float scale(float factor, int count)`).
- A register used only to carry a value into a store is folded into it:
  `rcx = &var_58; var_30 = rcx;` becomes `var_30 = &var_58;`. The copy is only removed when
  the register is redefined later in the same straight-line run, so a value a call or branch
  could still read keeps its round-trip. Notepad's listing loses 220 such lines.
- Stack-slot names are now one space per function: every block's `rsp` displacement is
  propagated over the control-flow graph and slots are named relative to the bottom of the
  frame, so the prologue, the body and every epilogue agree on what `var_XX` means. A
  register that holds a stack address (a saved `rsp` copy, a rebuilt `rbp`) carries the same
  base, so `[rbp-0x30]` and `[rsp+0x18]` naming the same slot print the same name, and a
  register saved at entry and restored at exit is recognised even when the two accesses sit
  at different stack depths. A block reached with two different displacements (an `alloca`,
  a dynamic adjustment) keeps the previous names.
- Matched `push`/`pop` pairs are treated as bookkeeping and left out of the listing: a
  register saved at entry and restored on the way out no longer shows as
  `var_n8 = rbx;` / `rbx = ...;`. Pairs are matched per register in LIFO order and only
  when every push has a pop, so a `push` that feeds a call's stack arguments stays visible.
  A lone prologue spill whose restore is not in the listing is hidden as well; after the
  first branch or call the same store may be a stack argument, so it is kept.
- A captured argument value survives a stack store between its setup and the call: a store
  to a `var_XX` slot cannot change memory reached through a pointer argument, so it no
  longer blocks the setup removal. Notepad's argument-setup lines drop from 4060 to 3949.
- Taking the address of a local now reads as such: `rax = (rsp + 0x30);` and
  `rcx = (rsp + 0x68);` render as `rax = &var_30;` / `rcx = &var_68;`, so a pointer passed to
  an out-parameter is visible as `RtlLookupFunctionEntry(rdi, &var_50, 0)`. The frame setup
  keeps `frame = rsp`. Notepad's listing converts 348 such assignments.
- A bare frame-slot access is named like every other local: `*(u64*)(rsp)` and
  `*(u64*)(rbp)` render as `var_0` (the `push`/`[rsp]` slot) instead of an untyped
  dereference. Notepad's listing loses 56 such dereferences.
- `bts`/`btr`/`btc` are lifted instead of kept as assembly: `bts ebx, 0x1c` renders as
  `ebx |= 0x10000000;` and `btr edx, 0x1c` as `edx &= 0xefffffff;` (a constant bit index is
  folded into the mask, and the old-bit flag definition is still available to a later
  `jb`/`jae`). `cdqe` renders as `rax = (int64)(eax);`. Notepad's listing loses 29 assembly
  lines (intrinsic instructions 244 → 219).
- Argument recovery no longer counts a zeroing idiom as an input: `xor r8d, r8d` at the top
  of a function used to look like a read of `r8` and invented a parameter, and pointer
  evidence was credited to whichever register happened to be the memory base even after it
  was reassigned (`bytes_op` typed its `int len` as `int32*`). Reads now ignore
  self-cancelling forms, and a pointer type follows the address back to the argument that
  originated it, through register copies (`r10 = arg1`, `rdx = r10`). On notepad the
  four-parameter signatures fall from 104 to 62 (most of those carried a phantom `r8`/`r9`),
  and corpus `bytes_op` now reads `int32 bytes_op(void* arg1, int32 arg2)` like its
  two-parameter source.
- Scalar parameters are typed from the width the body reads them with: an argument only ever
  read through its 32-bit half now renders as `int32` instead of `int64` (memory-based
  pointer inference still wins), so `count_bits(int32 arg1)` and
  `array_probe(int32* arg1, int32 arg2)` match their sources. Notepad gains at least one
  `int32` parameter in 166 of 502 signatures.
- The Decompile view no longer has the `L1 (structured)` / `L2 (vars)` checkboxes. It always
  renders the structured pseudo-C with the recovered signature, argument names and stack
  slots (the previous L2 output), and falls back to the raw listing for an address outside
  the analysed functions. The two locale keys for the removed toggles are gone.
- One-line conditionals lost the generator's wrapping parentheses: `if ((eax == 0)) break;`
  now reads `if (eax == 0) break;`, and the same for `continue`, `return ...` and
  `goto loc_...` (`if`/`while`/`do-while` block headers already did this). Notepad's listing
  drops 1065 such pairs.
- Argument-register setup consumed by a call is no longer printed twice. A direct call
  already renders the callee's parameters, and an indirect import call (`call [IAT]`) now
  prints the values last written to `rcx`/`rdx`/`r8`/`r9`, so the preceding copies are
  dropped when none of their inputs change before the call (and no store invalidates a
  memory read); self-cancelling forms such as `xor edx, edx` count as constants. On notepad
  the listing loses 2525 setup lines (6585 → 4060), e.g. `GetProcessHeap(); HeapFree(rax, 0,
  rbx);` instead of two extra register copies.
- `else` arms whose only statement is a nested `if` now render as `} else if (cond) {`, with
  the arm body moved one level left, so an else-if ladder reads like the source instead of
  gaining a level of nesting per branch. Notepad's listing merges 126 such arms.
- `while (1)` loops whose only exit is one `if (condition) break;` now show that test as
  the loop condition: at the top of the body it becomes `while (!condition)`, at the end it
  becomes `do { ... } while (!condition)`. On notepad 47 of 83 unconditional loops become
  real conditions, e.g. a table scan reads `while (rbx < rdi) { ... }`. A `continue` in the
  body keeps the loop unchanged, because it would reach the latch before the test.
- Signatures now follow arguments forwarded through direct calls. A call is treated as
  consuming the callee's argument registers (the callee's own signature is recovered
  depth-first through up to four wrapper levels, memoized, with one function-range query per
  decompile), so corpus `call_str` renders `int32 call_str(int64 arg1)` with
  `str_probe(arg1)` instead of `void` + `str_probe(rcx)`, and notepad wrappers like
  `sub_1400030e8` show the four registers they pass through. Unmodelled instructions report
  the registers they write, so `cpuid` and `sbb edx, edx` no longer invent parameters: on
  notepad, 40 of 502 signatures change — 33 gain the registers they forward, 2 drop such
  phantom arguments, the rest get a more precise pointer type.
- Jump-table functions no longer show the compiler's dispatch twice: the table-address
  loads and the trailing `__asm("jmp rcx")` that precede a recovered `switch` are dropped
  (the statements are replaced by the `switch` itself, and the body of each case is now a
  folded `return <value>;`). The cleanup only fires when that block consists solely of
  machine-register arithmetic ending in the indirect jump, so a call or a memory store
  sharing the block stays visible.
- Decompiled pseudo-C now returns the value the code computed instead of leaving it in a
  register: `eax = f(); return;` renders as `return f();`, `eax += x; return;` as
  `return (eax + x);`, and a value-returning path whose producer is not the adjacent
  statement names the return register (`return eax;` / `return rax;`) rather than a bare
  `return;`. A single-use temporary written just above the return is inlined into it, so
  `eax = a; eax += b; return;` becomes `return (a + b);`. Void functions keep the bare
  `return;`, and the multiply-by-one that scaled addressing leaves behind
  (`(arg2 * 1)`) is dropped. On the corpus, `add_leaf` now reads
  `return (arg1 + arg2);`.
- In the Decompile view, double-clicking a callee's name now enters that function (its
  pseudo-C replaces the listing and `Back` returns to the caller) instead of jumping to the
  disassembly row under the pointer. Callee names are resolved with the decompiler's own
  naming rules (stored name or `sub_<hex>`), a callee tooltip and pointing-hand cursor
  advertise the click, and double-clicks on anything else (locals, keywords, the current
  function's own name, the line-number gutter) still jump to the disassembly. Renaming a
  function now refreshes the Functions labels immediately, so the new name resolves without
  re-analysis.
- The Decompile view now shows IDA-style line numbers in a fixed left gutter instead of an
  address prefix; the gutter stays put while the code scrolls horizontally. Clicking a row
  selects that exact line and syncs the disassembly cursor; the listing is no longer
  re-anchored/re-rendered on click, and an address group that used to highlight several
  rows at once now highlights only the clicked row (or its first row when the selection
  comes from another view). Hovering a row shows its address.

### Fixed

- Double-clicking a string literal in the Decompile view now opens the string data it names
  (`db '…',0` in the disassembly) instead of the instruction that loads it. The pseudo-C
  renderer records every quoted literal with the address it quotes, so the view can follow
  the literal itself; identical texts at several addresses are disambiguated by the xref
  from the clicked instruction. The data view also handles literals that point inside a
  longer stored run (compiler string pooling: `"===="` inside an `"O====…"` run) and short
  literals below the string-extraction minimum (`"\r\n"`, `")"`), which fall back to the
  quoted literal recorded by the decompiler.
- Redundant parentheses around a condition are stripped in single-line branches too:
  `if ((al == 0)) return;` now reads `if (al == 0) return;`. The pass matches the closing
  parenthesis structurally instead of assuming a `{` terminator, and a genuinely needed pair
  (`if ((a) && (b))`) is left alone.
- A call argument that is a bare register no longer turns into the address of a stack slot:
  `helper(rbp)` used to print `helper(&var_0)` once `rbp` had been written, because a bare
  `[rbp]` access and the pointer value were treated alike. Only a real stack-slot address
  gets the `&var_XX` form now.
- A stack store that feeds a call (arguments 5 and beyond live on the stack) is no longer
  dropped as a dead callee-save spill or dead local: `RtlVirtualUnwind`'s outgoing
  `var_20`/`var_28`/`var_30` slots are kept, and a saved register is only treated as dead
  when no call can read its slot. Slots reached through a saved `rsp` copy are named
  `frame_XX` instead of `var_XX`, so they no longer collide with the current frame's locals.
- The saved-`rsp` alias is retired when a call clobbers its register, so a later
  `test rax, rax` reads `if (rax != 0)` instead of the bogus `if (frame != 0)`.
- A load-modify-store fold no longer drops the loaded value when a later statement still
  reads its register, most visibly a counter update that returns the new value: corpus
  `corpus_counter_bump` used to render `*(u32*)(...) += 1; return;`, losing the returned
  counter, and now keeps the load and ends in `return eax;`.
- A `while (1)` loop emitted for a loop header that carries statements (the structurer's
  fallback when the test cannot sit on a `do-while` latch) broke on the condition that
  *stays* in the loop instead of its negation, so the printed loop left when the machine
  continued and continued when it left. The fallback now negates the fallthrough condition:
  notepad's `cmp rdx, r9; je exit` reads `if (rdx == r9) break;` instead of
  `if (rdx != r9) break;`.
- `do { ... } while (...)` conditions no longer keep the generator's extra parentheses:
  `while ((ecx != 0));` renders as `while (ecx != 0);`, the same cleanup `if`/`while`
  already had.
- Branches that follow an arithmetic instruction now test the value that instruction
  produced. `sub ecx, 1; jne` used to print `ecx != 1` (the pre-write operand, subtracting
  a second time when read), and `shr ecx, 1; jne` reused the flags of an older `add`, so a
  loop condition could compare the wrong registers and the printed loop need not terminate
  the way the machine does. The lifter records the destination register as the flag source
  for `add`/`sub`/`and`/`or`/`xor`/`inc`/`dec`/`shl`/`shr`/`sar`/`neg`: `sub ecx, 1; jne`
  now reads `ecx != 0`, `shr ecx, 1; jne` reads `ecx != 0`, and corpus `count_bits` ends in
  `while (ecx != 0)` matching its source. Unsigned conditions after arithmetic (which need
  CF and cannot be expressed against the result) fall back to the existing "unmodelled
  flags" assembly instead of printing a wrong condition.
- The disassembly view can now be scrolled up past its loaded window. The pane renders a fixed
  window of 400 instructions, so its scroll area could only move inside that window: at the top
  edge the wheel did nothing and earlier code was unreachable. When the wheel keeps turning
  against the top/bottom edge the view now loads another page (50 instructions) above/below and
  compensates the scroll offset by the pixel height of the rows it added, so the text on screen
  stays put. Paging with `PageUp`/`PageDown`/`ArrowUp`/`ArrowDown` also stepped by
  `rows * first row length` bytes, which landed in the middle of variable-length instructions
  and decoded garbage (and could walk below the image and blank the view); stepping now uses
  real instruction boundaries via the new `Session::rewind`/`Session::advance` (swept from the
  nearest analysed function start, `int3` padding included), keeps the cursor line and stops at
  the segment start. `G`, follow, back/forward and hex-tab sync now reset the scroll position so
  a jumped-to address is actually visible.
- Keyboard shortcuts and paging no longer fire twice on frames where egui runs a second layout
  pass (`request_discard`): one key press used to scroll the disassembly by two pages.
- The built-in DeepSeek provider preset now offers the live model ids `deepseek-flash` and
  `deepseek-v4-pro`. The previous `deepseek-chat` / `deepseek-reasoner` ids were retired on
  2026-07-24 and no longer resolve, so the preset defaulted to a model the API rejects.
  Saved settings still pointing at a retired id are migrated to `deepseek-flash` on load, and
  the docs list the new ids.
- AI polish (and other AI actions) no longer wait about a minute for the first content token
  on the DeepSeek official endpoint: requests now ask for a direct answer
  (`{"thinking":{"type":"disabled"}}`). DeepSeek V4 enables thinking by default with high
  effort, and its reasoning pass delayed the output. `PromptRequest` carries the choice
  (default: direct answer, `with_thinking()` opts back in); both the streaming and
  non-streaming request builders honour it, and prompt/cache keys include it so fast and
  reasoning answers do not collide.
- Switching the AI provider now updates the endpoint, model and API-key field together:
  the outgoing key is remembered per provider, the matching key is restored, and an
  unknown provider starts disabled with an empty key field. Key environment names reset
  to the default, prompt caches are invalidated, and raw Custom-endpoint edits use the
  same key memory. Selecting the already-active provider leaves the current key untouched.
- Provider status now names the selected preset (`deepseek`, `opencode-go`,
  `openai-compatible`, `ollama`) instead of printing the shared transport name `openai`.
- Decompiler structuring now duplicates terminal blocks that end in padding/external
  tail calls instead of leaving a jump to an unemitted label, drops straight-line code
  after an unconditional `return`, and folds jumps to return-only labels. Shared blocks
  with multiple predecessors get a label at their first emission, so no generated `goto`
  targets a missing label. When an earlier arm consumed the post-dominator join, short
  acyclic arms are now duplicated instead of falling back to a raw jump, and jumps to the
  last labelled short tail copy that tail plus the function epilogue. Tail scanning now
  recognizes `} else {` as a closing brace, and jumps to any label whose balanced tail
  ends in `return`/a tail call copy that tail too. The last-label cleanup tail may now be
  a large balanced `if`/assignment region, not only straight-line statements. Dead code
  after `return` also drops orphan labels, ignoring references that only occur inside the
  dead tail, and exposes the rest of the unreachable tail. A jump at the end of one
  sibling arm to the short straight-line tail of its sibling is copied and removed;
  conditional jumps at an arm end become `if (cond) { tail }`.
  Copied tails now restore one indentation level per brace instead of collapsing nested
  bodies to a single column. On the notepad corpus this raises structured functions from
  432/502 (86.1%) to 484/502 (96.4%) while keeping instruction coverage at 99.4%; add
  regression tests for dead-tail, orphan-label, sibling-tail, balanced-if and last-tail
  cleanup.

### Changed

- An x86 function that re-reads an argument after the prologue moved `esp` used to get two
  parameters with the same name (`nested_loops(int32 arg1, int32 arg1)`); parameters are now
  deduplicated by name, giving `nested_loops(int32 arg1)` as on x64.
- The analysis pass no longer formats the statements it discards. `lift_block` gained a
  `render` flag: the first pass only needs the block's effects (writes, calls, stack use,
  argument copies), so it skips ~37 000 statement renderings for Notepad's biggest function.
  The emitted text is byte-identical; wall-clock change is within run-to-run noise on this
  machine (1.6 s either way for the 45 KB function), so treat it as work removed rather than
  a measured speedup.
- Decompiled pseudocode now reads closer to source: `x = (x + n);` becomes `x += n;`,
  `r = *(T*)(a); r -= k; *(T*)(a) = r;` collapses to `*(T*)(a) -= k;`,
  `if (!(x == 0))` becomes `if (x != 0)`, the redundant wrapper around a whole condition
  is dropped (`if ((x == 0))` becomes `if (x == 0)`), and a temporary written once and read
  once is inlined into its reader. The negation helper used chained
  string replacements that cancelled each other (`==` → `!=` → `==`), so equality branches
  kept a redundant `!`; it now flips exactly one comparison and leaves `&&`/`||` conditions
  wrapped in an explicit `!`.
- L1 structuring recovers more shape and never emits a `goto` to a missing label. Shared
  tails may now be up to 48 lines and may contain inner labels (the label is not copied),
  consecutive identical terminating arms merge into `if (a || b) { … }` (the compiler's
  short-circuit `||` lowering), and loop headers get a label when another path jumps to
  them. Labels that a live forward `goto` still names are kept when dead tails are pruned,
  so no jump dangles. On notepad this measures **487/502 structured (97.0%)** with
  coverage unchanged at 99.4%; corpus x64 stays at 31/31 and 97.4% coverage, and the
  golden snapshots were regenerated for the new statement forms.
- Indirect calls through the import table are rendered as the imported API instead of a
  pointer expression: `(**(u64*)(0x...))()` becomes `CreateFileW(...)`. IAT slots of named
  imports map to the function name (sanitised to a C identifier), and the arguments written
  to `rcx`/`rdx`/`r8`/`r9` before the call are rendered in ABI order (`&var_XX` for a
  stack-slot address). A `jmp [IAT]` tail call renders as `return Api();`. On notepad 1657
  of 1667 indirect calls are named and most carry their captured arguments.
- Call arguments that pass the address of a local now print as `&var_XX` instead of a raw
  `(rbp - 0x30)` pointer expression, for direct and indirect calls alike, and a full-width
  copy of an argument register (`rsi = arg1`) keeps the parameter name for later reads.
- Callee-save bookkeeping is removed whenever a spill slot is only saved and restored:
  one save may pair with several restores, and a save that is never restored counts as
  dead. On notepad the pseudocode shrinks by ~800 lines and most functions now start with
  their first real statement instead of `var_8 = rbx; ...`.
- A function that rebuilds its own frame pointer (`rbp = rsp - 0x27`) now names positive
  `[rbp+off]` accesses as locals too, and a stack store that is overwritten later in the
  same straight-line run without a read is removed.
- The `frame = rsp` alias is treated like `rsp` for stack-slot naming, so spills made
  through it become `var_XX` and are hidden like any other callee-save pair.
- Small sign-extended constants print as negative literals (`or rax, -1` instead of
  `or rax, 0xffffffffffffffff`, `and r9, -8` for a `0xfffffffffffffff8` mask), and
  `x |= -1` folds to `x = -1`.
- Callee-saved restores that lead directly to a `return` are dropped as calling-convention
  bookkeeping, and an argument copied into a callee-saved register in the prologue
  (`rsi = arg1;`) is replaced by the parameter name at every later read when the register
  is never assigned again. On notepad this removes ~2,250 lines of epilogue noise and
  turns `*(u16*)(rsi)` into `*(u16*)(arg1)` throughout L2 output.
- A `goto` whose target block was emitted without a label now gets that label materialized
  at the statement address, so generated pseudo-C no longer references a missing label on
  samples where the structurer emits a single-predecessor block inline (Go and kernel32).
- A control transfer past the end of the analysed function is a tail jump into the next
  function, so it renders as `return sub_xxx();` instead of a `goto` to a foreign block.
  This removes the remaining dangling jumps on kernel32 (0 of 2567 functions now reference
  a missing label) and on Go.
- L2 signatures fill gaps between recovered x64 argument registers: a function that reads
  `rcx` and `r8` is declared with `arg1, arg2, arg3` instead of silently omitting the unused
  `rdx` parameter, so the parameter list matches the calling convention.
- Dead-tail pruning now removes a nested `if { … }` region as a whole instead of stopping at
  its closing brace, which used to leave a stray `}` behind (3 kernel32 and 11 Go functions).
  A region holding a label that an outside jump still names is kept, so the fix does not
  reintroduce dangling jumps. kernel32 now has 0 of 2567 functions with unbalanced braces or
  missing labels.
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
