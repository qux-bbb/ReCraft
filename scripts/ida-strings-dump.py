# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 ReCraft contributors
"""Dump IDA's string list to JSON.

Run it headless so the list can be compared with `recraft strings`:

    ida64.exe -A -S"scripts/ida-strings-dump.py C:\\temp\\ida-strings.json" target.exe

The output is a JSON array of `{addr, len, type, text}`; `len` includes the terminator, as
`idautils.Strings()` reports it, and `type` is IDA's `STRTYPE_*` number.
"""

import json

import idautils
import idc

out = idc.ARGV[1] if len(idc.ARGV) > 1 else r"C:\Windows\Temp\ida-strings.json"
rows = []
for string in idautils.Strings():
    try:
        rows.append(
            {
                "addr": int(string.ea),
                "len": int(string.length),
                "type": int(string.strtype),
                "text": str(string),
            }
        )
    except Exception as error:  # noqa: BLE001 - report and keep going
        rows.append({"addr": int(string.ea), "len": int(string.length), "error": str(error)})
with open(out, "w", encoding="utf-8") as handle:
    json.dump(rows, handle, ensure_ascii=False)
idc.qexit(0)
