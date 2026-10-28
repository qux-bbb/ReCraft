# ReCraft

[简体中文](README.md) | **English**

> **Status: M3 complete, M4 (public preview) in progress.** Import/analyze PE32/PE32+
> (x64 + Win32), structured pseudo-C with signatures/ABI/types, rename/comment/prototype/
> struct-type writes through a journaled command bus with undo, MCP read-only server, and
> provider-backed AI (explain / rename proposals / triage) with a propose-apply UI.
> Quality gates: corpus semantic coverage **97.4%**, structured rate **100%**; notepad
> **99.4%** coverage and **97.0%** structured (487/502); 1 GiB
> sample: import ~4 s, analyze ~0.3 s, peak 5.6 MiB (debug). See
> [`docs/M4.md`](docs/M4.md) for the preview checklist and [`docs/FAQ.md`](docs/FAQ.md).

ReCraft is an AI-native reverse-engineering platform: a native desktop disassembler and
decompiler for Windows binaries, with an MCP server and LLM assistance built in from day one.

It is an independent, clean-room implementation. It is **not** affiliated with Hex-Rays or
IDA Pro, and does not use IDA's SDK, database format, or signature libraries.

## Why ReCraft

- **Native, not Electron**: Rust + egui desktop app; no webview, no Python runtime.
- **Decompiler in the open**: LIR → SSA-lite → structured pseudo-C; every token maps back
  to an instruction address; golden snapshots guard every change.
- **AI that proposes, never silently writes**: proposals carry evidence addresses and a
  confidence; writes go through the same journaled command bus as manual edits (undoable).
- **Provider required**: AI features use a configured LLM provider; prompts
  are redacted + budgeted, results cached and audited locally.
- **MCP-native**: Claude/Cursor and other agents can query functions, pseudocode, xrefs,
  strings, imports and call graphs through a read-only MCP server.
- **Built for triage**: API behaviour tagging, strings/xrefs, runtime symbol recovery
  (Go/Rust/C++), one-command triage with suspicious-function ranking.
- **Honest engineering**: documented limitations, measured quality gates, reproducible
  corpus (MSVC, no CRT, linker map as ground truth).

| | ReCraft | IDA Pro | Ghidra |
|---|---|---|---|
| Price / license | AGPL-3.0-only (+ commercial) | commercial | Apache-2.0 |
| UI | native Rust/egui | native | Java/Swing |
| Decompiler | self-built (readable, not compilable) | Hex-Rays (mature) | mature |
| MCP / AI assist | built in (provider-backed) | plugins/add-ons | plugins |
| 1 GiB sample gate | ✅ automated | ✅ | ✅ |
| IDA plugin ecosystem | ❌ | ✅ | ❌ |

## Documents

[`docs/README.md`](docs/README.md) indexes all design/milestone docs; start with
[`docs/DESIGN.md`](docs/DESIGN.md), then [`docs/FAQ.md`](docs/FAQ.md) and
[`docs/ROADMAP.md`](docs/ROADMAP.md). The icon concept and usage rules are documented in
[`docs/ICON.md`](docs/ICON.md).

## Goals (v1)

- **PE x64 + Win32 x86** static analysis. ELF / ARM64 / firmware later.
- **Self-built decompiler** (LIR → SSA → structured C-like pseudocode). Readable output,
  not guaranteed compilable — every token maps back to its instruction address.
- **Scales to 1 GB samples**, stays interactive (paged storage, background analysis).
- **Malware-triage first**: strings, imports, cross-references, API behaviour tagging,
  "find `main`", "follow the network callback".
- **MCP-native AI**: function explanation, naming proposals, sample summaries,
  natural-language queries over the database. AI can only *propose* changes; a human applies them.
- **Native egui UI**, dense IDA-like layout, modern look, English + 简体中文
  (Fluent `.ftl` resources under `crates/recraft-ui/locales/`).

## Architecture

Headless core + command bus + MCP server + native UI client. See
[`docs/DESIGN.md`](docs/DESIGN.md) for the full design.

