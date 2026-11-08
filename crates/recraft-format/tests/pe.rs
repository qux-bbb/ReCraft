// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! PE loader integration tests. The images are assembled byte by byte in code
//! so the tests do not depend on checked-in binaries or an assembler.

use recraft_common::Address;
use recraft_format::{Image, detect, load_pe as load};

const DOS_STUB_SIZE: usize = 0x80;
const OPTIONAL_OFFSET: usize = DOS_STUB_SIZE + 24;
const TEXT_RAW: usize = 0x400;
const TEXT_RVA: u32 = 0x1000;
const TEXT_SIZE: usize = 0x600;
const DATA_RAW: usize = 0xa00;
const DATA_RVA: u32 = 0x2000;
const DATA_SIZE: usize = 0x200;
const FILE_SIZE: usize = DATA_RAW + DATA_SIZE;
const IMAGE_BASE_32: u32 = 0x0040_0000;
const IMAGE_BASE_64: u64 = 0x0000_0001_4000_0000;

const TEXT_CHARACTERISTICS: u32 = 0x6000_0020; // code | execute | read
const DATA_CHARACTERISTICS: u32 = 0xc000_0040; // initialized data | read | write

fn text_offset(rva: u32) -> usize {
    TEXT_RAW + (rva - TEXT_RVA) as usize
}

fn put_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn put_ptr(bytes: &mut [u8], offset: usize, value: u64, is_64: bool) {
    if is_64 {
        put_u64(bytes, offset, value);
    } else {
        put_u32(bytes, offset, value as u32);
    }
}

fn put_bytes(bytes: &mut [u8], offset: usize, value: &[u8]) {
    bytes[offset..offset + value.len()].copy_from_slice(value);
}

#[allow(clippy::too_many_arguments)] // Test helper; a builder struct would only add noise.
fn write_section(
    bytes: &mut [u8],
    offset: usize,
    name: &[u8],
    virtual_size: u32,
    virtual_address: u32,
    raw_size: u32,
    raw_offset: u32,
    characteristics: u32,
) {
    assert!(name.len() <= 8);
    for (index, &byte) in name.iter().enumerate() {
        bytes[offset + index] = byte;
    }
    put_u32(bytes, offset + 8, virtual_size);
    put_u32(bytes, offset + 12, virtual_address);
    put_u32(bytes, offset + 16, raw_size);
    put_u32(bytes, offset + 20, raw_offset);
    put_u32(bytes, offset + 36, characteristics);
}

fn optional_layout(is_64: bool) -> (u16, u16, usize, usize, usize) {
    if is_64 {
        (0x20b, 240, 108, 112, 24)
    } else {
        (0x10b, 224, 92, 96, 28)
    }
}

/// Build a PE32 or PE32+ image with imports, delay imports, exports, TLS and
/// (for PE32+) a single `.pdata` entry.
fn build_pe(is_64: bool) -> Vec<u8> {
    build_pe_with_delay_attributes(is_64, false)
}

