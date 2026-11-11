// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! ELF32 (i386) parsing: a synthetic 32-bit executable carries one loadable segment,
//! a `.symtab`, a `.dynsym` plus a `SHT_REL` `.rel.plt` (the relocation layout i386
//! linkers use instead of x86-64's `SHT_RELA`) and a `.plt` stub. Building the bytes
//! here keeps the test independent of a 32-bit Linux toolchain.

use recraft_format::{Arch, Format};

/// The single `PT_LOAD` maps file offset 0 to this address, so a section's VA is
/// `LOAD_VA + file offset`.
const LOAD_VA: u64 = 0x0804_7000;

const TEXT_OFFSET: usize = 0x1000;
const TEXT_VA: u64 = LOAD_VA + TEXT_OFFSET as u64;
const TEXT_SIZE: usize = 0x40;

const PLT_OFFSET: usize = 0x1100;
const PLT_VA: u64 = LOAD_VA + PLT_OFFSET as u64;

const GOT_OFFSET: usize = 0x1200;
const GOT_VA: u64 = LOAD_VA + GOT_OFFSET as u64;

const DYNSYM_OFFSET: usize = 0x1300;
const DYNSTR_OFFSET: usize = 0x1400;
const RELPLT_OFFSET: usize = 0x1500;
const SYMTAB_OFFSET: usize = 0x1600;
const STRTAB_OFFSET: usize = 0x1700;
const SHSTRTAB_OFFSET: usize = 0x1800;
const SHSTRTAB_SIZE: usize = 72;
const SHOFF: usize = 0x1900;
const SECTION_COUNT: usize = 10;
const SECTION_HEADER_SIZE: usize = 40;

