// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! ELF64 loader.
//!
//! Scope: little-endian, 64-bit, x86-64 executables and shared objects (`ET_EXEC` /
//! `ET_DYN`). ARM64 images parse but are rejected for now, because the decoder is
//! still iced-x86; `aarch64` support lands with the ARM64 lifter.
//!
//! What a project needs from the container is the same vocabulary the PE loader
//! produces: sections, imports (here: `R_X86_64_JUMP_SLOT`/`GLOB_DAT` relocations
//! against the GOT), exports (defined `STT_FUNC` symbols), function ranges (symbol
//! sizes plus `.eh_frame` FDEs) and the entry point. Everything is bounds-checked
//! and non-fatal problems become [`Image::warnings`], like the PE loader.

use std::collections::{BTreeMap, HashMap};

use recraft_common::{Address, Error, Result};

use crate::{
    Arch, Export, Format, FunctionRange, Image, ImportFunction, ImportModule, LoadedImage, Loader,
    Section, SymbolName,
};

const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];
const ELFCLASS64: u8 = 2;
const ELFDATA2LSB: u8 = 1;
const EV_CURRENT: u8 = 1;

const ET_EXEC: u16 = 2;
const ET_DYN: u16 = 3;

const EM_X86_64: u16 = 0x3e;
const EM_AARCH64: u16 = 0xb7;

const PT_LOAD: u32 = 1;
const PT_DYNAMIC: u32 = 2;

const SHT_PROGBITS: u32 = 1;
const SHT_SYMTAB: u32 = 2;
const SHT_STRTAB: u32 = 3;
const SHT_RELA: u32 = 4;
const SHT_NOBITS: u32 = 8;
const SHT_DYNSYM: u32 = 11;
const SHT_DYNAMIC: u32 = 6;

const SHF_WRITE: u64 = 0x1;
const SHF_ALLOC: u64 = 0x2;
const SHF_EXECINSTR: u64 = 0x4;

const STT_FUNC: u8 = 2;
const STB_GLOBAL: u8 = 1;
const STB_WEAK: u8 = 2;

const DT_NULL: u64 = 0;
const DT_NEEDED: u64 = 1;
const DT_STRTAB: u64 = 5;
const DT_STRSZ: u64 = 10;
const DT_SONAME: u64 = 14;

const R_X86_64_64: u32 = 1;
const R_X86_64_GLOB_DAT: u32 = 6;
const R_X86_64_JUMP_SLOT: u32 = 7;

/// Synthetic PE section flags, so the rest of the pipeline can keep asking
/// `Section::is_executable()`/`is_writable()` for any container.
const IMAGE_SCN_MEM_EXECUTE: u32 = 0x2000_0000;
const IMAGE_SCN_MEM_READ: u32 = 0x4000_0000;
const IMAGE_SCN_MEM_WRITE: u32 = 0x8000_0000;

const MAX_SECTIONS: usize = 4096;
const MAX_SEGMENTS: usize = 1024;
const MAX_SYMBOLS: usize = 500_000;
const MAX_RELATIONS: usize = 500_000;
const MAX_EH_FRAME_RECORDS: usize = 200_000;
const MAX_FUNCTION_BYTES: u64 = 16 << 20;
const MAX_NAME_LEN: usize = 512;

#[derive(Debug, Clone, Copy)]
struct Header {
    e_type: u16,
    e_machine: u16,
    e_entry: u64,
    e_phoff: u64,
    e_shoff: u64,
    e_phentsize: u16,
    e_phnum: u16,
    e_shentsize: u16,
    e_shnum: u16,
    e_shstrndx: u16,
}

#[derive(Debug, Clone, Copy)]
struct ProgramHeader {
    p_type: u32,
    p_offset: u64,
    p_vaddr: u64,
    p_filesz: u64,
    p_memsz: u64,
}

#[derive(Debug, Clone, Copy)]
struct SectionHeader {
    name_offset: u32,
    sh_type: u32,
    sh_flags: u64,
    sh_addr: u64,
    sh_offset: u64,
    sh_size: u64,
    sh_link: u32,
}

#[derive(Debug, Clone)]
struct Symbol {
    name: String,
    value: u64,
    size: u64,
    info: u8,
    shndx: u16,
}

/// The ELF loader.
pub struct ElfLoader;

impl Loader for ElfLoader {
    const NAME: &'static str = "ELF";
    const FORMAT: Format = Format::Elf;

    fn detect(bytes: &[u8]) -> bool {
        detect(bytes)
    }

    fn load(bytes: &[u8]) -> Result<LoadedImage> {
        load(bytes)
    }
}