fn build_pe_with_delay_attributes(is_64: bool, delay_fields_are_va: bool) -> Vec<u8> {
    let mut bytes = vec![0u8; FILE_SIZE];
    let (magic, optional_size, directory_count_offset, directory_offset, image_base_offset) =
        optional_layout(is_64);
    let image_base = if is_64 {
        IMAGE_BASE_64
    } else {
        u64::from(IMAGE_BASE_32)
    };
    let pointer_size = if is_64 { 8usize } else { 4usize };

    // DOS header.
    put_bytes(&mut bytes, 0, b"MZ");
    put_u32(&mut bytes, 0x3c, DOS_STUB_SIZE as u32);

    // PE signature + COFF header.
    let coff = DOS_STUB_SIZE + 4;
    put_bytes(&mut bytes, DOS_STUB_SIZE, b"PE\0\0");
    put_u16(&mut bytes, coff, if is_64 { 0x8664 } else { 0x014c });
    put_u16(&mut bytes, coff + 2, 2);
    put_u32(&mut bytes, coff + 4, 0x5f5e_1000);
    put_u16(&mut bytes, coff + 16, optional_size);
    put_u16(&mut bytes, coff + 18, 0x0102);
    put_u16(&mut bytes, coff + 20, 0x0022);

    // Optional header (only the fields the loader consumes plus the basics).
    let optional = OPTIONAL_OFFSET;
    put_u16(&mut bytes, optional, magic);
    put_u32(&mut bytes, optional + 16, TEXT_RVA + 0x10); // entry point
    if is_64 {
        put_u64(&mut bytes, optional + image_base_offset, image_base);
    } else {
        put_u32(&mut bytes, optional + image_base_offset, image_base as u32);
    }
    put_u32(&mut bytes, optional + 32, 0x1000); // SectionAlignment
    put_u32(&mut bytes, optional + 36, 0x200); // FileAlignment
    put_u32(&mut bytes, optional + 56, 0x3000); // SizeOfImage
    put_u32(&mut bytes, optional + 60, 0x400); // SizeOfHeaders
    put_u16(&mut bytes, optional + 68, 3); // console subsystem
    put_u32(&mut bytes, optional + directory_count_offset, 16);

    // Section table.
    let section_table = optional + usize::from(optional_size);
    write_section(
        &mut bytes,
        section_table,
        b".text",
        TEXT_SIZE as u32,
        TEXT_RVA,
        TEXT_SIZE as u32,
        TEXT_RAW as u32,
        TEXT_CHARACTERISTICS,
    );
    write_section(
        &mut bytes,
        section_table + 40,
        b".data",
        DATA_SIZE as u32,
        DATA_RVA,
        DATA_SIZE as u32,
        DATA_RAW as u32,
        DATA_CHARACTERISTICS,
    );

    // Code / callback targets.
    put_bytes(&mut bytes, text_offset(0x1000), &[0xc3; 8]);
    put_bytes(&mut bytes, text_offset(0x1010), &[0x90, 0xc3]);
    put_bytes(&mut bytes, text_offset(0x1020), &[0x90, 0xc3]);
    put_bytes(&mut bytes, text_offset(0x1030), &[0x90, 0xc3]);

    // Import directory: one normal module.
    let import_descriptor = text_offset(0x1100);
    put_u32(&mut bytes, import_descriptor, 0x1280); // OriginalFirstThunk
    put_u32(&mut bytes, import_descriptor + 12, 0x1200); // Name
    put_u32(&mut bytes, import_descriptor + 16, 0x12a0); // FirstThunk
    // The 20 bytes at 0x1114 are already zero (terminator).

    // Delay-import directory: one module.
    let delay_descriptor = text_offset(0x1160);
    let delay_field = |rva: u32| -> u32 {
        if delay_fields_are_va {
            (image_base + u64::from(rva)) as u32
        } else {
            rva
        }
    };
    put_u32(
        &mut bytes,
        delay_descriptor,
        if delay_fields_are_va { 1 } else { 0 },
    );
    put_u32(&mut bytes, delay_descriptor + 4, delay_field(0x1240));
    put_u32(&mut bytes, delay_descriptor + 12, delay_field(0x12e0));
    put_u32(&mut bytes, delay_descriptor + 16, delay_field(0x12c0));
    // The 32 bytes at 0x1180 are already zero (terminator).

    // Module and function names.
    put_bytes(&mut bytes, text_offset(0x1200), b"test.dll\0");
    put_u16(&mut bytes, text_offset(0x1210), 0x1234);
    put_bytes(&mut bytes, text_offset(0x1212), b"FuncA\0");
    put_bytes(&mut bytes, text_offset(0x1240), b"delay.dll\0");
    put_u16(&mut bytes, text_offset(0x1260), 0x4321);
    put_bytes(&mut bytes, text_offset(0x1262), b"DelayFunc\0");

    // Normal thunks: named function + ordinal import.
    let ordinal_thunk = if is_64 {
        0x8000_0000_0000_0002
    } else {
        0x8000_0002
    };
    put_ptr(&mut bytes, text_offset(0x1280), 0x1210, is_64);
    put_ptr(
        &mut bytes,
        text_offset(0x1280) + pointer_size,
        ordinal_thunk,
        is_64,
    );
    put_ptr(&mut bytes, text_offset(0x12a0), 0x1210, is_64);
    put_ptr(
        &mut bytes,
        text_offset(0x12a0) + pointer_size,
        ordinal_thunk,
        is_64,
    );

    // Delay-import name table (the IAT itself stays zero on disk).
    put_ptr(&mut bytes, text_offset(0x12c0), 0x1260, is_64);

    // Export directory.
    let export_directory = text_offset(0x1300);
    put_u32(&mut bytes, export_directory + 12, 0x13a0); // Name
    put_u32(&mut bytes, export_directory + 16, 1); // Base
    put_u32(&mut bytes, export_directory + 20, 3); // NumberOfFunctions
    put_u32(&mut bytes, export_directory + 24, 2); // NumberOfNames
    put_u32(&mut bytes, export_directory + 28, 0x1340); // AddressOfFunctions
    put_u32(&mut bytes, export_directory + 32, 0x1360); // AddressOfNames
    put_u32(&mut bytes, export_directory + 36, 0x1380); // AddressOfNameOrdinals

    let export_functions = text_offset(0x1340);
    put_u32(&mut bytes, export_functions, 0x1010);
    put_u32(&mut bytes, export_functions + 4, 0x13b0); // forwarder
    put_u32(&mut bytes, export_functions + 8, 0x1030);

    let export_names = text_offset(0x1360);
    put_u32(&mut bytes, export_names, 0x13c0);
    put_u32(&mut bytes, export_names + 4, 0x13d0);

    let export_ordinals = text_offset(0x1380);
    put_u16(&mut bytes, export_ordinals, 0);
    put_u16(&mut bytes, export_ordinals + 2, 1);

    put_bytes(&mut bytes, text_offset(0x13a0), b"test.dll\0");
    put_bytes(&mut bytes, text_offset(0x13b0), b"other.Func\0");
    put_bytes(&mut bytes, text_offset(0x13c0), b"ExportedFunc\0");
    put_bytes(&mut bytes, text_offset(0x13d0), b"ForwardedFunc\0");

    // TLS directory + callback array (callback VAs terminated by zero).
    let tls_directory = text_offset(0x1400);
    let callbacks_va = image_base + u64::from(0x1440_u32);
    if is_64 {
        put_u64(&mut bytes, tls_directory + 24, callbacks_va);
    } else {
        put_u32(&mut bytes, tls_directory + 12, callbacks_va as u32);
    }
    let callbacks = text_offset(0x1440);
    put_ptr(
        &mut bytes,
        callbacks,
        image_base + u64::from(0x1010_u32),
        is_64,
    );
    put_ptr(
        &mut bytes,
        callbacks + pointer_size,
        image_base + u64::from(0x1020_u32),
        is_64,
    );

    // .pdata entry (honoured for PE32+ only).
    let pdata = text_offset(0x1500);
    put_u32(&mut bytes, pdata, 0x1010);
    put_u32(&mut bytes, pdata + 4, 0x1030);
    put_u32(&mut bytes, pdata + 8, 0);

    // Data directories.
    let directories = optional + directory_offset;
    put_u32(&mut bytes, directories, 0x1300); // export
    put_u32(&mut bytes, directories + 4, 0xc0);
    put_u32(&mut bytes, directories + 8, 0x1100); // import
    put_u32(&mut bytes, directories + 12, 40);
    put_u32(&mut bytes, directories + 24, 0x1500); // exception
    put_u32(&mut bytes, directories + 28, 12);
    put_u32(&mut bytes, directories + 72, 0x1400); // TLS
    put_u32(&mut bytes, directories + 76, if is_64 { 40 } else { 24 });
    put_u32(&mut bytes, directories + 104, 0x1160); // delay import
    put_u32(&mut bytes, directories + 108, 64);

    bytes
}

