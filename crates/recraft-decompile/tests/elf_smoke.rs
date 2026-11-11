// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! End-to-end smoke test for the ELF64 path: a synthetic x86-64 executable is
//! imported into a project, analysed, decompiled, and linted. The bytes are built
//! here (like the PE fixture in the format tests) so the test needs no Linux
//! toolchain.

use recraft_common::Address;
use recraft_core::Session;
use recraft_decompile::{LintLevel, lint, pseudocode_l2};

const TEXT_VA: u64 = 0x401_000;
const TEXT_OFFSET: usize = 0x1000;
const SYMTAB_OFFSET: usize = 0x1020;
const STRTAB_OFFSET: usize = 0x1068;
const SHSTRTAB_OFFSET: usize = 0x1075;
const SHSTRTAB_SIZE: usize = 33;
const SHOFF: usize = 0x1100;

fn put_u16(file: &mut [u8], offset: usize, value: u16) {
    file[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn put_u32(file: &mut [u8], offset: usize, value: u32) {
    file[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(file: &mut [u8], offset: usize, value: u64) {
    file[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn put_symbol(file: &mut [u8], offset: usize, name: u32, shndx: u16, value: u64, size: u64) {
    put_u32(file, offset, name);
    file[offset + 4] = 0x12; // STB_GLOBAL | STT_FUNC
    put_u16(file, offset + 6, shndx);
    put_u64(file, offset + 8, value);
    put_u64(file, offset + 16, size);
}

#[allow(clippy::too_many_arguments)]
fn put_section(
    file: &mut [u8],
    offset: usize,
    name: u32,
    kind: u32,
    flags: u64,
    address: u64,
    file_offset: u64,
    size: u64,
    link: u32,
    info: u32,
    align: u64,
    entsize: u64,
) {
    put_u32(file, offset, name);
    put_u32(file, offset + 4, kind);
    put_u64(file, offset + 8, flags);
    put_u64(file, offset + 16, address);
    put_u64(file, offset + 24, file_offset);
    put_u64(file, offset + 32, size);
    put_u32(file, offset + 40, link);
    put_u32(file, offset + 44, info);
    put_u64(file, offset + 48, align);
    put_u64(file, offset + 56, entsize);
}

fn build_elf() -> Vec<u8> {
    let mut file = vec![0u8; SHOFF + 5 * 64];

    file[..4].copy_from_slice(b"\x7fELF");
    file[4] = 2;
    file[5] = 1;
    file[6] = 1;
    put_u16(&mut file, 16, 2); // ET_EXEC
    put_u16(&mut file, 18, 0x3e); // EM_X86_64
    put_u32(&mut file, 20, 1);
    put_u64(&mut file, 24, TEXT_VA); // e_entry
    put_u64(&mut file, 32, 0x40); // e_phoff
    put_u64(&mut file, 40, SHOFF as u64);
    put_u16(&mut file, 52, 64);
    put_u16(&mut file, 54, 56);
    put_u16(&mut file, 56, 1);
    put_u16(&mut file, 58, 64);
    put_u16(&mut file, 60, 5);
    put_u16(&mut file, 62, 4);

    let loaded_size = (SHSTRTAB_OFFSET + SHSTRTAB_SIZE - TEXT_OFFSET) as u64;
    let ph = 0x40;
    put_u32(&mut file, ph, 1);
    put_u32(&mut file, ph + 4, 5);
    put_u64(&mut file, ph + 8, TEXT_OFFSET as u64);
    put_u64(&mut file, ph + 16, TEXT_VA);
    put_u64(&mut file, ph + 24, TEXT_VA);
    put_u64(&mut file, ph + 32, loaded_size);
    put_u64(&mut file, ph + 40, loaded_size);
    put_u64(&mut file, ph + 48, 0x1000);

    // main: mov eax, 1; ret. helper: xor eax, eax; ret.
    let mut text = [0x90u8; 0x20];
    text[..6].copy_from_slice(&[0xb8, 0x01, 0x00, 0x00, 0x00, 0xc3]);
    text[0x10..0x13].copy_from_slice(&[0x31, 0xc0, 0xc3]);
    file[TEXT_OFFSET..TEXT_OFFSET + text.len()].copy_from_slice(&text);

    put_symbol(&mut file, SYMTAB_OFFSET + 24, 1, 1, TEXT_VA, 6);
    put_symbol(&mut file, SYMTAB_OFFSET + 48, 6, 1, TEXT_VA + 0x10, 4);

    let strtab = b"\0main\0helper\0";
    file[STRTAB_OFFSET..STRTAB_OFFSET + strtab.len()].copy_from_slice(strtab);
    let shstrtab = b"\0.text\0.symtab\0.strtab\0.shstrtab\0";
    file[SHSTRTAB_OFFSET..SHSTRTAB_OFFSET + shstrtab.len()].copy_from_slice(shstrtab);

    put_section(
        &mut file,
        SHOFF + 64,
        1,
        1,
        0x6,
        TEXT_VA,
        TEXT_OFFSET as u64,
        0x20,
        0,
        0,
        16,
        0,
    );
    put_section(
        &mut file,
        SHOFF + 128,
        7,
        2,
        0,
        0,
        SYMTAB_OFFSET as u64,
        3 * 24,
        3,
        1,
        8,
        24,
    );
    put_section(
        &mut file,
        SHOFF + 192,
        15,
        3,
        0,
        0,
        STRTAB_OFFSET as u64,
        strtab.len() as u64,
        0,
        0,
        1,
        0,
    );
    put_section(
        &mut file,
        SHOFF + 256,
        23,
        3,
        0,
        0,
        SHSTRTAB_OFFSET as u64,
        SHSTRTAB_SIZE as u64,
        0,
        0,
        1,
        0,
    );

    file
}

#[test]
#[ignore = "writes a fixture for manual CLI/GUI smoke tests"]
fn writes_a_fixture_for_manual_smoke_tests() {
    // Tests run with the crate directory as CWD, so write outside the tree.
    let path = std::env::temp_dir().join("recraft-elf-fixture");
    std::fs::write(&path, build_elf()).expect("write fixture");
    eprintln!("wrote {}", path.display());
}

#[test]
fn a_synthetic_elf_binary_analyzes_and_decompiles() {
    let bytes = build_elf();
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("sample.rcraft");

    let mut session = Session::create(&path, &bytes, &mut |_, _| {}, &|| false).expect("create");
    assert_eq!(session.format(), recraft_core::Format::Elf);
    assert_eq!(session.arch(), recraft_core::Arch::X64);
    assert_eq!(session.entry_point().get(), TEXT_VA);

    let result = recraft_analysis::analyze(&mut session).expect("analyze");
    recraft_analysis::persist(&mut session, &result).expect("persist");

    let functions = session.functions().expect("functions");
    assert!(
        functions.iter().any(|f| f.start.get() == TEXT_VA),
        "entry function was not discovered: {functions:?}"
    );

    let name = session.name_at(Address::new(TEXT_VA)).expect("name_at");
    assert_eq!(name.as_deref(), Some("main"));

    let text = pseudocode_l2(&session, Address::new(TEXT_VA)).expect("l2");
    let body: Vec<&str> = text.lines.iter().map(|line| line.text.as_str()).collect();
    let body = body.join("\n");
    assert!(body.contains("main"), "{body}");
    assert!(body.contains("return 1;"), "{body}");

    let report = lint(&mut session, usize::MAX, LintLevel::Both).expect("lint");
    assert!(report.findings.is_empty(), "{:?}", report.findings);
}

// --- ELF32 (i386) ----------------------------------------------------------------

const LOAD32_VA: u64 = 0x0804_7000;
const TEXT32_OFFSET: usize = 0x1000;
const TEXT32_VA: u64 = LOAD32_VA + TEXT32_OFFSET as u64;
const PLT32_OFFSET: usize = 0x1100;
const PLT32_VA: u64 = LOAD32_VA + PLT32_OFFSET as u64;
const GOT32_OFFSET: usize = 0x1200;
const GOT32_VA: u64 = LOAD32_VA + GOT32_OFFSET as u64;
const DYNSYM32_OFFSET: usize = 0x1300;
const DYNSTR32_OFFSET: usize = 0x1400;
const RELPLT32_OFFSET: usize = 0x1500;
const SYMTAB32_OFFSET: usize = 0x1600;
const STRTAB32_OFFSET: usize = 0x1700;
const SHSTRTAB32_OFFSET: usize = 0x1800;
const SHSTRTAB32_SIZE: usize = 72;
const SHOFF32: usize = 0x1900;
const SECTION32_COUNT: usize = 10;
const SECTION32_HEADER_SIZE: usize = 40;

fn put_symbol32(
    file: &mut [u8],
    offset: usize,
    name: u32,
    info: u8,
    shndx: u16,
    value: u32,
    size: u32,
) {
    put_u32(file, offset, name);
    put_u32(file, offset + 4, value);
    put_u32(file, offset + 8, size);
    file[offset + 12] = info;
    put_u16(file, offset + 14, shndx);
}

#[allow(clippy::too_many_arguments)]
fn put_section32(
    file: &mut [u8],
    offset: usize,
    name: u32,
    kind: u32,
    flags: u32,
    address: u32,
    file_offset: u32,
    size: u32,
    link: u32,
    info: u32,
    align: u32,
    entsize: u32,
) {
    put_u32(file, offset, name);
    put_u32(file, offset + 4, kind);
    put_u32(file, offset + 8, flags);
    put_u32(file, offset + 12, address);
    put_u32(file, offset + 16, file_offset);
    put_u32(file, offset + 20, size);
    put_u32(file, offset + 24, link);
    put_u32(file, offset + 28, info);
    put_u32(file, offset + 32, align);
    put_u32(file, offset + 36, entsize);
}

/// A 32-bit i386 executable: `main` adds its two stack arguments and calls
/// `puts@plt`, whose stub jumps through a GOT slot filled by a `SHT_REL` relocation.
fn build_elf32() -> Vec<u8> {
    let mut file = vec![0u8; SHOFF32 + SECTION32_COUNT * SECTION32_HEADER_SIZE];

    file[..4].copy_from_slice(b"\x7fELF");
    file[4] = 1; // ELFCLASS32
    file[5] = 1;
    file[6] = 1;
    put_u16(&mut file, 16, 2); // ET_EXEC
    put_u16(&mut file, 18, 3); // EM_386
    put_u32(&mut file, 20, 1);
    put_u32(&mut file, 24, TEXT32_VA as u32);
    put_u32(&mut file, 28, 52);
    put_u32(&mut file, 32, SHOFF32 as u32);
    put_u16(&mut file, 40, 52);
    put_u16(&mut file, 42, 32);
    put_u16(&mut file, 44, 1);
    put_u16(&mut file, 46, SECTION32_HEADER_SIZE as u16);
    put_u16(&mut file, 48, SECTION32_COUNT as u16);
    put_u16(&mut file, 50, 9);

    let ph = 52;
    put_u32(&mut file, ph, 1);
    put_u32(&mut file, ph + 4, 0);
    put_u32(&mut file, ph + 8, LOAD32_VA as u32);
    put_u32(&mut file, ph + 12, LOAD32_VA as u32);
    put_u32(&mut file, ph + 16, SHOFF32 as u32);
    put_u32(&mut file, ph + 20, SHOFF32 as u32);
    put_u32(&mut file, ph + 24, 5);
    put_u32(&mut file, ph + 28, 0x1000);

    // main: push ebp; mov ebp, esp; mov eax,[ebp+8]; add eax,[ebp+0xc]; pop ebp; ret
    let main = [
        0x55, 0x89, 0xE5, 0x8B, 0x45, 0x08, 0x03, 0x45, 0x0C, 0x5D, 0xC3,
    ];
    file[TEXT32_OFFSET..TEXT32_OFFSET + main.len()].copy_from_slice(&main);

    // .plt: PLT0 then `endbr32; jmp dword ptr [ebx + 4]`.
    file[PLT32_OFFSET..PLT32_OFFSET + 6].copy_from_slice(&[0xFF, 0x25, 0, 0, 0, 0]);
    file[PLT32_OFFSET + 16..PLT32_OFFSET + 26]
        .copy_from_slice(&[0xF3, 0x0F, 0x1E, 0xFB, 0xFF, 0xA3, 0x04, 0x00, 0x00, 0x00]);

    put_symbol32(&mut file, DYNSYM32_OFFSET + 16, 1, 0x12, 0, 0, 0);
    let dynstr = b"\0puts\0";
    file[DYNSTR32_OFFSET..DYNSTR32_OFFSET + dynstr.len()].copy_from_slice(dynstr);

    put_u32(&mut file, RELPLT32_OFFSET, (GOT32_VA + 4) as u32);
    put_u32(&mut file, RELPLT32_OFFSET + 4, (1 << 8) | 7);

    put_symbol32(
        &mut file,
        SYMTAB32_OFFSET + 16,
        1,
        0x12,
        1,
        TEXT32_VA as u32,
        main.len() as u32,
    );
    let strtab = b"\0main\0";
    file[STRTAB32_OFFSET..STRTAB32_OFFSET + strtab.len()].copy_from_slice(strtab);
    let shstrtab =
        b"\0.text\0.plt\0.got.plt\0.dynsym\0.dynstr\0.rel.plt\0.symtab\0.strtab\0.shstrtab\0";
    assert_eq!(shstrtab.len(), SHSTRTAB32_SIZE);
    file[SHSTRTAB32_OFFSET..SHSTRTAB32_OFFSET + shstrtab.len()].copy_from_slice(shstrtab);

    put_section32(
        &mut file,
        SHOFF32 + SECTION32_HEADER_SIZE,
        1,
        1,
        0x6,
        TEXT32_VA as u32,
        TEXT32_OFFSET as u32,
        0x40,
        0,
        0,
        16,
        0,
    );
    put_section32(
        &mut file,
        SHOFF32 + 2 * SECTION32_HEADER_SIZE,
        7,
        1,
        0x6,
        PLT32_VA as u32,
        PLT32_OFFSET as u32,
        0x20,
        0,
        0,
        16,
        16,
    );
    put_section32(
        &mut file,
        SHOFF32 + 3 * SECTION32_HEADER_SIZE,
        12,
        1,
        0x3,
        GOT32_VA as u32,
        GOT32_OFFSET as u32,
        0x10,
        0,
        0,
        4,
        4,
    );
    put_section32(
        &mut file,
        SHOFF32 + 4 * SECTION32_HEADER_SIZE,
        21,
        11,
        0x2,
        (LOAD32_VA + DYNSYM32_OFFSET as u64) as u32,
        DYNSYM32_OFFSET as u32,
        2 * 16,
        5,
        1,
        4,
        16,
    );
    put_section32(
        &mut file,
        SHOFF32 + 5 * SECTION32_HEADER_SIZE,
        29,
        3,
        0x2,
        (LOAD32_VA + DYNSTR32_OFFSET as u64) as u32,
        DYNSTR32_OFFSET as u32,
        dynstr.len() as u32,
        0,
        0,
        1,
        0,
    );
    put_section32(
        &mut file,
        SHOFF32 + 6 * SECTION32_HEADER_SIZE,
        37,
        9,
        0x2,
        (LOAD32_VA + RELPLT32_OFFSET as u64) as u32,
        RELPLT32_OFFSET as u32,
        8,
        4,
        1,
        4,
        8,
    );
    put_section32(
        &mut file,
        SHOFF32 + 7 * SECTION32_HEADER_SIZE,
        46,
        2,
        0,
        0,
        SYMTAB32_OFFSET as u32,
        2 * 16,
        8,
        1,
        4,
        16,
    );
    put_section32(
        &mut file,
        SHOFF32 + 8 * SECTION32_HEADER_SIZE,
        54,
        3,
        0,
        0,
        STRTAB32_OFFSET as u32,
        strtab.len() as u32,
        0,
        0,
        1,
        0,
    );
    put_section32(
        &mut file,
        SHOFF32 + 9 * SECTION32_HEADER_SIZE,
        62,
        3,
        0,
        0,
        SHSTRTAB32_OFFSET as u32,
        SHSTRTAB32_SIZE as u32,
        0,
        0,
        1,
        0,
    );

    file
}

#[test]
fn a_synthetic_elf32_binary_analyzes_and_decompiles() {
    let bytes = build_elf32();
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("sample32.rcraft");

    let mut session = Session::create(&path, &bytes, &mut |_, _| {}, &|| false).expect("create");
    assert_eq!(session.format(), recraft_core::Format::Elf);
    assert_eq!(session.arch(), recraft_core::Arch::X86);
    assert!(!session.image().is_64);
    assert_eq!(session.entry_point().get(), TEXT32_VA);

    // The i386 PLT stub (`jmp [ebx+4]`) resolves to the imported name.
    let stubs = recraft_core::plt::stubs(&session).expect("plt");
    assert_eq!(stubs.len(), 1, "{stubs:?}");
    assert_eq!(stubs[0].name, "puts");
    assert_eq!(stubs[0].slot.get(), GOT32_VA + 4);

    let result = recraft_analysis::analyze(&mut session).expect("analyze");
    recraft_analysis::persist(&mut session, &result).expect("persist");

    let name = session.name_at(Address::new(TEXT32_VA)).expect("name_at");
    assert_eq!(name.as_deref(), Some("main"));

    let text = pseudocode_l2(&session, Address::new(TEXT32_VA)).expect("l2");
    let body = text
        .lines
        .iter()
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(body.contains("__cdecl"), "{body}");
    assert!(body.contains("main(int32 arg1, int32 arg2)"), "{body}");
    assert!(body.contains("arg1"), "{body}");
    assert!(body.contains("arg2"), "{body}");

    let report = lint(&mut session, usize::MAX, LintLevel::Both).expect("lint");
    assert!(report.findings.is_empty(), "{:?}", report.findings);
}