/// Return `true` if `bytes` looks like a 64-bit little-endian ELF image.
pub fn detect(bytes: &[u8]) -> bool {
    bytes.len() >= 64
        && bytes[..4] == ELF_MAGIC
        && bytes[4] == ELFCLASS64
        && bytes[5] == ELFDATA2LSB
}

/// Parse an ELF64 image.
pub fn load(bytes: &[u8]) -> Result<LoadedImage> {
    let header = parse_header(bytes)?;
    if header.e_type != ET_EXEC && header.e_type != ET_DYN {
        return Err(Error::format(format!(
            "ELF type {} is not a linked executable (ET_EXEC/ET_DYN only)",
            header.e_type
        )));
    }
    let (arch, is_64) = match header.e_machine {
        EM_X86_64 => (Arch::X64, true),
        EM_AARCH64 => (Arch::Aarch64, true),
        other => {
            return Err(Error::format(format!(
                "unsupported ELF machine {other:#x} (x86-64 today; arm64 with the ARM64 lifter)"
            )));
        }
    };
    if arch != Arch::X64 {
        return Err(Error::format(
            "ELF arm64 images parse, but the decoder is x86-only until the ARM64 lifter lands",
        ));
    }

    let program_headers = parse_program_headers(bytes, &header)?;
    let section_headers = parse_section_headers(bytes, &header)?;
    let names = section_names(bytes, &header, &section_headers);

    let mut image = Image {
        is_64,
        image_base: Address::new(0),
        entry_point: Address::new(header.e_entry),
        file_size: bytes.len() as u64,
        ..Image::default()
    };

    // Sections carry the address space for this loader; program headers only say
    // which file ranges are mapped.
    for (index, section) in section_headers.iter().enumerate() {
        if section.sh_flags & SHF_ALLOC == 0 || section.sh_size == 0 {
            continue;
        }
        let name = names.get(index).cloned().unwrap_or_default();
        let executable = section.sh_flags & SHF_EXECINSTR != 0;
        let writable = section.sh_flags & SHF_WRITE != 0;
        let mut characteristics = IMAGE_SCN_MEM_READ;
        if executable {
            characteristics |= IMAGE_SCN_MEM_EXECUTE;
        }
        if writable {
            characteristics |= IMAGE_SCN_MEM_WRITE;
        }
        let raw_size = if section.sh_type == SHT_NOBITS {
            0
        } else {
            section.sh_size
        };
        image.sections.push(Section {
            name,
            va: Address::new(section.sh_addr),
            virtual_size: section.sh_size,
            raw_offset: u32::try_from(section.sh_offset).unwrap_or(u32::MAX),
            raw_size: u32::try_from(raw_size).unwrap_or(u32::MAX),
            characteristics,
            rva: u32::try_from(section.sh_addr).unwrap_or(u32::MAX),
        });
    }
    image.sections.sort_by_key(|section| section.va.get());

    let image_end = program_headers
        .iter()
        .filter(|header| header.p_type == PT_LOAD)
        .map(|header| header.p_vaddr.saturating_add(header.p_memsz))
        .max()
        .unwrap_or_else(|| {
            image
                .sections
                .iter()
                .map(|section| section.va.get().saturating_add(section.virtual_size))
                .max()
                .unwrap_or(0)
        });
    image.size_of_image = image_end;

    let dynamic = dynamic_entries(bytes, &header, &section_headers, &program_headers);
    let dynstr = dynamic_string_table(bytes, &section_headers, &dynamic, &program_headers, &names);

    // Symbols: `.dynsym` for the dynamic symbol table, `.symtab` when not stripped.
    let dynsyms = symbol_table(
        bytes,
        &section_headers,
        &names,
        SHT_DYNSYM,
        &mut image.warnings,
    );
    let symtab = symbol_table(
        bytes,
        &section_headers,
        &names,
        SHT_SYMTAB,
        &mut image.warnings,
    );

    image.imports = imports(bytes, &section_headers, &names, &dynsyms, &dynstr, &dynamic);
    image.exports = exports(&dynsyms);

    let mut ranges: BTreeMap<u64, u64> = BTreeMap::new();
    for table in [&dynsyms, &symtab] {
        for symbol in table.iter() {
            if symbol.shndx == 0 || symbol.size == 0 {
                continue;
            }
            if symbol.info & 0xf != STT_FUNC {
                continue;
            }
            if symbol.size > MAX_FUNCTION_BYTES {
                continue;
            }
            let end = symbol.value.saturating_add(symbol.size);
            ranges.insert(symbol.value, end);
        }
    }
    let exec_sections: Vec<(u64, u64)> = section_headers
        .iter()
        .zip(names.iter())
        .filter(|(section, _)| section.sh_flags & SHF_EXECINSTR != 0 && section.sh_size > 0)
        .map(|(section, _)| {
            (
                section.sh_addr,
                section.sh_addr.saturating_add(section.sh_size),
            )
        })
        .collect();
    for (start, end) in eh_frame_functions(bytes, &section_headers, &names) {
        if exec_sections
            .iter()
            .any(|(low, high)| start >= *low && start < *high)
            && end > start
            && end - start <= MAX_FUNCTION_BYTES
        {
            ranges.entry(start).or_insert(end);
        }
    }
    image.symbols = defined_symbols(&dynsyms, &symtab);
    image.pdata_functions = ranges
        .into_iter()
        .map(|(start, end)| FunctionRange {
            start: Address::new(start),
            end: Address::new(end),
        })
        .collect();
    image.debug_lines = crate::dwarf::parse_line_table(
        &crate::dwarf::DwarfSections {
            debug_line: section_data(bytes, &section_headers, &names, ".debug_line"),
            debug_line_str: section_data(bytes, &section_headers, &names, ".debug_line_str"),
            debug_str: section_data(bytes, &section_headers, &names, ".debug_str"),
        },
        8,
    );

    if image.entry_point.get() == 0 {
        image.warnings.push("ELF has no entry point".to_owned());
    }

    Ok(LoadedImage {
        image,
        format: Format::Elf,
        arch,
    })
}

