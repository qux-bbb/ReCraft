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

    let text = pseudocode_l2(&mut session, Address::new(TEXT_VA)).expect("l2");
    let body: Vec<&str> = text.lines.iter().map(|line| line.text.as_str()).collect();
    let body = body.join("\n");
    assert!(body.contains("main"), "{body}");
    assert!(body.contains("return 1;"), "{body}");

    let report = lint(&mut session, usize::MAX, LintLevel::Both).expect("lint");
    assert!(report.findings.is_empty(), "{:?}", report.findings);
}