fn put_u16(file: &mut [u8], offset: usize, value: u16) {
    file[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn put_u32(file: &mut [u8], offset: usize, value: u32) {
    file[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

/// One `Elf32_Sym`: 16 bytes, with 32-bit `st_value`/`st_size`.
fn write_symbol(
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
    file[offset + 13] = 0;
    put_u16(file, offset + 14, shndx);
}

/// One `Elf32_Shdr`: 40 bytes, with 32-bit flags/address/offset/size.
#[allow(clippy::too_many_arguments)]
fn write_section(
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

/// The synthetic i386 executable: `main` (adds its two stack arguments), a `puts@plt`
/// stub backed by a `SHT_REL` relocation, and a `.symtab`.
fn build_elf32() -> Vec<u8> {
    let mut file = vec![0u8; SHOFF + SECTION_COUNT * SECTION_HEADER_SIZE];

    // --- ELF header (52 bytes) ---
    file[..4].copy_from_slice(b"\x7fELF");
    file[4] = 1; // ELFCLASS32
    file[5] = 1; // little endian
    file[6] = 1; // EV_CURRENT
    put_u16(&mut file, 16, 2); // ET_EXEC
    put_u16(&mut file, 18, 3); // EM_386
    put_u32(&mut file, 20, 1); // e_version
    put_u32(&mut file, 24, TEXT_VA as u32); // e_entry
    put_u32(&mut file, 28, 52); // e_phoff: right after the header
    put_u32(&mut file, 32, SHOFF as u32); // e_shoff
    put_u32(&mut file, 36, 0); // e_flags
    put_u16(&mut file, 40, 52); // e_ehsize
    put_u16(&mut file, 42, 32); // e_phentsize
    put_u16(&mut file, 44, 1); // e_phnum
    put_u16(&mut file, 46, SECTION_HEADER_SIZE as u16);
    put_u16(&mut file, 48, SECTION_COUNT as u16);
    put_u16(&mut file, 50, 9); // e_shstrndx: .shstrtab

    // --- one PT_LOAD covering file offset 0 up to the section header table ---
    let ph = 52;
    let loaded = SHOFF as u32;
    put_u32(&mut file, ph, 1); // PT_LOAD
    put_u32(&mut file, ph + 4, 0); // p_offset
    put_u32(&mut file, ph + 8, LOAD_VA as u32); // p_vaddr
    put_u32(&mut file, ph + 12, LOAD_VA as u32); // p_paddr
    put_u32(&mut file, ph + 16, loaded); // p_filesz
    put_u32(&mut file, ph + 20, loaded); // p_memsz
    put_u32(&mut file, ph + 24, 5); // PF_R | PF_X
    put_u32(&mut file, ph + 28, 0x1000); // p_align

    // --- code ---
    // main: push ebp; mov ebp, esp; mov eax, [ebp+8]; add eax, [ebp+0xc]; pop ebp; ret
    let main = [
        0x55, 0x89, 0xE5, 0x8B, 0x45, 0x08, 0x03, 0x45, 0x0C, 0x5D, 0xC3,
    ];
    file[TEXT_OFFSET..TEXT_OFFSET + main.len()].copy_from_slice(&main);
    // helper: xor eax, eax; ret
    file[TEXT_OFFSET + 0x20..TEXT_OFFSET + 0x23].copy_from_slice(&[0x31, 0xC0, 0xC3]);

    // --- .plt: PLT0 (16 bytes) then the puts stub: endbr32; jmp dword ptr [ebx+4] ---
    file[PLT_OFFSET..PLT_OFFSET + 6].copy_from_slice(&[0xFF, 0x25, 0x00, 0x00, 0x00, 0x00]);
    let stub = [
        0xF3, 0x0F, 0x1E, 0xFB, // endbr32
        0xFF, 0xA3, 0x04, 0x00, 0x00, 0x00, // jmp dword ptr [ebx + 4]
    ];
    file[PLT_OFFSET + 16..PLT_OFFSET + 16 + stub.len()].copy_from_slice(&stub);

    // --- .dynsym / .dynstr: null, then the undefined `puts` ---
    write_symbol(&mut file, DYNSYM_OFFSET + 16, 1, 0x12, 0, 0, 0);
    let dynstr = b"\0puts\0";
    file[DYNSTR_OFFSET..DYNSTR_OFFSET + dynstr.len()].copy_from_slice(dynstr);

    // --- .rel.plt: one SHT_REL entry, JUMP_SLOT against `puts` in the second GOT slot ---
    put_u32(&mut file, RELPLT_OFFSET, (GOT_VA + 4) as u32); // r_offset
    put_u32(&mut file, RELPLT_OFFSET + 4, (1 << 8) | 7); // r_info: sym 1, R_386_JMP_SLOT

    // --- .symtab / .strtab: null, main, helper ---
    write_symbol(
        &mut file,
        SYMTAB_OFFSET + 16,
        1,
        0x12,
        1,
        TEXT_VA as u32,
        main.len() as u32,
    );
    write_symbol(
        &mut file,
        SYMTAB_OFFSET + 32,
        6,
        0x12,
        1,
        (TEXT_VA + 0x20) as u32,
        3,
    );
    let strtab = b"\0main\0helper\0";
    file[STRTAB_OFFSET..STRTAB_OFFSET + strtab.len()].copy_from_slice(strtab);

    let shstrtab =
        b"\0.text\0.plt\0.got.plt\0.dynsym\0.dynstr\0.rel.plt\0.symtab\0.strtab\0.shstrtab\0";
    assert_eq!(shstrtab.len(), SHSTRTAB_SIZE);
    file[SHSTRTAB_OFFSET..SHSTRTAB_OFFSET + shstrtab.len()].copy_from_slice(shstrtab);

    // --- section headers ---
    write_section(
        &mut file,
        SHOFF + SECTION_HEADER_SIZE,
        1,   // ".text"
        1,   // SHT_PROGBITS
        0x6, // ALLOC | EXECINSTR
        TEXT_VA as u32,
        TEXT_OFFSET as u32,
        TEXT_SIZE as u32,
        0,
        0,
        16,
        0,
    );
    write_section(
        &mut file,
        SHOFF + 2 * SECTION_HEADER_SIZE,
        7, // ".plt"
        1,
        0x6,
        PLT_VA as u32,
        PLT_OFFSET as u32,
        0x20,
        0,
        0,
        16,
        16,
    );
    write_section(
        &mut file,
        SHOFF + 3 * SECTION_HEADER_SIZE,
        12, // ".got.plt"
        1,
        0x3, // ALLOC | WRITE
        GOT_VA as u32,
        GOT_OFFSET as u32,
        0x10,
        0,
        0,
        4,
        4,
    );
    write_section(
        &mut file,
        SHOFF + 4 * SECTION_HEADER_SIZE,
        21, // ".dynsym"
        11, // SHT_DYNSYM
        0x2,
        (LOAD_VA + DYNSYM_OFFSET as u64) as u32,
        DYNSYM_OFFSET as u32,
        2 * 16,
        5, // .dynstr
        1,
        4,
        16,
    );
    write_section(
        &mut file,
        SHOFF + 5 * SECTION_HEADER_SIZE,
        29, // ".dynstr"
        3,
        0x2,
        (LOAD_VA + DYNSTR_OFFSET as u64) as u32,
        DYNSTR_OFFSET as u32,
        dynstr.len() as u32,
        0,
        0,
        1,
        0,
    );
    write_section(
        &mut file,
        SHOFF + 6 * SECTION_HEADER_SIZE,
        37, // ".rel.plt"
        9,  // SHT_REL
        0x2,
        (LOAD_VA + RELPLT_OFFSET as u64) as u32,
        RELPLT_OFFSET as u32,
        8,
        4, // .dynsym
        1,
        4,
        8,
    );
    write_section(
        &mut file,
        SHOFF + 7 * SECTION_HEADER_SIZE,
        46, // ".symtab"
        2,
        0,
        0,
        SYMTAB_OFFSET as u32,
        3 * 16,
        8, // .strtab
        1,
        4,
        16,
    );
    write_section(
        &mut file,
        SHOFF + 8 * SECTION_HEADER_SIZE,
        54, // ".strtab"
        3,
        0,
        0,
        STRTAB_OFFSET as u32,
        strtab.len() as u32,
        0,
        0,
        1,
        0,
    );
    write_section(
        &mut file,
        SHOFF + 9 * SECTION_HEADER_SIZE,
        62, // ".shstrtab"
        3,
        0,
        0,
        SHSTRTAB_OFFSET as u32,
        SHSTRTAB_SIZE as u32,
        0,
        0,
        1,
        0,
    );

    file
}

#[test]
fn a_32_bit_executable_parses_with_symbols_and_rel_relocations() {
    let bytes = build_elf32();
    let loaded = recraft_format::load(&bytes).expect("ELF32 should parse");

    assert_eq!(loaded.format, Format::Elf);
    assert_eq!(loaded.arch, Arch::X86);
    assert!(!loaded.image.is_64);
    assert_eq!(loaded.image.entry_point.get(), TEXT_VA);
    assert_eq!(loaded.image.image_base.get(), 0);

    let text = loaded
        .image
        .sections
        .iter()
        .find(|section| section.name == ".text")
        .expect(".text section");
    assert_eq!(text.va.get(), TEXT_VA);
    assert_eq!(text.virtual_size, TEXT_SIZE as u64);
    assert!(text.is_executable());

    // The import comes from `.rel.plt` (SHT_REL, 8-byte entries, symbol index in the
    // high 24 bits of r_info) and lands in the GOT slot named by r_offset.
    assert_eq!(loaded.image.imports.len(), 1, "{:?}", loaded.image.imports);
    let module = &loaded.image.imports[0];
    assert_eq!(module.functions.len(), 1);
    assert_eq!(module.functions[0].name.as_deref(), Some("puts"));
    assert_eq!(module.functions[0].iat_va.get(), GOT_VA + 4);

    // Function ranges come from the 32-bit symbol sizes.
    let ranges: Vec<(u64, u64)> = loaded
        .image
        .pdata_functions
        .iter()
        .map(|range| (range.start.get(), range.end.get()))
        .collect();
    assert_eq!(
        ranges,
        [(TEXT_VA, TEXT_VA + 11), (TEXT_VA + 0x20, TEXT_VA + 0x23)]
    );

    let names: Vec<(&str, u64)> = loaded
        .image
        .symbols
        .iter()
        .map(|symbol| (symbol.name.as_str(), symbol.address.get()))
        .collect();
    assert_eq!(names, [("main", TEXT_VA), ("helper", TEXT_VA + 0x20)]);
}

#[test]
fn the_dispatcher_recognises_elf32_without_a_hint() {
    let bytes = build_elf32();
    assert_eq!(recraft_format::detect_format(&bytes), Some(Format::Elf));
    let loaded = recraft_format::load(&bytes).expect("dispatched to the ELF loader");
    assert_eq!(loaded.arch, Arch::X86);
}

#[test]
fn a_class_and_machine_mismatch_is_rejected() {
    let mut bytes = build_elf32();
    put_u16(&mut bytes, 18, 0x3e); // EM_X86_64 in an ELF32 header
    let error = recraft_format::load(&bytes).unwrap_err();
    assert!(error.to_string().contains("class and machine"), "{error}");

    // A 64-bit x86 header that claims ELF32 class must fail the same way.
    let mut wrong = build_elf32();
    wrong[4] = 2; // ELFCLASS64
    let error = recraft_format::load(&wrong).unwrap_err();
    assert!(error.to_string().contains("class and machine"), "{error}");
}

#[test]
fn truncated_32_bit_input_is_an_error_not_a_panic() {
    let bytes = build_elf32();
    assert!(recraft_format::load(&bytes[..40]).is_err());
    assert!(recraft_format::load(&bytes[..0x60]).is_err());
}
