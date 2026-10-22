# Contributing to ReCraft

Thanks for your interest. ReCraft is pre-alpha and the design is still moving, so please
open an issue before starting anything non-trivial.

## Before you write code

1. **Read [`docs/DESIGN.md`](docs/DESIGN.md).** It defines the architecture, the crate graph,
   and the non-goals. Changes that cut across those need an ADR entry first.
2. **Sign the CLA.** Every contribution (code, docs, assets) requires the
   [Individual CLA](CLA.md) or a Corporate CLA. PRs without a signed CLA cannot be merged.
   This is required by the AGPL + commercial dual-license model.
3. **No malware, no binaries.** Do not commit samples, packed executables, or anything that
   requires execution. Tests use source-built fixtures and synthetic data.

## Development setup

- Rust stable (1.85+). `rust-toolchain.toml` pins stable and the required components.
- Windows is the primary platform. Core crates must stay free of Windows-only dependencies;
  put OS-specific code behind `#[cfg(windows)]` or in a dedicated crate.

## Checks (must pass locally)

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check licenses bans sources   # if cargo-deny is installed
```

CI runs the same on Windows, plus `cargo-deny` on Linux.

## Code rules

- Every source file carries `SPDX-License-Identifier: AGPL-3.0-only` and the copyright line.
- Only permissive dependencies (`deny.toml` allowlist). No GPL-2.0-only, no GPL-3.0.
- Public APIs need doc comments; internal code should explain *why*, not *what*.
- Prefer explicit error types over `anyhow` in library crates (`recraft-common::Error`).
- Do not add a dependency to a core crate without updating `docs/DESIGN.md` §1.1 if it
  changes the crate's responsibility.

## Commit / PR style

- Conventional Commits (`feat:`, `fix:`, `refactor:`, `docs:`, `test:`, `chore:`).
- Keep PRs focused; include tests for behaviour changes.
- Security-relevant changes (loader, decompiler) need a note on fuzz/robustness impact.

## Reporting security issues

See [`SECURITY.md`](SECURITY.md). Never open a public issue for a vulnerability.