// ------------------------------------------------------------------ readers --

fn read_u8(bytes: &[u8], offset: u64) -> Result<u8> {
    let offset = usize::try_from(offset).map_err(|_| Error::format("ELF offset overflow"))?;
    bytes
        .get(offset)
        .copied()
        .ok_or_else(|| Error::format("ELF read past end of file"))
}

fn read_u16(bytes: &[u8], offset: u64) -> Result<u16> {
    let start = usize::try_from(offset).map_err(|_| Error::format("ELF offset overflow"))?;
    let slice = bytes
        .get(start..start + 2)
        .ok_or_else(|| Error::format("ELF read past end of file"))?;
    Ok(u16::from_le_bytes([slice[0], slice[1]]))
}

fn read_u32(bytes: &[u8], offset: u64) -> Result<u32> {
    let start = usize::try_from(offset).map_err(|_| Error::format("ELF offset overflow"))?;
    let slice = bytes
        .get(start..start + 4)
        .ok_or_else(|| Error::format("ELF read past end of file"))?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn read_u64(bytes: &[u8], offset: u64) -> Result<u64> {
    let start = usize::try_from(offset).map_err(|_| Error::format("ELF offset overflow"))?;
    let slice = bytes
        .get(start..start + 8)
        .ok_or_else(|| Error::format("ELF read past end of file"))?;
    let mut buffer = [0u8; 8];
    buffer.copy_from_slice(slice);
    Ok(u64::from_le_bytes(buffer))
}

fn read_string(bytes: &[u8], offset: u64) -> Option<String> {
    let start = usize::try_from(offset).ok()?;
    let slice = bytes.get(start..)?;
    let end = slice.iter().position(|byte| *byte == 0)?;
    if end > MAX_NAME_LEN {
        return None;
    }
    Some(String::from_utf8_lossy(&slice[..end]).into_owned())
}

// ------------------------------------------------------------------- header --

fn parse_header(bytes: &[u8]) -> Result<Header> {
    if bytes.len() < 64 {
        return Err(Error::format("ELF header is truncated"));
    }
    if bytes[..4] != ELF_MAGIC {
        return Err(Error::format("not an ELF image"));
    }
    if bytes[4] != ELFCLASS64 {
        return Err(Error::format(
            "only ELF64 is supported (ELF32 is a later format)",
        ));
    }
    if bytes[5] != ELFDATA2LSB {
        return Err(Error::format("only little-endian ELF is supported"));
    }
    if bytes[6] != EV_CURRENT {
        return Err(Error::format("unsupported ELF version"));
    }
    Ok(Header {
        e_type: read_u16(bytes, 16)?,
        e_machine: read_u16(bytes, 18)?,
        e_entry: read_u64(bytes, 24)?,
        e_phoff: read_u64(bytes, 32)?,
        e_shoff: read_u64(bytes, 40)?,
        e_phentsize: read_u16(bytes, 54)?,
        e_phnum: read_u16(bytes, 56)?,
        e_shentsize: read_u16(bytes, 58)?,
        e_shnum: read_u16(bytes, 60)?,
        e_shstrndx: read_u16(bytes, 62)?,
    })
}

fn parse_program_headers(bytes: &[u8], header: &Header) -> Result<Vec<ProgramHeader>> {
    let count = usize::from(header.e_phnum).min(MAX_SEGMENTS);
    let size = u64::from(header.e_phentsize);
    if count == 0 {
        return Ok(Vec::new());
    }
    if size < 56 {
        return Err(Error::format("ELF program header entry is too small"));
    }
    let mut headers = Vec::with_capacity(count);
    for index in 0..count {
        let base = header.e_phoff + (index as u64) * size;
        headers.push(ProgramHeader {
            p_type: read_u32(bytes, base)?,
            p_offset: read_u64(bytes, base + 8)?,
            p_vaddr: read_u64(bytes, base + 16)?,
            p_filesz: read_u64(bytes, base + 32)?,
            p_memsz: read_u64(bytes, base + 40)?,
        });
    }
    Ok(headers)
}

fn parse_section_headers(bytes: &[u8], header: &Header) -> Result<Vec<SectionHeader>> {
    let count = usize::from(header.e_shnum).min(MAX_SECTIONS);
    let size = u64::from(header.e_shentsize);
    if count == 0 {
        return Ok(Vec::new());
    }
    if size < 64 {
        return Err(Error::format("ELF section header entry is too small"));
    }
    let mut headers = Vec::with_capacity(count);
    for index in 0..count {
        let base = header.e_shoff + (index as u64) * size;
        headers.push(SectionHeader {
            name_offset: read_u32(bytes, base)?,
            sh_type: read_u32(bytes, base + 4)?,
            sh_flags: read_u64(bytes, base + 8)?,
            sh_addr: read_u64(bytes, base + 16)?,
            sh_offset: read_u64(bytes, base + 24)?,
            sh_size: read_u64(bytes, base + 32)?,
            sh_link: read_u32(bytes, base + 40)?,
        });
    }
    Ok(headers)
}

fn section_names(bytes: &[u8], header: &Header, sections: &[SectionHeader]) -> Vec<String> {
    let index = usize::from(header.e_shstrndx);
    let Some(strings) = sections.get(index) else {
        return sections.iter().map(|_| String::new()).collect();
    };
    let start = match usize::try_from(strings.sh_offset) {
        Ok(start) => start,
        Err(_) => return sections.iter().map(|_| String::new()).collect(),
    };
    let end = start.saturating_add(usize::try_from(strings.sh_size).unwrap_or(0));
    let Some(table) = bytes.get(start..end.min(bytes.len())) else {
        return sections.iter().map(|_| String::new()).collect();
    };
    sections
        .iter()
        .map(|section| {
            let offset = u64::from(section.name_offset);
            let slice = table.get(usize::try_from(offset).unwrap_or(usize::MAX)..);
            slice
                .and_then(|slice| {
                    slice
                        .iter()
                        .position(|byte| *byte == 0)
                        .map(|end| &slice[..end])
                })
                .map(|name| String::from_utf8_lossy(name).into_owned())
                .unwrap_or_default()
        })
        .collect()
}

// ------------------------------------------------------------------ dynamic --

struct DynamicEntry {
    tag: u64,
    value: u64,
}

fn dynamic_entries(
    bytes: &[u8],
    _header: &Header,
    sections: &[SectionHeader],
    program_headers: &[ProgramHeader],
) -> Vec<DynamicEntry> {
    let (offset, size) = if let Some(section) = sections.iter().find(|s| s.sh_type == SHT_DYNAMIC) {
        (section.sh_offset, section.sh_size)
    } else if let Some(segment) = program_headers.iter().find(|p| p.p_type == PT_DYNAMIC) {
        (segment.p_offset, segment.p_filesz)
    } else {
        return Vec::new();
    };
    let mut entries = Vec::new();
    let mut cursor = offset;
    let end = offset.saturating_add(size);
    while cursor + 16 <= end {
        let Ok(tag) = read_u64(bytes, cursor) else {
            break;
        };
        let Ok(value) = read_u64(bytes, cursor + 8) else {
            break;
        };
        if tag == DT_NULL {
            break;
        }
        if entries.len() >= 4096 {
            break;
        }
        entries.push(DynamicEntry { tag, value });
        cursor += 16;
    }
    entries
}

/// The dynamic string table, either from `.dynstr` or from `DT_STRTAB`.
fn dynamic_string_table(
    bytes: &[u8],
    sections: &[SectionHeader],
    dynamic: &[DynamicEntry],
    program_headers: &[ProgramHeader],
    names: &[String],
) -> Vec<u8> {
    if let Some(section) = sections
        .iter()
        .zip(names.iter())
        .find(|(section, name)| section.sh_type == SHT_STRTAB && *name == ".dynstr")
        .map(|(section, _)| section)
    {
        if let (Ok(start), Ok(size)) = (
            usize::try_from(section.sh_offset),
            usize::try_from(section.sh_size),
        ) {
            if let Some(slice) = bytes.get(start..start.saturating_add(size).min(bytes.len())) {
                return slice.to_vec();
            }
        }
    }
    let address = dynamic
        .iter()
        .find(|entry| entry.tag == DT_STRTAB)
        .map(|entry| entry.value);
    let size = dynamic
        .iter()
        .find(|entry| entry.tag == DT_STRSZ)
        .map(|entry| entry.value)
        .unwrap_or(0);
    let Some(address) = address else {
        return Vec::new();
    };
    let Some(offset) = va_to_offset(program_headers, address) else {
        return Vec::new();
    };
    let start = usize::try_from(offset).unwrap_or(usize::MAX);
    let end = start
        .saturating_add(usize::try_from(size).unwrap_or(0))
        .min(bytes.len());
    bytes
        .get(start..end)
        .map(|slice| slice.to_vec())
        .unwrap_or_default()
}

fn va_to_offset(program_headers: &[ProgramHeader], va: u64) -> Option<u64> {
    program_headers
        .iter()
        .filter(|header| header.p_type == PT_LOAD)
        .find(|header| {
            va >= header.p_vaddr && va - header.p_vaddr < header.p_filesz.max(header.p_memsz)
        })
        .map(|header| header.p_offset + (va - header.p_vaddr))
}

// ------------------------------------------------------------------ symbols --

fn symbol_table(
    bytes: &[u8],
    sections: &[SectionHeader],
    names: &[String],
    table_type: u32,
    warnings: &mut Vec<String>,
) -> Vec<Symbol> {
    let Some((table, name)) = sections
        .iter()
        .zip(names.iter())
        .find(|(section, _)| section.sh_type == table_type)
    else {
        return Vec::new();
    };
    let Some(strings) = sections.get(table.sh_link as usize) else {
        warnings.push(format!("{name} has no linked string table"));
        return Vec::new();
    };
    let (Ok(strings_start), Ok(strings_size)) = (
        usize::try_from(strings.sh_offset),
        usize::try_from(strings.sh_size),
    ) else {
        return Vec::new();
    };
    let Some(stringbytes) =
        bytes.get(strings_start..strings_start.saturating_add(strings_size).min(bytes.len()))
    else {
        return Vec::new();
    };

    let entry_size = 24u64;
    let count = (table.sh_size / entry_size).min(MAX_SYMBOLS as u64) as usize;
    let mut symbols = Vec::with_capacity(count);
    for index in 0..count {
        let base = table.sh_offset + (index as u64) * entry_size;
        let Ok(name_offset) = read_u32(bytes, base) else {
            break;
        };
        let Ok(info) = read_u8(bytes, base + 4) else {
            break;
        };
        let Ok(shndx) = read_u16(bytes, base + 6) else {
            break;
        };
        let Ok(value) = read_u64(bytes, base + 8) else {
            break;
        };
        let Ok(size) = read_u64(bytes, base + 16) else {
            break;
        };
        let name = read_string(stringbytes, u64::from(name_offset)).unwrap_or_default();
        symbols.push(Symbol {
            name,
            value,
            size,
            info,
            shndx,
        });
    }
    symbols
}

// ------------------------------------------------------------------ imports --

fn imports(
    bytes: &[u8],
    sections: &[SectionHeader],
    names: &[String],
    symbols: &[Symbol],
    dynstr: &[u8],
    dynamic: &[DynamicEntry],
) -> Vec<ImportModule> {
    let mut slots: BTreeMap<u64, String> = BTreeMap::new();
    for (section, name) in sections.iter().zip(names.iter()) {
        if section.sh_type != SHT_RELA {
            continue;
        }
        if !name.contains("rela") {
            continue;
        }
        let Ok(offset) = usize::try_from(section.sh_offset) else {
            continue;
        };
        let Ok(size) = usize::try_from(section.sh_size) else {
            continue;
        };
        if section.sh_size % 24 != 0 {
            continue;
        }
        let count = (size / 24).min(MAX_RELATIONS);
        for index in 0..count {
            let entry = match bytes.get(offset + index * 24..offset + index * 24 + 24) {
                Some(entry) => entry,
                None => break,
            };
            let mut offsetbytes = [0u8; 8];
            offsetbytes.copy_from_slice(&entry[0..8]);
            let mut infobytes = [0u8; 8];
            infobytes.copy_from_slice(&entry[8..16]);
            let r_offset = u64::from_le_bytes(offsetbytes);
            let r_info = u64::from_le_bytes(infobytes);
            let kind = (r_info & 0xffff_ffff) as u32;
            let symbol_index = (r_info >> 32) as usize;
            if !matches!(kind, R_X86_64_JUMP_SLOT | R_X86_64_GLOB_DAT | R_X86_64_64) {
                continue;
            }
            let Some(symbol) = symbols.get(symbol_index) else {
                continue;
            };
            if symbol.shndx != 0 || symbol.name.is_empty() {
                continue;
            }
            slots.insert(r_offset, symbol.name.clone());
        }
    }
    if slots.is_empty() {
        return Vec::new();
    }

    let module_name = module_name(dynstr, dynamic, symbols);
    let functions = slots
        .into_iter()
        .map(|(iat_va, name)| ImportFunction {
            name: Some(name),
            ordinal: None,
            hint: None,
            iat_va: Address::new(iat_va),
        })
        .collect();
    vec![ImportModule {
        name: module_name,
        iat_va: Address::new(0),
        delayed: false,
        functions,
    }]
}

/// Best effort: the SONAME when the image has one, else the single `DT_NEEDED`,
/// else a neutral name (ELF does not attribute symbols to libraries without
/// version tables).
fn module_name(dynstr: &[u8], dynamic: &[DynamicEntry], _symbols: &[Symbol]) -> String {
    let soname = dynamic
        .iter()
        .find(|entry| entry.tag == DT_SONAME)
        .and_then(|entry| read_string(dynstr, entry.value));
    if let Some(name) = soname {
        return name;
    }
    let needed: Vec<String> = dynamic
        .iter()
        .filter(|entry| entry.tag == DT_NEEDED)
        .filter_map(|entry| read_string(dynstr, entry.value))
        .collect();
    if needed.len() == 1 {
        return needed[0].clone();
    }
    "elf".to_owned()
}

fn exports(symbols: &[Symbol]) -> Vec<Export> {
    let mut exports = Vec::new();
    for (index, symbol) in symbols.iter().enumerate() {
        if symbol.shndx == 0 || symbol.name.is_empty() {
            continue;
        }
        let binding = symbol.info >> 4;
        if binding != STB_GLOBAL && binding != STB_WEAK {
            continue;
        }
        if symbol.info & 0xf != STT_FUNC {
            continue;
        }
        exports.push(Export {
            name: Some(symbol.name.clone()),
            ordinal: index as u32,
            rva: u32::try_from(symbol.value).unwrap_or(u32::MAX),
            forwarder: None,
        });
    }
    exports
}

/// Names the container carries: defined `STT_FUNC` symbols from both tables,
/// deduplicated by address (a dynamic symbol wins over the static one).
fn defined_symbols(dynsyms: &[Symbol], symtab: &[Symbol]) -> Vec<SymbolName> {
    let mut by_address: BTreeMap<u64, SymbolName> = BTreeMap::new();
    for table in [symtab, dynsyms] {
        for symbol in table {
            if symbol.shndx == 0 || symbol.name.is_empty() || symbol.value == 0 {
                continue;
            }
            if symbol.info & 0xf != STT_FUNC {
                continue;
            }
            by_address.insert(
                symbol.value,
                SymbolName {
                    address: Address::new(symbol.value),
                    name: symbol.name.clone(),
                    size: symbol.size,
                },
            );
        }
    }
    by_address.into_values().collect()
}

// ----------------------------------------------------------------- .eh_frame --

/// Function ranges from `.eh_frame` FDEs. Stripped binaries carry no symbols, and
/// the unwind tables are then the only exact function boundaries.
fn eh_frame_functions(
    bytes: &[u8],
    sections: &[SectionHeader],
    names: &[String],
) -> Vec<(u64, u64)> {
    let Some((section, _)) = sections
        .iter()
        .zip(names.iter())
        .find(|(section, name)| *name == ".eh_frame" && section.sh_type == SHT_PROGBITS)
    else {
        return Vec::new();
    };
    let (Ok(start), Ok(size)) = (
        usize::try_from(section.sh_offset),
        usize::try_from(section.sh_size),
    ) else {
        return Vec::new();
    };
    let Some(data) = bytes.get(start..start.saturating_add(size).min(bytes.len())) else {
        return Vec::new();
    };
    parse_eh_frame(data, section.sh_addr)
}

/// Raw bytes of a named section, or an empty slice when it is absent or truncated.
fn section_data<'a>(
    bytes: &'a [u8],
    sections: &[SectionHeader],
    names: &[String],
    wanted: &str,
) -> &'a [u8] {
    let Some(section) = sections
        .iter()
        .zip(names.iter())
        .find(|(section, name)| *name == wanted && section.sh_type != SHT_NOBITS)
        .map(|(section, _)| section)
    else {
        return &[];
    };
    let (Ok(start), Ok(size)) = (
        usize::try_from(section.sh_offset),
        usize::try_from(section.sh_size),
    ) else {
        return &[];
    };
    bytes
        .get(start..start.saturating_add(size).min(bytes.len()))
        .unwrap_or(&[])
}

