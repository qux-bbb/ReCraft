// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! ELF64 parsing: a synthetic x86-64 executable carries one loadable text
//! segment, a `.symtab` and the usual string tables. Building the bytes here (like
//! `tests/pe.rs` does for PE) keeps the test independent of a Linux toolchain.

use recraft_format::{Arch, Format};

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

fn write_symbol(
    file: &mut [u8],
    offset: usize,
    name: u32,
    info: u8,
    shndx: u16,
    value: u64,
    size: u64,
) {
    put_u32(file, offset, name);
    file[offset + 4] = info;
    file[offset + 5] = 0;
    put_u16(file, offset + 6, shndx);
    put_u64(file, offset + 8, value);
    put_u64(file, offset + 16, size);
}

#[allow(clippy::too_many_arguments)]
fn write_section(
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

    // --- ELF header ---
    file[..4].copy_from_slice(b"\x7fELF");
    file[4] = 2; // ELFCLASS64
    file[5] = 1; // little endian
    file[6] = 1; // EV_CURRENT
    put_u16(&mut file, 16, 2); // ET_EXEC
    put_u16(&mut file, 18, 0x3e); // EM_X86_64
    put_u32(&mut file, 20, 1);
    put_u64(&mut file, 24, TEXT_VA); // e_entry
    put_u64(&mut file, 32, 0x40); // e_phoff
    put_u64(&mut file, 40, SHOFF as u64); // e_shoff
    put_u16(&mut file, 52, 64); // e_ehsize
    put_u16(&mut file, 54, 56); // e_phentsize
    put_u16(&mut file, 56, 1); // e_phnum
    put_u16(&mut file, 58, 64); // e_shentsize
    put_u16(&mut file, 60, 5); // e_shnum
    put_u16(&mut file, 62, 4); // e_shstrndx

    // --- one PT_LOAD covering .text and the string tables ---
    let loaded_size = (SHSTRTAB_OFFSET + SHSTRTAB_SIZE - TEXT_OFFSET) as u64;
    let ph = 0x40;
    put_u32(&mut file, ph, 1); // PT_LOAD
    put_u32(&mut file, ph + 4, 5); // PF_R | PF_X
    put_u64(&mut file, ph + 8, TEXT_OFFSET as u64);
    put_u64(&mut file, ph + 16, TEXT_VA);
    put_u64(&mut file, ph + 24, TEXT_VA);
    put_u64(&mut file, ph + 32, loaded_size);
    put_u64(&mut file, ph + 40, loaded_size);
    put_u64(&mut file, ph + 48, 0x1000);

    // --- code: main (mov eax,1; ret) and helper (xor eax,eax; ret) ---
    let mut text = [0x90u8; 0x20];
    text[..6].copy_from_slice(&[0xb8, 0x01, 0x00, 0x00, 0x00, 0xc3]);
    text[0x10..0x13].copy_from_slice(&[0x31, 0xc0, 0xc3]);
    file[TEXT_OFFSET..TEXT_OFFSET + text.len()].copy_from_slice(&text);

    // --- .symtab: null, main, helper ---
    write_symbol(&mut file, SYMTAB_OFFSET + 24, 1, 0x12, 1, TEXT_VA, 6);
    write_symbol(&mut file, SYMTAB_OFFSET + 48, 6, 0x12, 1, TEXT_VA + 0x10, 4);

    // --- .strtab / .shstrtab ---
    let strtab = b"\0main\0helper\0";
    file[STRTAB_OFFSET..STRTAB_OFFSET + strtab.len()].copy_from_slice(strtab);
    let shstrtab = b"\0.text\0.symtab\0.strtab\0.shstrtab\0";
    assert_eq!(shstrtab.len(), SHSTRTAB_SIZE);
    file[SHSTRTAB_OFFSET..SHSTRTAB_OFFSET + shstrtab.len()].copy_from_slice(shstrtab);

    // --- section headers ---
    // 0: SHT_NULL (already zeroed)
    write_section(
        &mut file,
        SHOFF + 64,
        1,   // ".text"
        1,   // SHT_PROGBITS
        0x6, // ALLOC | EXECINSTR
        TEXT_VA,
        TEXT_OFFSET as u64,
        0x20,
        0,
        0,
        16,
        0,
    );
    write_section(
        &mut file,
        SHOFF + 128,
        7, // ".symtab"
        2, // SHT_SYMTAB
        0,
        0,
        SYMTAB_OFFSET as u64,
        3 * 24,
        3, // strings: .strtab
        1,
        8,
        24,
    );
    write_section(
        &mut file,
        SHOFF + 192,
        15, // ".strtab"
        3,  // SHT_STRTAB
        0,
        0,
        STRTAB_OFFSET as u64,
        strtab.len() as u64,
        0,
        0,
        1,
        0,
    );
    write_section(
        &mut file,
        SHOFF + 256,
        23, // ".shstrtab"
        3,  // SHT_STRTAB
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
fn a_elf64_executable_yields_sections_exports_and_functions() {
    let bytes = build_elf();
    let loaded = recraft_format::load(&bytes).expect("ELF64 should parse");

    assert_eq!(loaded.format, Format::Elf);
    assert_eq!(loaded.arch, Arch::X64);
    assert_eq!(loaded.image.image_base.get(), 0);
    assert_eq!(loaded.image.entry_point.get(), TEXT_VA);
    assert!(loaded.image.is_64);

    let text = loaded
        .image
        .sections
        .iter()
        .find(|section| section.name == ".text")
        .expect(".text section");
    assert_eq!(text.va.get(), TEXT_VA);
    assert_eq!(text.virtual_size, 0x20);
    assert!(text.is_executable());
    assert!(text.is_readable());

    // Exports describe the dynamic export table, which a static ET_EXEC does not
    // have; the functions come from the symbol sizes instead.
    assert!(loaded.image.exports.is_empty());

    let ranges: Vec<(u64, u64)> = loaded
        .image
        .pdata_functions
        .iter()
        .map(|range| (range.start.get(), range.end.get()))
        .collect();
    assert_eq!(
        ranges,
        [(TEXT_VA, TEXT_VA + 6), (TEXT_VA + 0x10, TEXT_VA + 0x14)]
    );

    assert!(loaded.image.imports.is_empty());
}

#[test]
fn truncated_and_unsupported_inputs_are_errors_not_panics() {
    let bytes = build_elf();

    assert!(recraft_format::load(&bytes[..32]).is_err());

    // A relocatable object is not a linked image.
    let mut relocatable = build_elf();
    put_u16(&mut relocatable, 16, 1); // ET_REL
    let error = recraft_format::load(&relocatable).unwrap_err();
    assert!(error.to_string().contains("ELF type"), "{error}");

    // Truncating the file after the headers must not panic.
    assert!(recraft_format::load(&bytes[..0x70]).is_err());
}

#[test]
fn the_dispatcher_recognises_elf_without_a_hint() {
    let bytes = build_elf();
    assert_eq!(recraft_format::detect_format(&bytes), Some(Format::Elf));
    let loaded = recraft_format::load(&bytes).expect("dispatched to the ELF loader");
    assert_eq!(loaded.format, Format::Elf);
}

// --- a stripped image: function boundaries only from `.eh_frame` -----------------

const EH_FRAME_OFFSET: usize = 0x1080;
const EH_FRAME_VA: u64 = 0x401_080;

fn build_eh_frame() -> Vec<u8> {
    // CIE: version 1, augmentation "zR", code align 1, data align -8, return reg 16,
    // one byte of augmentation data: `DW_EH_PE_pcrel | DW_EH_PE_sdata4`.
    let mut cie = Vec::new();
    cie.extend_from_slice(&0u32.to_le_bytes()); // CIE id
    cie.push(1); // version
    cie.extend_from_slice(b"zR\0");
    cie.push(1); // code alignment
    cie.push(0x78); // data alignment: sleb128(-8)
    cie.push(16); // return address register
    cie.push(1); // augmentation data length
    cie.push(0x1b); // DW_EH_PE_pcrel | DW_EH_PE_sdata4

    let mut fde = Vec::new();
    // The CIE pointer is the distance back from this field to the CIE start; the CIE
    // starts at 0 and this field is at 0x11 + 4, so the value is 0x15.
    fde.extend_from_slice(&0x15u32.to_le_bytes());
    // initial_location is pcrel from the field's own address (0x11 + 8 = 0x19).
    let field_va = EH_FRAME_VA + 0x11 + 8;
    let delta = (TEXT_VA as i64 - field_va as i64) as i32;
    fde.extend_from_slice(&delta.to_le_bytes());
    fde.extend_from_slice(&6i32.to_le_bytes()); // address_range: main's six bytes
    fde.push(0); // augmentation data length

    let mut section = Vec::new();
    section.extend_from_slice(&(cie.len() as u32).to_le_bytes());
    section.extend_from_slice(&cie);
    section.extend_from_slice(&(fde.len() as u32).to_le_bytes());
    section.extend_from_slice(&fde);
    section
}

fn build_stripped_elf() -> Vec<u8> {
    let eh_frame = build_eh_frame();
    let shoff = SHOFF;
    let mut file = vec![0u8; shoff + 4 * 64];

    file[..4].copy_from_slice(b"\x7fELF");
    file[4] = 2;
    file[5] = 1;
    file[6] = 1;
    put_u16(&mut file, 16, 2);
    put_u16(&mut file, 18, 0x3e);
    put_u32(&mut file, 20, 1);
    put_u64(&mut file, 24, TEXT_VA);
    put_u64(&mut file, 32, 0x40);
    put_u64(&mut file, 40, shoff as u64);
    put_u16(&mut file, 52, 64);
    put_u16(&mut file, 54, 56);
    put_u16(&mut file, 56, 1);
    put_u16(&mut file, 58, 64);
    put_u16(&mut file, 60, 4);
    put_u16(&mut file, 62, 3);

    file[TEXT_OFFSET..TEXT_OFFSET + 0x20].copy_from_slice(&[0x90u8; 0x20]);
    file[TEXT_OFFSET..TEXT_OFFSET + 6].copy_from_slice(&[0xb8, 0x01, 0x00, 0x00, 0x00, 0xc3]);
    file[EH_FRAME_OFFSET..EH_FRAME_OFFSET + eh_frame.len()].copy_from_slice(&eh_frame);

    let loaded_size = (EH_FRAME_OFFSET + eh_frame.len() - TEXT_OFFSET) as u64;
    let ph = 0x40;
    put_u32(&mut file, ph, 1);
    put_u32(&mut file, ph + 4, 5);
    put_u64(&mut file, ph + 8, TEXT_OFFSET as u64);
    put_u64(&mut file, ph + 16, TEXT_VA);
    put_u64(&mut file, ph + 24, TEXT_VA);
    put_u64(&mut file, ph + 32, loaded_size);
    put_u64(&mut file, ph + 40, loaded_size);
    put_u64(&mut file, ph + 48, 0x1000);

    let shstrtab = b"\0.text\0.eh_frame\0.shstrtab\0";
    let shstrtab_offset = EH_FRAME_OFFSET + eh_frame.len();
    file[shstrtab_offset..shstrtab_offset + shstrtab.len()].copy_from_slice(shstrtab);

    write_section(
        &mut file,
        shoff + 64,
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
    write_section(
        &mut file,
        shoff + 128,
        7,
        1,
        0x2,
        EH_FRAME_VA,
        EH_FRAME_OFFSET as u64,
        eh_frame.len() as u64,
        0,
        0,
        8,
        0,
    );
    write_section(
        &mut file,
        shoff + 192,
        17,
        3,
        0,
        0,
        shstrtab_offset as u64,
        shstrtab.len() as u64,
        0,
        0,
        1,
        0,
    );

    file
}

#[test]
fn a_stripped_elf_gets_function_ranges_from_eh_frame() {
    let loaded = recraft_format::load(&build_stripped_elf()).expect("stripped ELF should parse");
    assert!(loaded.image.symbols.is_empty());
    let ranges: Vec<(u64, u64)> = loaded
        .image
        .pdata_functions
        .iter()
        .map(|range| (range.start.get(), range.end.get()))
        .collect();
    assert_eq!(ranges, [(TEXT_VA, TEXT_VA + 6)]);
}