fn exports_named<'a>(image: &'a Image, name: &str) -> &'a recraft_format::Export {
    image
        .exports
        .iter()
        .find(|export| export.name.as_deref() == Some(name))
        .unwrap_or_else(|| panic!("missing export {name:?}"))
}

#[test]
fn parses_pe32_image() {
    let bytes = build_pe(false);
    assert!(detect(&bytes));

    let image = load(&bytes).expect("PE32 should parse");
    assert!(!image.is_64);
    assert_eq!(image.image_base, Address(u64::from(IMAGE_BASE_32)));
    assert_eq!(image.entry_point, Address(0x0040_1010));
    assert_eq!(image.size_of_image, 0x3000);
    assert_eq!(image.size_of_headers, 0x400);
    assert_eq!(image.subsystem, 3);
    assert_eq!(image.timestamp, 0x5f5e_1000);
    assert_eq!(image.file_size, FILE_SIZE as u64);
    assert_eq!(image.overlay_file_offset, None);

    assert_eq!(image.sections.len(), 2);
    let text = &image.sections[0];
    assert_eq!(text.name, ".text");
    assert_eq!(text.va, Address(0x0040_1000));
    assert_eq!(text.virtual_size, 0x600);
    assert_eq!(text.raw_offset, 0x400);
    assert_eq!(text.raw_size, 0x600);
    assert_eq!(text.characteristics, TEXT_CHARACTERISTICS);
    assert!(text.is_executable());
    assert!(text.is_readable());
    assert!(!text.is_writable());
    assert!(!text.is_discardable());
    assert!(text.contains_rva(0x1000));
    assert!(text.contains_rva(0x15ff));
    assert!(!text.contains_rva(0x1600));
    assert!(!text.contains_rva(0x2000));

    let data = &image.sections[1];
    assert_eq!(data.name, ".data");
    assert_eq!(data.va, Address(0x0040_2000));
    assert!(data.is_readable());
    assert!(data.is_writable());
    assert!(data.contains_rva(0x2000));

    // Imports: normal module first, then the delayed module.
    assert_eq!(image.imports.len(), 2);
    let normal = &image.imports[0];
    assert_eq!(normal.name, "test.dll");
    assert!(!normal.delayed);
    assert_eq!(normal.iat_va, Address(0x0040_12a0));
    assert_eq!(normal.functions.len(), 2);
    assert_eq!(normal.functions[0].name.as_deref(), Some("FuncA"));
    assert_eq!(normal.functions[0].hint, Some(0x1234));
    assert_eq!(normal.functions[0].ordinal, None);
    assert_eq!(normal.functions[0].iat_va, Address(0x0040_12a0));
    assert_eq!(normal.functions[1].name, None);
    assert_eq!(normal.functions[1].ordinal, Some(2));
    assert_eq!(normal.functions[1].hint, None);
    assert_eq!(normal.functions[1].iat_va, Address(0x0040_12a4));

    let delayed = &image.imports[1];
    assert_eq!(delayed.name, "delay.dll");
    assert!(delayed.delayed);
    assert_eq!(delayed.iat_va, Address(0x0040_12e0));
    assert_eq!(delayed.functions.len(), 1);
    assert_eq!(delayed.functions[0].name.as_deref(), Some("DelayFunc"));
    assert_eq!(delayed.functions[0].hint, Some(0x4321));
    assert_eq!(delayed.functions[0].iat_va, Address(0x0040_12e0));

    // Exports: two named (one forwarded) plus one ordinal-only entry.
    assert_eq!(image.exports.len(), 3);
    let exported = exports_named(&image, "ExportedFunc");
    assert_eq!(exported.ordinal, 1);
    assert_eq!(exported.rva, 0x1010);
    assert_eq!(exported.forwarder, None);

    let forwarded = exports_named(&image, "ForwardedFunc");
    assert_eq!(forwarded.ordinal, 2);
    assert_eq!(forwarded.rva, 0x13b0);
    assert_eq!(forwarded.forwarder.as_deref(), Some("other.Func"));

    let ordinal_only = image
        .exports
        .iter()
        .find(|export| export.ordinal == 3)
        .expect("ordinal-only export");
    assert_eq!(ordinal_only.name, None);
    assert_eq!(ordinal_only.rva, 0x1030);

    assert_eq!(
        image.tls_callbacks,
        vec![Address(0x0040_1010), Address(0x0040_1020)]
    );
    assert!(image.pdata_functions.is_empty(), "PE32 must skip .pdata");

    assert_eq!(image.rva_to_file_offset(0x1000), Some(0x400));
    assert_eq!(
        image
            .segment_containing(Address(0x0040_1010))
            .map(|section| section.name.as_str()),
        Some(".text")
    );
    assert_eq!(
        image
            .segment_containing(Address(0x0040_2000))
            .map(|section| section.name.as_str()),
        Some(".data")
    );
    assert!(image.segment_containing(Address(0x0040_1f00)).is_none());
}