fn parse_eh_frame(data: &[u8], section_va: u64) -> Vec<(u64, u64)> {
    let mut cies: HashMap<usize, Cie> = HashMap::new();
    let mut ranges = Vec::new();
    let mut position = 0usize;
    while position + 4 <= data.len() {
        let length = u32::from_le_bytes([
            data[position],
            data[position + 1],
            data[position + 2],
            data[position + 3],
        ]) as usize;
        if length == 0 {
            break;
        }
        let body = position + 4;
        let end = match body.checked_add(length) {
            Some(end) if end <= data.len() => end,
            _ => break,
        };
        if length < 4 {
            break;
        }
        let id = u32::from_le_bytes([data[body], data[body + 1], data[body + 2], data[body + 3]]);
        if id == 0 {
            if let Some(cie) = parse_cie(&data[body + 4..end]) {
                cies.insert(position, cie);
            }
        } else {
            let cie_offset = body.wrapping_sub(id as usize);
            let Some(cie) = cies.get(&cie_offset).copied() else {
                position = end;
                continue;
            };
            if let Some(range) = parse_fde(&data[body + 4..end], section_va, body + 4, &cie) {
                ranges.push(range);
                if ranges.len() >= MAX_EH_FRAME_RECORDS {
                    break;
                }
            }
        }
        position = end;
    }
    ranges
}

