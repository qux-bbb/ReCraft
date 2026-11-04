# NOTICE

ReCraft is licensed under **AGPL-3.0-only** (see [LICENSE](LICENSE)); there is no commercial
license. Contributions are accepted under the same license (inbound = outbound, DCO
sign-off). Documentation and the design documents in `docs/` are distributed under the same
license. "ReCraft" and the ReCraft logo are trademarks of the project authors (see
[TRADEMARK.md](TRADEMARK.md)).

ReCraft links the following third-party components. Their licenses are compatible with
AGPL-3.0-only (no GPL-2.0-only / GPL-3.0 code is used). Direct dependencies:

| Component | Purpose | License |
|---|---|---|
| egui / eframe / epaint | native UI | MIT OR Apache-2.0 |
| egui_dock | dockable pane layout | MIT |
| iced-x86 | x86/x64 decoder | MIT |
| rusqlite (bundled) | project storage | MIT; bundled SQLite is public domain |
| goblin / pelite | PE parsing helpers | MIT |
| rustc-demangle | symbol demangling | MIT OR Apache-2.0 |
| rayon | parallel analysis | MIT OR Apache-2.0 |
| serde / serde_json | serialization | MIT OR Apache-2.0 |
| clap | CLI parsing | MIT OR Apache-2.0 |
| tracing | diagnostics | MIT |
| ureq + rustls | HTTPS transport for LLM providers | MIT OR Apache-2.0 |
| tempfile | tests and temporary projects | MIT OR Apache-2.0 |

Animation/UI fonts: the build embeds a subset of **Noto Sans Mono CJK**, licensed under the
SIL Open Font License 1.1. Malware samples, signatures and analysis outputs are never
included in the repository or in release artifacts.

Dependency license/ban/source policy is enforced by `cargo-deny` (see `deny.toml`, CI
`licenses` job) and the repository header/notice check (`scripts/check-license.ps1`).