#[test]
fn parses_pe32_plus_image() {
    let bytes = build_pe(true);
    assert!(detect(&bytes));

    let image = load(&bytes).expect("PE32+ should parse");
    assert!(image.is_64);
    assert_eq!(image.image_base, Address(IMAGE_BASE_64));
    assert_eq!(image.entry_point, Address(IMAGE_BASE_64 + 0x1010));
    assert_eq!(image.size_of_image, 0x3000);
    assert_eq!(image.sections.len(), 2);

    assert_eq!(image.imports.len(), 2);
    let normal = &image.imports[0];
    assert_eq!(normal.name, "test.dll");
    assert_eq!(normal.iat_va, Address(IMAGE_BASE_64 + 0x12a0));
    assert_eq!(normal.functions.len(), 2);
    assert_eq!(normal.functions[0].name.as_deref(), Some("FuncA"));
    assert_eq!(normal.functions[0].iat_va, Address(IMAGE_BASE_64 + 0x12a0));
    assert_eq!(normal.functions[1].ordinal, Some(2));
    assert_eq!(normal.functions[1].iat_va, Address(IMAGE_BASE_64 + 0x12a8));

    let delayed = &image.imports[1];
    assert_eq!(delayed.name, "delay.dll");
    assert!(delayed.delayed);
    assert_eq!(delayed.iat_va, Address(IMAGE_BASE_64 + 0x12e0));
    assert_eq!(delayed.functions.len(), 1);
    assert_eq!(delayed.functions[0].iat_va, Address(IMAGE_BASE_64 + 0x12e0));

    assert_eq!(image.exports.len(), 3);
    assert_eq!(exports_named(&image, "ExportedFunc").ordinal, 1);
    assert_eq!(
        exports_named(&image, "ForwardedFunc").forwarder.as_deref(),
        Some("other.Func")
    );

    assert_eq!(
        image.tls_callbacks,
        vec![
            Address(IMAGE_BASE_64 + 0x1010),
            Address(IMAGE_BASE_64 + 0x1020)
        ]
    );
    assert_eq!(
        image.pdata_functions,
        vec![recraft_format::FunctionRange {
            start: Address(IMAGE_BASE_64 + 0x1010),
            end: Address(IMAGE_BASE_64 + 0x1030),
        }]
    );
    assert_eq!(image.overlay_file_offset, None);
}