#[derive(Clone, Copy)]
struct Cie {
    fde_encoding: u8,
    has_z: bool,
}

fn parse_cie(mut data: &[u8]) -> Option<Cie> {
    let _version = data.first().copied()?;
    data = &data[1..];
    let nul = data.iter().position(|byte| *byte == 0)?;
    let augmentation = std::str::from_utf8(&data[..nul]).ok()?;
    data = &data[nul + 1..];
    let mut cursor = Cursor::new(data);
    cursor.uleb()?; // code alignment
    cursor.sleb()?; // data alignment
    cursor.uleb()?; // return address register
    let has_z = augmentation.starts_with('z');
    let mut fde_encoding = 0u8;
    if has_z {
        let length = cursor.uleb()? as usize;
        let mut body = cursor.take(length)?;
        // The 'z' body follows the augmentation string order after the leading 'z'.
        for character in augmentation[1..].chars() {
            match character {
                'R' => fde_encoding = body.first().copied()?,
                'L' => {
                    body = body.get(1..)?;
                }
                'P' => {
                    let encoding = body.first().copied()?;
                    body = body.get(1..)?;
                    let size = encoded_size(encoding)?;
                    body = body.get(size..)?;
                }
                'S' => {}
                _ => return None,
            };
        }
    }
    Some(Cie {
        fde_encoding,
        has_z,
    })
}

