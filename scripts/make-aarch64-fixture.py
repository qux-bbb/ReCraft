#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 ReCraft contributors
"""Build the tiny AArch64 ELF fixture used by the golden snapshots.

The output is a ~2 KB static ELF with 32 small functions (.symtab names and sizes),
which keeps the committed fixture small while still exercising analysis, the neutral
decoder, the A64 lifter and the pseudo-C renderer end to end.

Usage: python scripts/make-aarch64-fixture.py [output]
"""

import struct
import sys

BASE = 0x1000

# A64 encodings: add/sub immediate, ldr/str (unscaled), cmp, b.cond, bl, ret.
ADD = 0x91000000
SUB = 0xD1000000
LDR = 0xF9400000
STR = 0xF9000000
CMP = 0xF1000000
B_COND = 0x54000000
BL = 0x94000000
RET = 0xD65F03C0
NOP = 0xD503201F


def add(rd, rn, imm):
    return ADD | ((imm & 0xFFF) << 10) | ((rn & 31) << 5) | (rd & 31)


def sub(rd, rn, imm):
    return SUB | ((imm & 0xFFF) << 10) | ((rn & 31) << 5) | (rd & 31)


def ldr(rt, rn, off):
    return LDR | (((off // 8) & 0xFFF) << 10) | ((rn & 31) << 5) | (rt & 31)


def str_(rt, rn, off):
    return STR | (((off // 8) & 0xFFF) << 10) | ((rn & 31) << 5) | (rt & 31)


def cmp(rn, imm):
    return CMP | ((imm & 0xFFF) << 10) | ((rn & 31) << 5) | 31


def b_cond(offset_words, cond=0x0):
    return B_COND | ((offset_words & 0x7FFFF) << 5) | (cond & 0xF)


def bl(offset_words):
    return BL | (offset_words & 0x3FFFFFF)


def function(index):
    """Return the instruction words of one fixture function."""
    if index % 4 == 0:
        return [add(0, 0, index + 1), RET]
    if index % 4 == 1:
        return [sub(1, 1, index + 1), add(2, 1, index), RET]
    if index % 4 == 2:
        return [ldr(2, 0, 8), str_(3, 0, 16), add(0, 2, 1), RET]
    return [cmp(0, index + 1), b_cond(2, 0xB), add(0, 0, 1), RET]


def build():
    functions = []
    offset = 0
    for index in range(32):
        words = function(index)
        name = f"fixture_{index:02d}"
        functions.append((name, BASE + offset * 4, len(words) * 4))
        offset += len(words)
    code = b"".join(struct.pack("<I", word) for index in range(32) for word in function(index))
    code_size = len(code)

    # Layout: ELF header, one program header, code at 0x1000, section headers at the end.
    ehsize, phentsize, phnum = 64, 56, 1
    code_off = 0x1000
    shstr = b"\0.text\0.symtab\0.strtab\0.shstrtab\0"
    strtab = b"\0" + b"".join(name.encode() + b"\0" for name, _, _ in functions)
    names_off = {name: strtab.index(name.encode()) for name, _, _ in functions}

    code_end = code_off + code_size
    symtab_off = code_end
    symtab = b"\0" * 24  # null symbol
    for name, addr, size in functions:
        symtab += struct.pack(
            "<IBBHQQ", names_off[name], 0x12, 0, 1, addr, size  # GLOBAL, OBJECT/FUNC
        )
    strtab_off = symtab_off + len(symtab)
    shstr_off = strtab_off + len(strtab)
    shoff = (shstr_off + len(shstr) + 7) & ~7

    header = struct.pack(
        "<4sBBBBB7xHHIQQQIHHHHHH",
        b"\x7fELF", 2, 1, 1, 0, 0,  # magic, ELF64, little, version, SYSV, ABI version
        2, 0xB7, 1,                  # ET_EXEC, EM_AARCH64, version
        BASE,                        # entry
        ehsize, 0, 0,                # phoff, shoff (patched below), flags
        ehsize, phentsize, phnum, 64, 0, 0,  # ehsize..shstrndx (patched)
    )
    phdr = struct.pack(
        "<IIQQQQQQ",
        1, 5,                        # PT_LOAD, R+X
        code_off, BASE, BASE, code_size, code_size, 0x1000,
    )

    shdrs = b""
    shdrs += b"\0" * 64  # SHT_NULL
    shdrs += struct.pack(
        "<IIQQQQIIQQ", 1, 1, 6, BASE, code_off, code_size, 0, 0, 4, 0
    )  # .text PROGBITS, ALLOC|EXECINSTR
    shdrs += struct.pack(
        "<IIQQQQIIQQ", 7, 2, 0, 0, symtab_off, len(symtab), 3, 1, 8, 24
    )  # .symtab: link=3 (.strtab), one symbol per 24 bytes
    shdrs += struct.pack(
        "<IIQQQQIIQQ", 15, 3, 0, 0, strtab_off, len(strtab), 0, 0, 1, 0
    )  # .strtab
    shdrs += struct.pack(
        "<IIQQQQIIQQ", 24, 3, 0, 0, shstr_off, len(shstr), 0, 0, 1, 0
    )  # .shstrtab
    shnum = 5
    shstrndx = 4

    header = header[:40] + struct.pack("<Q", shoff) + header[48:]
    header = header[:58] + struct.pack("<HHH", 64, shnum, shstrndx)

    image = bytearray(header + phdr)
    image.extend(b"\0" * (code_off - len(image)))
    image.extend(code)
    image.extend(symtab)
    image.extend(strtab)
    image.extend(shstr)
    image.extend(b"\0" * (shoff - len(image)))
    image.extend(shdrs)
    return bytes(image)


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else "tests/fixtures/aarch64/tiny.elf"
    data = build()
    with open(out, "wb") as handle:
        handle.write(data)
    print(f"wrote {out} ({len(data)} bytes)")


if __name__ == "__main__":
    main()