#[test]
fn delay_descriptor_can_use_vas() {
    let image = load(&build_pe_with_delay_attributes(false, true)).expect("VA delay imports");
    let delayed = &image.imports[1];
    assert_eq!(delayed.name, "delay.dll");
    assert!(delayed.delayed);
    assert_eq!(delayed.iat_va, Address(0x0040_12e0));
    assert_eq!(delayed.functions[0].name.as_deref(), Some("DelayFunc"));
    assert_eq!(delayed.functions[0].hint, Some(0x4321));
}

#[test]
fn rva_to_file_offset_across_section_boundaries() {
    let image = load(&build_pe(false)).expect("PE32 should parse");

    // Header region.
    assert_eq!(image.rva_to_file_offset(0), Some(0));
    assert_eq!(image.rva_to_file_offset(0x100), Some(0x100));
    assert_eq!(image.rva_to_file_offset(0x3ff), Some(0x3ff));
    // .text.
    assert_eq!(image.rva_to_file_offset(0x1000), Some(0x400));
    assert_eq!(image.rva_to_file_offset(0x15ff), Some(0x9ff));
    // Virtual gap / end of .text.
    assert_eq!(image.rva_to_file_offset(0x1600), None);
    assert_eq!(image.rva_to_file_offset(0x1fff), None);
    // .data.
    assert_eq!(image.rva_to_file_offset(0x2000), Some(0xa00));
    assert_eq!(image.rva_to_file_offset(0x21ff), Some(0xbff));
    assert_eq!(image.rva_to_file_offset(0x2200), None);
    assert_eq!(image.rva_to_file_offset(0xffff_0000), None);
}