```mermaid
flowchart TB
    subgraph storage
        fmt[recraft-format<br/>PE32/PE32+] --> st[recraft-storage<br/>SQLite + journal]
        dec[recraft-decode<br/>iced-x86] --> ir[recraft-ir<br/>LIR lift]
    end
    st --> core[recraft-core<br/>Session / Command bus / undo]
    ir --> core
    core --> ana[recraft-analysis<br/>CFG / xrefs / strings / symbols]
    core --> dc[recraft-decompile<br/>L1 structure / L2 types]
    ana --> dc
    dc --> app[recraft-app]
    core --> app
    app --> cli[recraft-cli]
    app --> ui[recraft-ui<br/>egui/Glow]
    app --> mcp[recraft-mcp<br/>stdio JSON-RPC]
    app --> ai[recraft-ai<br/>explain / proposals / providers]
    ai --> cli
    ai --> ui
```

All writes (rename/comment/prototype/apply_type) go through `recraft-core`'s journaled
command bus; analysis output lives in derived tables and can be rebuilt at any time.

## Building

Requires Rust **1.85+** (stable). No other toolchain is needed on Windows.

```powershell
cargo build --workspace
cargo test --workspace
```

Run the desktop app against a project (or drag a PE file onto its window):

```powershell
cargo run -p recraft-ui -- path\to\sample.rcraft
```

First import a binary from the command line (read-only commands such as `strings`,
`funcs`, `decompile`, `search` or `callgraph` also accept a PE path directly and
auto-analyze it into a temporary project):

```powershell
cargo run -p recraft-cli -- analyze C:\Windows\System32\notepad.exe --out notepad.rcraft
cargo run -p recraft-cli -- import C:\Windows\System32\notepad.exe -o notepad.rcraft
cargo run -p recraft-cli -- disasm notepad.rcraft --count 40
cargo run -p recraft-cli -- decompile notepad.rcraft --addr entry
cargo run -p recraft-cli -- decompile notepad.rcraft --addr entry --structured
cargo run -p recraft-cli -- decompile notepad.rcraft --addr entry --l2
cargo run -p recraft-cli -- analyze notepad.rcraft
cargo run -p recraft-cli -- strings notepad.rcraft
cargo run -p recraft-cli -- xrefs notepad.rcraft --addr entry
cargo run -p recraft-cli -- imports notepad.rcraft
cargo run -p recraft-cli -- search notepad.rcraft --bytes "4d5a" --limit 5
cargo run -p recraft-cli -- search notepad.rcraft --string "This program"
cargo run -p recraft-cli -- callgraph notepad.rcraft --json
cargo run -p recraft-cli -- batch notepad.rcraft --script batch.json --dry-run
cargo run -p recraft-cli -- eval C:\Windows\System32\kernel32.dll
cargo run -p recraft-cli -- metrics notepad.rcraft
cargo run -p recraft-cli -- golden notepad.rcraft --dir golden-out --check
```

The analysis corpus is built with MSVC (no CRT) and doubles as a regression benchmark
where exports + `.pdata` are ground truth:

```powershell
.\corpus\build.ps1
cargo run -p recraft-cli -- eval .\corpus\build\x64\corpus.exe
# Linker map: ground truth for every function, including non-exported helpers.
cargo run -p recraft-cli -- eval .\corpus\build\x64\corpus.exe --map .\corpus\build\x64\corpus.map
```

A Go sample validates runtime symbol recovery (kept `pclntab`):

```powershell
.\corpus\build-go.ps1
cargo run -p recraft-cli -- import .\corpus\build\go\corpus-go.exe -o go.rcraft
cargo run -p recraft-cli -- analyze go.rcraft
cargo run -p recraft-cli -- funcs go.rcraft    # main.main, runtime.*, ...
```

A C++ sample validates symbol demangling:

```powershell
.\corpus\build-cpp.ps1
cargo run -p recraft-cli -- import .\corpus\build\cpp\corpus-cpp.dll -o cpp.rcraft
cargo run -p recraft-cli -- analyze cpp.rcraft
cargo run -p recraft-cli -- funcs cpp.rcraft   # recraft::DemoTarget::get(void) const, ...
cargo run -p recraft-cli -- demangle "?overloaded@recraft@@YAHH@Z"
```