fn parse_fde(data: &[u8], section_va: u64, field_offset: usize, cie: &Cie) -> Option<(u64, u64)> {
    let mut cursor = Cursor::new(data);
    let field_va = section_va.checked_add(field_offset as u64)?;
    let begin = cursor.encoded(cie.fde_encoding, field_va)?;
    let range = cursor.encoded(cie.fde_encoding & 0x0f, 0)?;
    if cie.has_z {
        let length = cursor.uleb()? as usize;
        cursor.take(length)?;
    }
    if range == 0 {
        return None;
    }
    Some((begin, begin.checked_add(range)?))
}

/// Number of bytes an encoded value occupies, for skipping (only the fixed-width
/// encodings appear in personality pointers).
fn encoded_size(encoding: u8) -> Option<usize> {
    match encoding & 0x0f {
        0x00 => Some(8),
        0x02 | 0x0a => Some(2),
        0x03 | 0x0b => Some(4),
        0x04 | 0x0c => Some(8),
        _ => None,
    }
}

struct Cursor<'a> {
    data: &'a [u8],
    position: usize,
}

impl<'a> Cursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, position: 0 }
    }

    fn take(&mut self, count: usize) -> Option<&'a [u8]> {
        let end = self.position.checked_add(count)?;
        let slice = self.data.get(self.position..end)?;
        self.position = end;
        Some(slice)
    }

    fn u8(&mut self) -> Option<u8> {
        Some(self.take(1)?[0])
    }

    fn u16(&mut self) -> Option<u16> {
        let slice = self.take(2)?;
        Some(u16::from_le_bytes([slice[0], slice[1]]))
    }

    fn u32(&mut self) -> Option<u32> {
        let slice = self.take(4)?;
        Some(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
    }

    fn u64(&mut self) -> Option<u64> {
        let slice = self.take(8)?;
        let mut buffer = [0u8; 8];
        buffer.copy_from_slice(slice);
        Some(u64::from_le_bytes(buffer))
    }

    fn uleb(&mut self) -> Option<u64> {
        let mut result = 0u64;
        let mut shift = 0u32;
        loop {
            let byte = self.u8()?;
            result |= u64::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                return Some(result);
            }
            shift += 7;
            if shift >= 64 {
                return None;
            }
        }
    }

    fn sleb(&mut self) -> Option<i64> {
        let mut result = 0i64;
        let mut shift = 0u32;
        let mut byte;
        loop {
            byte = self.u8()?;
            result |= i64::from(byte & 0x7f) << shift;
            shift += 7;
            if byte & 0x80 == 0 {
                break;
            }
            if shift >= 64 {
                return None;
            }
        }
        if shift < 64 && byte & 0x40 != 0 {
            result |= -1i64 << shift;
        }
        Some(result)
    }

    /// Read one DWARF-encoded pointer (`encoding` is the `DW_EH_PE_*` byte).
    fn encoded(&mut self, encoding: u8, field_va: u64) -> Option<u64> {
        if encoding == 0xff {
            return None;
        }
        let format = encoding & 0x0f;
        let application = encoding & 0x70;
        if encoding & 0x80 != 0 {
            // Indirect encodings point at a slot; the slot's content is the address,
            // which we cannot follow without reading the image again.
            return None;
        }
        let mut value = match format {
            0x00 => self.u64()?,
            0x01 => self.uleb()?,
            0x02 => u64::from(self.u16()?),
            0x03 => u64::from(self.u32()?),
            0x04 => self.u64()?,
            0x09 => self.sleb()? as u64,
            0x0a => i64::from(self.u16()? as i16) as u64,
            0x0b => i64::from(self.u32()? as i32) as u64,
            0x0c => self.u64()?,
            _ => return None,
        };
        match application {
            0x00 => {}
            0x10 => value = value.wrapping_add(field_va),
            _ => return None,
        }
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_needs_a_64_bit_little_endian_header() {
        let mut bytes = vec![0u8; 64];
        bytes[..4].copy_from_slice(&ELF_MAGIC);
        bytes[4] = ELFCLASS64;
        bytes[5] = ELFDATA2LSB;
        assert!(detect(&bytes));

        bytes[4] = 1; // ELFCLASS32
        assert!(!detect(&bytes));
        bytes[4] = ELFCLASS64;
        bytes[5] = 2; // big endian
        assert!(!detect(&bytes));
        assert!(!detect(b"\x7fELF"));
    }

    #[test]
    fn a_truncated_header_is_rejected_with_a_message() {
        let mut bytes = vec![0u8; 64];
        bytes[..4].copy_from_slice(&ELF_MAGIC);
        bytes[4] = ELFCLASS64;
        bytes[5] = ELFDATA2LSB;
        bytes[6] = EV_CURRENT;
        // e_type = ET_REL is a relocatable object, not a linked image.
        bytes[16] = ET_EXEC as u8;
        bytes[18] = 0x28; // EM_ARM, unsupported
        let error = load(&bytes).unwrap_err();
        assert!(
            error.to_string().contains("unsupported ELF machine"),
            "{error}"
        );
    }

    #[test]
    fn arm64_images_are_refused_until_the_lifter_exists() {
        let mut bytes = vec![0u8; 64];
        bytes[..4].copy_from_slice(&ELF_MAGIC);
        bytes[4] = ELFCLASS64;
        bytes[5] = ELFDATA2LSB;
        bytes[6] = EV_CURRENT;
        bytes[16] = ET_EXEC as u8;
        bytes[18] = EM_AARCH64 as u8;
        let error = load(&bytes).unwrap_err();
        assert!(error.to_string().contains("ARM64 lifter"), "{error}");
    }
}