#[test]
fn detects_overlay() {
    let mut bytes = build_pe(true);
    bytes.extend_from_slice(&[0xaa; 16]);
    let image = load(&bytes).expect("image with overlay");
    assert_eq!(image.file_size, FILE_SIZE as u64 + 16);
    assert_eq!(image.overlay_file_offset, Some(FILE_SIZE as u64));
    assert!(
        image
            .sections
            .iter()
            .all(|section| !section.contains_rva(0xffffff))
    );
}

#[test]
fn rejects_empty_and_truncated_headers() {
    assert!(!detect(&[]));
    assert!(load(&[]).is_err());

    assert!(!detect(b"MZ"));
    assert!(load(b"MZ").is_err());

    let mut bad_lfanew = vec![0u8; 0x40];
    bad_lfanew[..2].copy_from_slice(b"MZ");
    put_u32(&mut bad_lfanew, 0x3c, 0x1000);
    assert!(!detect(&bad_lfanew));
    assert!(load(&bad_lfanew).is_err());
}

#[test]
fn rejects_unknown_optional_magic() {
    let mut bytes = build_pe(true);
    put_u16(&mut bytes, OPTIONAL_OFFSET, 0x999);
    assert!(!detect(&bytes));
    assert!(load(&bytes).is_err());
}

#[test]
fn rejects_zero_or_huge_section_counts() {
    let mut no_sections = build_pe(true);
    put_u16(&mut no_sections, DOS_STUB_SIZE + 4 + 2, 0);
    // Documented degradation: a sectionless PE is rejected outright.
    assert!(detect(&no_sections));
    assert!(load(&no_sections).is_err());

    let mut many_sections = build_pe(true);
    put_u16(&mut many_sections, DOS_STUB_SIZE + 4 + 2, 0xffff);
    assert!(load(&many_sections).is_err());
}

#[test]
fn rejects_section_raw_data_past_eof() {
    let mut bytes = build_pe(true);
    let section_table = OPTIONAL_OFFSET + 240;
    put_u32(&mut bytes, section_table + 20, 0x00ff_0000);
    assert!(load(&bytes).is_err());
}

#[test]
fn import_directory_past_eof_degrades_with_warning() {
    let mut bytes = build_pe(true);
    let directories = OPTIONAL_OFFSET + 112;
    put_u32(&mut bytes, directories + 8, 0x00ff_0000);
    let image = load(&bytes).expect("broken imports must not make the image unloadable");
    // Normal imports are gone; delay imports are parsed independently and survive.
    assert!(image.imports.iter().all(|module| module.delayed));
    assert!(!image.warnings.is_empty());
}

#[test]
fn truncated_hint_name_string_degrades_with_warning() {
    let mut bytes = build_pe(true);
    // Point the first import at a hint/name record that runs to EOF without a
    // terminator.
    bytes[0xbf0..0xc00].fill(b'A');
    let first_thunk = text_offset(0x1280);
    put_ptr(&mut bytes, first_thunk, u64::from(DATA_RVA) + 0x1f0, true);
    let image = load(&bytes).expect("truncated hint/name must not make the image unloadable");
    assert!(!image.warnings.is_empty());
    // The normal module is still reported, only its function list is lost.
    let normal: Vec<_> = image
        .imports
        .iter()
        .filter(|module| !module.delayed)
        .collect();
    assert_eq!(normal.len(), 1);
    assert!(normal[0].functions.is_empty());
}

#[test]
fn broken_tls_degrades_to_empty_callbacks() {
    let mut bytes = build_pe(true);
    // AddressOfCallBacks is a VA outside the image.
    let tls_directory = text_offset(0x1400);
    put_u64(&mut bytes, tls_directory + 24, IMAGE_BASE_64 + 0x9000);
    let image = load(&bytes).expect("broken TLS must not fail the load");
    assert!(image.tls_callbacks.is_empty());
}

#[test]
fn broken_pdata_degrades_to_empty_ranges() {
    let mut bytes = build_pe(true);
    let directories = OPTIONAL_OFFSET + 112;
    put_u32(&mut bytes, directories + 24, 0x00ff_0000);
    let image = load(&bytes).expect("broken .pdata must not fail the load");
    assert!(image.pdata_functions.is_empty());
}