### Packaging

```powershell
pwsh -File scripts/package.ps1 -NoInstaller   # release build + green zip + SHA256SUMS
# dist/recraft-v0.1.0-windows-x64.zip, dist/ReCraft-0.1.0/, dist/SHA256SUMS.txt
pwsh -File scripts/package.ps1                # also builds the NSIS installer if makensis exists
# dist/recraft-0.1.0-setup.exe (per-user install to %LOCALAPPDATA%\ReCraft)
pwsh -File scripts/verify-installer.ps1       # install -> CLI/GUI -> uninstall, fully scripted
```

See [`docs/RELEASE.md`](docs/RELEASE.md) for the release checklist; CI uploads the zip,
the NSIS installer (installer job) and checksums as artifacts.

### Installing (unsigned preview builds)

v0.1 preview binaries are **not code-signed** (a deliberate decision; see
[`docs/RELEASE.md`](docs/RELEASE.md)). Windows SmartScreen will show "Unknown publisher"
until the project adopts a trusted OSS signing service:

1. Download `recraft-v0.1.0-windows-x64.zip` or `recraft-0.1.0-setup.exe` together with
   `SHA256SUMS.txt`.
2. Verify the download before running:
   ```powershell
   Get-FileHash .\recraft-0.1.0-setup.exe -Algorithm SHA256
   # compare with the matching line in SHA256SUMS.txt
   ```
3. If SmartScreen appears: **More info → Run anyway**. The installer is per-user
   (`%LOCALAPPDATA%\ReCraft`, no elevation) and the portable zip needs no installation.

Signed builds (and the exact certificate identity) will be announced when available;
until then checksums are the integrity reference.

### Demo

```powershell
pwsh -File scripts/demo.ps1 -Keep     # 9-step CLI walkthrough, then open the GUI
```

The full 10-minute script (CLI + GUI + MCP client + local LLM) is in
[`docs/DEMO.md`](docs/DEMO.md).

The UI binary renders with OpenGL (Glow); wgpu/DX12 crashed on an Intel UHD driver
during the spike, see [`docs/adr/ADR-002-ui-framework.md`](docs/adr/ADR-002-ui-framework.md).

### MCP server

Read-only Model Context Protocol tools over stdio: `list_functions`, `get_disassembly`,
`get_pseudocode` (L0/L1/L2), `xrefs_to/from`, `list_strings`, `list_imports`,
`callgraph`, `search_bytes`, `function_summary`, plus project resources.

```powershell
cargo run -p recraft-cli -- serve-mcp notepad.rcraft
# newline-delimited JSON-RPC 2.0 on stdin/stdout; see docs/MCP.md
```

### AI assist

Provider-backed explanation, rename proposals and sample triage; proposals carry
evidence addresses and are only applied through the journaled command bus. The GUI's **AI**
sidebar can configure the optional LLM provider (endpoint/model/API-key env var, persisted to
`%LOCALAPPDATA%\ReCraft\ai-settings.json`) and run **Explain**/**Polish** without any CLI
configuration:

```powershell
cargo run -p recraft-cli -- ai explain notepad.rcraft --addr entry
cargo run -p recraft-cli -- ai suggest notepad.rcraft --out proposals.json
cargo run -p recraft-cli -- ai triage notepad.rcraft
cargo run -p recraft-cli -- ai apply notepad.rcraft proposals.json --yes   # undoable
```

## Contributing

Contributions are welcome, but **every contribution requires a signed CLA**
(this is what makes the AGPL + commercial dual-license model work).

See [`CONTRIBUTING.md`](CONTRIBUTING.md) and [`CLA.md`](CLA.md).

Never commit malware samples or binaries — use local, uncommitted test data only.

## License

- Open-source edition: **AGPL-3.0-only** (see [`LICENSE`](LICENSE)).
- Commercial license for use without AGPL obligations: see [`LICENSE-COMMERCIAL.md`](LICENSE-COMMERCIAL.md).
- The "ReCraft" name and logo are governed by [`TRADEMARK.md`](TRADEMARK.md).
