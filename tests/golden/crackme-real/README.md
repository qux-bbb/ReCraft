# crackme-real snapshots (reserved)

This directory is intentionally empty while `corpus/real/crackme/build.ps1` reproduces the
pinned case binary (`6C84C77B...`) byte-for-byte: the 160 committed snapshots in `../crackme`
already describe it, and a second identical set would only double review noise.

When the MSVC toolchain drifts, record the rebuilt binary here:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File corpus/real/crackme/build.ps1 -Snapshots
```

`the_real_crackme_fixture_matches_its_snapshots` prefers any `.c` files found here and falls
back to `../crackme` otherwise, so the original case binary keeps its own set.
