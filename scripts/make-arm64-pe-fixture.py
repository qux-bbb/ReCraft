#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 ReCraft contributors
"""Build the tiny ARM64 PE fixture used by the loader/analysis tests.

It has two sections: `.text` with 8 small functions and `.pdata` with the matching ARM64
exception-unwind entries (8 bytes each: BeginAddress + packed length), which is how a
PE-ARM64 image tells an analyser where its functions are.

Usage: python scripts/make-arm64-pe-fixture.py [output]
"""

import struct
import sys

IMAGE_BASE = 0x1_4000_0000
TEXT_RVA = 0x1000
PDATA_RVA = 0x2000
CODE_OFF = 0x200
PDATA_OFF = 0x400
SIZE_OF_HEADERS = 0x200
SIZE_OF_IMAGE = 0x3000

ADD = 0x91000000
SUB = 0xD1000000
RET = 0xD65F03C0


def add(rd, rn, imm):
    return ADD | ((imm & 0xFFF) << 10) | ((rn & 31) << 5) | (rd & 31)


def sub(rd, rn, imm):
    return SUB | ((imm & 0xFFF) << 10) | ((rn & 31) << 5) | (rd & 31)


def code():
    words, lengths = [], []
    for index in range(8):
        body = [add(0, 0, index + 1), sub(1, 1, index + 1), RET]
        words.extend(body)
        lengths.append(len(body))
    return b"".join(struct.pack("<I", word) for word in words), lengths


def pdata(lengths):
    """ARM64 RUNTIME_FUNCTION: BeginAddress plus packed unwind (flag 0, length in words)."""
    entries = b""
    offset = 0
    for length in lengths:
        entries += struct.pack("<II", TEXT_RVA + offset * 4, (length & 0x7FF) << 2)
        offset += length
    return entries


def optional_header(entry_rva, size_of_code, size_of_initialized_data, pdata_size):
    # PE32+ optional header: 240 bytes. Offsets follow the PE specification (SizeOfImage is
    # at 0x38, Subsystem at 0x44, NumberOfRvaAndSizes at 0x6C, directories at 0x70).
    header = bytearray(240)
    struct.pack_into("<H", header, 0x00, 0x20B)
    struct.pack_into("<I", header, 0x04, size_of_code)
    struct.pack_into("<I", header, 0x08, size_of_initialized_data)
    struct.pack_into("<I", header, 0x10, entry_rva)
    struct.pack_into("<I", header, 0x14, TEXT_RVA)
    struct.pack_into("<Q", header, 0x18, IMAGE_BASE)
    struct.pack_into("<I", header, 0x20, 0x1000)   # SectionAlignment
    struct.pack_into("<I", header, 0x24, 0x200)    # FileAlignment
    struct.pack_into("<HH", header, 0x28, 6, 0)    # OS version
    struct.pack_into("<HH", header, 0x30, 0, 0)    # image version
    struct.pack_into("<HH", header, 0x32, 6, 0)    # subsystem version
    struct.pack_into("<I", header, 0x38, SIZE_OF_IMAGE)
    struct.pack_into("<I", header, 0x3C, SIZE_OF_HEADERS)
    struct.pack_into("<H", header, 0x44, 3)        # Subsystem: console
    struct.pack_into("<H", header, 0x46, 0x8160)   # DllCharacteristics
    struct.pack_into("<Q", header, 0x48, 0x100000)  # stack reserve
    struct.pack_into("<Q", header, 0x50, 0x1000)   # stack commit
    struct.pack_into("<Q", header, 0x58, 0x100000)  # heap reserve
    struct.pack_into("<Q", header, 0x60, 0x1000)   # heap commit
    struct.pack_into("<I", header, 0x6C, 16)       # NumberOfRvaAndSizes
    struct.pack_into("<II", header, 0x70 + 3 * 8, PDATA_RVA, pdata_size)
    return bytes(header)

def section(name, virtual_size, rva, raw_size, raw_offset, characteristics):
    header = bytearray(40)
    header[: len(name)] = name.encode()
    struct.pack_into("<I", header, 8, virtual_size)
    struct.pack_into("<I", header, 12, rva)
    struct.pack_into("<I", header, 16, raw_size)
    struct.pack_into("<I", header, 20, raw_offset)
    struct.pack_into("<I", header, 36, characteristics)
    return bytes(header)


def build():
    text, lengths = code()
    table = pdata(lengths)
    text_raw = (len(text) + 0x1FF) & ~0x1FF
    pdata_raw = (len(table) + 0x1FF) & ~0x1FF

    dos = bytearray(64)
    dos[0:2] = b"MZ"
    struct.pack_into("<I", dos, 0x3C, 0x40)

    coff = struct.pack(
        "<HHIIIHH",
        0xAA64,   # IMAGE_FILE_MACHINE_ARM64
        2,        # sections
        0,        # timestamp
        0, 0,     # symbol table
        0xF0,     # size of optional header (PE32+)
        0x0022,   # executable | large address aware
    )
    optional = optional_header(TEXT_RVA, len(text), len(table), len(table))
    sections = section(".text", len(text), TEXT_RVA, text_raw, CODE_OFF, 0x60000020)
    sections += section(".pdata", len(table), PDATA_RVA, pdata_raw, PDATA_OFF, 0x40000040)

    image = bytearray(dos + b"PE\0\0" + coff + optional + sections)
    image.extend(b"\0" * (CODE_OFF - len(image)))
    image.extend(text)
    image.extend(b"\0" * (PDATA_OFF - len(image)))
    image.extend(table)
    image.extend(b"\0" * (PDATA_OFF + pdata_raw - len(image)))
    return bytes(image)


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else "tests/fixtures/aarch64/tiny-arm64.exe"
    data = build()
    with open(out, "wb") as handle:
        handle.write(data)
    print(f"wrote {out} ({len(data)} bytes)")


if __name__ == "__main__":
    main()
