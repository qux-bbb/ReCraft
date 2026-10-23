# ReCraft

> **Status: M0 complete, M1 in progress.** You can import a PE, analyze it (function
> discovery from `.pdata`/exports/entry + recursive descent), browse disassembly and
> L0 pseudo-C, rename/comment with undo/redo, and reopen projects — from both the CLI and
> the native UI. Strings, xrefs, runtime recovery (Go/Rust) and MCP/AI are next.
>
> See [`docs/M0-REPORT.md`](docs/M0-REPORT.md) and [`docs/M1.md`](docs/M1.md) for metrics
> and known limitations.

ReCraft is an AI-native reverse-engineering platform: a native desktop disassembler and
decompiler for Windows binaries, with an MCP server and LLM assistance built in from day one.

It is an independent, clean-room implementation. It is **not** affiliated with Hex-Rays or
IDA Pro, and does not use IDA's SDK, database format, or signature libraries.

## Goals (v1)

- **PE x64 + Win32 x86** static analysis. ELF / ARM64 / firmware later.
- **Self-built decompiler** (LIR → SSA → structured C-like pseudocode). Readable output,
  not guaranteed compilable — every token maps back to its instruction address.
- **Scales to 1 GB samples**, stays interactive (paged storage, background analysis).
- **Malware-triage first**: strings, imports, cross-references, API behaviour tagging,
  "find `main`", "follow the network callback".
- **MCP-native AI**: function explanation, naming proposals, sample summaries,
  natural-language queries over the database. AI can only *propose* changes; a human applies them.
- **Native egui UI**, dense IDA-like layout, modern look, English + 简体中文.

## Architecture

Headless core + command bus + MCP server + UI client. See [`docs/DESIGN.md`](docs/DESIGN.md).

```
recraft-core ── recraft-analysis ── recraft-ir ── recraft-decompile
      │                                               │
      └──────────── recraft-app ── recraft-mcp / recraft-cli / recraft-ui
```

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

First import a binary from the command line:

```powershell
cargo run -p recraft-cli -- import C:\Windows\System32\notepad.exe -o notepad.rcraft
cargo run -p recraft-cli -- disasm notepad.rcraft --count 40
cargo run -p recraft-cli -- decompile notepad.rcraft --addr entry
cargo run -p recraft-cli -- analyze notepad.rcraft
cargo run -p recraft-cli -- strings notepad.rcraft
cargo run -p recraft-cli -- xrefs notepad.rcraft --addr entry
cargo run -p recraft-cli -- eval C:\Windows\System32\kernel32.dll
```

The analysis corpus is built with MSVC (no CRT) and doubles as a regression benchmark
where exports + `.pdata` are ground truth:

```powershell
.\corpus\build.ps1
cargo run -p recraft-cli -- eval .\corpus\build\x64\corpus.exe
```

The UI binary renders with OpenGL (Glow); wgpu/DX12 crashed on an Intel UHD driver
during the spike, see [`docs/adr/ADR-002-ui-framework.md`](docs/adr/ADR-002-ui-framework.md).

## Contributing

Contributions are welcome, but **every contribution requires a signed CLA**
(this is what makes the AGPL + commercial dual-license model work).

See [`CONTRIBUTING.md`](CONTRIBUTING.md) and [`CLA.md`](CLA.md).

Never commit malware samples or binaries — use local, uncommitted test data only.

## License

- Open-source edition: **AGPL-3.0-only** (see [`LICENSE`](LICENSE)).
- Commercial license for use without AGPL obligations: see [`LICENSE-COMMERCIAL.md`](LICENSE-COMMERCIAL.md).
- The "ReCraft" name and logo are governed by [`TRADEMARK.md`](TRADEMARK.md).

---

## 简体中文

ReCraft 是一个 AI 原生的逆向工程平台：原生桌面反汇编/反编译器，内置 MCP 与 LLM 辅助。
独立实现，与 Hex-Rays / IDA Pro 无关，不使用其 SDK、数据库格式或签名库。

v1 目标：PE x64/x86 静态分析、自研反编译器（输出可读、不保证可编译）、支持 1GB 样本、
恶意样本初筛工作流、MCP 原生 AI、egui 原生界面（英文 + 简中）。

当前处于 pre-alpha 阶段，设计文档见 [`docs/DESIGN.md`](docs/DESIGN.md)。
贡献需要签署 CLA，开源版许可为 AGPL-3.0-only，另有商业许可。
