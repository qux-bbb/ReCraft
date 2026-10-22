# Security Policy

## Reporting a vulnerability

Please **do not** open a public issue. Report privately to the maintainer
(`qux-bbb` via GitHub) with:

- affected version / commit,
- a minimal reproduction (malformed input where possible),
- impact assessment.

You will get an acknowledgement within 7 days. Coordinated disclosure is expected.

## Threat model

ReCraft parses **untrusted binaries**. The loader, decoder and analysis pipeline are
therefore security-critical and must never:

- execute the sample or any of its code,
- resolve or load the sample's dependencies,
- trust lengths, offsets, or counts from the file,
- panic, hang, or allocate unbounded memory on malformed input.

Fuzzing targets (`cargo-fuzz`) and a corpus of malformed PE files live under
`crates/recraft-format` and are part of CI's long-running jobs.

## AI / MCP surface

- The MCP server may be exposed over HTTP; it must require authentication and be bound to
  localhost by default.
- The AI client is opt-in and offline by default. Samples are never uploaded without an
  explicit user action.
- AI write operations are proposals only; they must be applied by a human through the
  command bus.

## Out of scope

- Vulnerabilities in third-party dependencies (report upstream; we track via `cargo-deny`).
- Issues requiring the attacker to already have code execution on the analyst's machine.
