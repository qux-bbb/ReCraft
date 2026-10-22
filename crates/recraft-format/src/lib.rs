// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Executable format loaders (PE first; other formats behind the same trait).
//!
//! Part of ReCraft. See `docs/DESIGN.md` for the architecture.
//!
//! The PE loader is intentionally dependency-free and defensive: every read is
//! bounds-checked, every attacker-controlled count is capped, and malformed
//! input yields [`recraft_common::Error::Format`] (or degrades to an empty
//! collection where the format allows optional data).
//!
//! Degradation policy:
//! - a PE without sections is rejected;
//! - a section whose raw data extends past EOF is rejected;
//! - broken TLS data yields no TLS callbacks;
//! - broken `.pdata` data yields no function ranges;
//! - a broken delay-import descriptor skips that module only.
//!
//! Sections store both their absolute VA (`Section::va`) and the original RVA
//! (private, used by [`Section::contains_rva`]).

#![forbid(unsafe_code)]

use std::collections::HashMap;

use recraft_common::{Error, Result};

pub use recraft_common::Address;

/// Typical page size used by PE section alignment.
pub const PAGE_SIZE: u64 = 0x1000;

const MAX_SECTIONS: usize = 96;
const MAX_IMPORT_MODULES: usize = 4096;
const MAX_IMPORT_FUNCTIONS: usize = 200_000;
const MAX_TLS_CALLBACKS: usize = 256;
const MAX_PDATA_ENTRIES: usize = 200_000;
const MAX_EXPORTS: usize = 200_000;
const MAX_NAME_LEN: usize = 512;
const MAX_HINT_NAME_LEN: usize = 256;

const IMAGE_SCN_MEM_DISCARDABLE: u32 = 0x0200_0000;
const IMAGE_SCN_MEM_EXECUTE: u32 = 0x2000_0000;
const IMAGE_SCN_MEM_READ: u32 = 0x4000_0000;
const IMAGE_SCN_MEM_WRITE: u32 = 0x8000_0000;

const DIRECTORY_EXPORT: usize = 0;
const DIRECTORY_IMPORT: usize = 1;
const DIRECTORY_EXCEPTION: usize = 3;
const DIRECTORY_TLS: usize = 9;
const DIRECTORY_DELAY_IMPORT: usize = 13;

/// A loaded PE image.
#[derive(Debug, Clone, Default)]
pub struct PeImage {
    pub is_64: bool,
    pub image_base: Address,
    pub entry_point: Address,
    pub size_of_image: u64,
    pub size_of_headers: u64,
    pub subsystem: u16,
    pub timestamp: u32,
    pub sections: Vec<Section>,
    pub imports: Vec<ImportModule>,
    pub exports: Vec<Export>,
    pub tls_callbacks: Vec<Address>,
    pub pdata_functions: Vec<FunctionRange>,
    pub overlay_file_offset: Option<u64>,
    pub file_size: u64,
    /// Non-fatal problems found while parsing (malformed import/export/TLS tables,
    /// truncated tables, ...). A warning never prevents analysis of the image.
    pub warnings: Vec<String>,
}

impl PeImage {
    /// Find the section that maps `va` (headers are not represented).
    pub fn segment_containing(&self, va: Address) -> Option<&Section> {
        self.sections.iter().find(|section| {
            let start = section.va.get();
            let size = section.virtual_size.max(u64::from(section.raw_size));
            va.get() >= start && va.get() - start < size
        })
    }

    /// Translate an RVA to a file offset, or `None` when the RVA is not backed
    /// by file data (unmapped, virtual-only, or past EOF).
    pub fn rva_to_file_offset(&self, rva: u32) -> Option<u32> {
        map_rva(self.file_size, self.size_of_headers, &self.sections, rva)
    }
}

/// One PE section.
#[derive(Debug, Clone, Default)]
pub struct Section {
    pub name: String,
    pub va: Address,
    pub virtual_size: u64,
    pub raw_offset: u32,
    pub raw_size: u32,
    pub characteristics: u32,
    /// Original RVA (needed by [`Section::contains_rva`]).
    rva: u32,
}

impl Section {
    #[inline]
    pub fn is_executable(&self) -> bool {
        self.characteristics & IMAGE_SCN_MEM_EXECUTE != 0
    }

    #[inline]
    pub fn is_readable(&self) -> bool {
        self.characteristics & IMAGE_SCN_MEM_READ != 0
    }

    #[inline]
    pub fn is_writable(&self) -> bool {
        self.characteristics & IMAGE_SCN_MEM_WRITE != 0
    }

    #[inline]
    pub fn is_discardable(&self) -> bool {
        self.characteristics & IMAGE_SCN_MEM_DISCARDABLE != 0
    }

    /// Whether `rva` lies inside this section's virtual extent.
    pub fn contains_rva(&self, rva: u32) -> bool {
        let start = u64::from(self.rva);
        let size = self.virtual_size.max(u64::from(self.raw_size));
        u64::from(rva) >= start && u64::from(rva) - start < size
    }
}

/// One imported module.
#[derive(Debug, Clone, Default)]
pub struct ImportModule {
    pub name: String,
    pub iat_va: Address,
    pub delayed: bool,
    pub functions: Vec<ImportFunction>,
}

/// One imported function.
#[derive(Debug, Clone, Default)]
pub struct ImportFunction {
    pub name: Option<String>,
    pub ordinal: Option<u16>,
    pub hint: Option<u16>,
    pub iat_va: Address,
}

/// One exported symbol.
#[derive(Debug, Clone, Default)]
pub struct Export {
    pub name: Option<String>,
    pub ordinal: u32,
    pub rva: u32,
    pub forwarder: Option<String>,
}

/// A half-open virtual-address range from `.pdata`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FunctionRange {
    pub start: Address,
    pub end: Address,
}

/// Common interface implemented by every executable-format loader.
pub trait Loader {
    const NAME: &'static str;

    fn detect(bytes: &[u8]) -> bool
    where
        Self: Sized;

    fn load(bytes: &[u8]) -> Result<PeImage>
    where
        Self: Sized;
}

/// The PE loader.
pub struct PeLoader;

impl Loader for PeLoader {
    const NAME: &'static str = "PE";

    fn detect(bytes: &[u8]) -> bool {
        detect(bytes)
    }

    fn load(bytes: &[u8]) -> Result<PeImage> {
        load(bytes)
    }
}

/// Return `true` if `bytes` looks like a PE32 or PE32+ image.
pub fn detect(bytes: &[u8]) -> bool {
    let Some(pe_offset) = pe_offset(bytes) else {
        return false;
    };
    if bytes.get(pe_offset..pe_offset + 4) != Some(b"PE\0\0".as_slice()) {
        return false;
    }
    matches!(read_u16(bytes, pe_offset + 24), Ok(0x10b | 0x20b))
}

/// Parse a PE32/PE32+ image.
pub fn load(bytes: &[u8]) -> Result<PeImage> {
    let file_size = bytes.len() as u64;
    let pe_offset = pe_offset(bytes).ok_or_else(|| Error::format("not a PE image"))?;

    // COFF file header starts right after the PE signature.
    let coff = pe_offset + 4;
    let _machine = read_u16(bytes, coff)?;
    let number_of_sections = read_u16(bytes, coff + 2)?;
    let timestamp = read_u32(bytes, coff + 4)?;
    let size_of_optional_header = usize::from(read_u16(bytes, coff + 16)?);
    let _characteristics = read_u16(bytes, coff + 18)?;

    let optional = coff + 20;
    let optional_end = optional
        .checked_add(size_of_optional_header)
        .ok_or_else(|| Error::format("optional header size overflow"))?;
    if optional_end > bytes.len() {
        return Err(Error::format("optional header extends past end of file"));
    }

    let magic = read_u16(bytes, optional)?;
    let is_64 = match magic {
        0x10b => false,
        0x20b => true,
        other => {
            return Err(Error::format(format!(
                "unknown optional header magic {other:#x}"
            )));
        }
    };

    // Minimum sizes needed to reach NumberOfRvaAndSizes.
    let (minimum_optional_size, directory_count_offset, directory_offset) =
        if is_64 { (116, 108, 112) } else { (96, 92, 96) };
    if size_of_optional_header < minimum_optional_size {
        return Err(Error::format("optional header is too short"));
    }

    let image_base = if is_64 {
        read_u64(bytes, optional + 24)?
    } else {
        u64::from(read_u32(bytes, optional + 28)?)
    };
    let entry_rva = read_u32(bytes, optional + 16)?;
    let size_of_image = u64::from(read_u32(bytes, optional + 56)?);
    let size_of_headers = u64::from(read_u32(bytes, optional + 60)?);
    let subsystem = read_u16(bytes, optional + 68)?;

    let declared_directories = read_u32(bytes, optional + directory_count_offset)?;
    let directory_start = optional + directory_offset;
    // Only the first 16 entries are defined; ignore extras and entries that do
    // not physically fit in the optional header.
    let available_directories = ((optional_end.saturating_sub(directory_start)) / 8)
        .min(16)
        .min(declared_directories as usize);
    let mut directories = [DataDirectory::default(); 16];
    for (index, directory) in directories
        .iter_mut()
        .take(available_directories)
        .enumerate()
    {
        let offset = directory_start + index * 8;
        directory.rva = read_u32(bytes, offset)?;
        directory.size = read_u32(bytes, offset + 4)?;
    }

    if number_of_sections == 0 {
        return Err(Error::format("PE image has no sections"));
    }
    if usize::from(number_of_sections) > MAX_SECTIONS {
        return Err(Error::format(format!(
            "PE image declares {number_of_sections} sections (limit {MAX_SECTIONS})"
        )));
    }

    let section_table = optional_end;
    let section_table_end = section_table
        .checked_add(usize::from(number_of_sections) * 40)
        .ok_or_else(|| Error::format("section table size overflow"))?;
    if section_table_end > bytes.len() {
        return Err(Error::format("section table extends past end of file"));
    }

    let mut sections = Vec::with_capacity(usize::from(number_of_sections));
    for index in 0..usize::from(number_of_sections) {
        let entry = section_table + index * 40;
        let raw_name = read_array::<8>(bytes, entry, "section name")?;
        let name_end = raw_name.iter().position(|&byte| byte == 0).unwrap_or(8);
        let name = String::from_utf8_lossy(&raw_name[..name_end]).into_owned();

        let virtual_size = u64::from(read_u32(bytes, entry + 8)?);
        let virtual_address = read_u32(bytes, entry + 12)?;
        let raw_size = read_u32(bytes, entry + 16)?;
        let raw_offset = read_u32(bytes, entry + 20)?;
        let characteristics = read_u32(bytes, entry + 36)?;

        if raw_size > 0 {
            let raw_end = u64::from(raw_offset) + u64::from(raw_size);
            if raw_end > file_size {
                return Err(Error::format(format!(
                    "section {name:?} raw data extends past end of file"
                )));
            }
        }

        let va = image_base
            .checked_add(u64::from(virtual_address))
            .ok_or_else(|| Error::format("section virtual address overflow"))?;

        sections.push(Section {
            name,
            va: Address(va),
            virtual_size,
            raw_offset,
            raw_size,
            characteristics,
            rva: virtual_address,
        });
    }

    let parser = ImageParser {
        bytes,
        image_base,
        is_64,
        size_of_headers,
        file_size,
        sections: &sections,
    };

    // Malformed import/export tables must not make an otherwise analysable binary
    // unloadable: keep whatever parsed and record a warning (untrusted samples do this).
    let mut warnings = Vec::new();
    let mut imports = parser.parse_imports(directories[DIRECTORY_IMPORT], &mut warnings);
    // Delay imports are optional/best-effort; append after normal imports.
    imports.extend(parser.parse_delay_imports(directories[DIRECTORY_DELAY_IMPORT]));
    let exports = parser.parse_exports(directories[DIRECTORY_EXPORT], &mut warnings);
    let tls_callbacks = parser.parse_tls(directories[DIRECTORY_TLS]);
    let pdata_functions = parser.parse_pdata(directories[DIRECTORY_EXCEPTION]);

    let overlay_file_offset = overlay_start(&sections, file_size);
    let entry_point = image_base
        .checked_add(u64::from(entry_rva))
        .ok_or_else(|| Error::format("entry point address overflow"))?;

    Ok(PeImage {
        is_64,
        image_base: Address(image_base),
        entry_point: Address(entry_point),
        size_of_image,
        size_of_headers,
        subsystem,
        timestamp,
        sections,
        imports,
        exports,
        tls_callbacks,
        pdata_functions,
        overlay_file_offset,
        file_size,
        warnings,
    })
}

#[derive(Debug, Clone, Copy, Default)]
struct DataDirectory {
    rva: u32,
    size: u32,
}

struct ImageParser<'a> {
    bytes: &'a [u8],
    image_base: u64,
    is_64: bool,
    size_of_headers: u64,
    file_size: u64,
    sections: &'a [Section],
}

impl ImageParser<'_> {
    fn rva_to_file_offset(&self, rva: u32) -> Option<u32> {
        map_rva(self.file_size, self.size_of_headers, self.sections, rva)
    }

    fn mapped(&self, rva: u32) -> Result<usize> {
        self.rva_to_file_offset(rva)
            .map(|offset| offset as usize)
            .ok_or_else(|| Error::format(format!("RVA {rva:#x} is not mapped to file data")))
    }

    fn read_u16_rva(&self, rva: u32) -> Result<u16> {
        read_u16(self.bytes, self.mapped(rva)?)
    }

    fn read_ptr_rva(&self, rva: u32) -> Result<u64> {
        let offset = self.mapped(rva)?;
        if self.is_64 {
            read_u64(self.bytes, offset)
        } else {
            read_u32(self.bytes, offset).map(u64::from)
        }
    }

    fn read_cstring_rva(&self, rva: u32, maximum: usize) -> Result<String> {
        let offset = self.mapped(rva)?;
        read_cstring(self.bytes, offset, maximum)
    }

    /// Translate a delay-load descriptor field, which may hold a VA.
    fn delay_rva(&self, raw: u32, attributes: u32) -> Option<u32> {
        if attributes & 1 == 0 {
            return Some(raw);
        }
        let value = u64::from(raw);
        if value >= self.image_base {
            u32::try_from(value - self.image_base).ok()
        } else {
            Some(raw)
        }
    }

    fn va_to_rva(&self, va: u64) -> Option<u32> {
        u32::try_from(va.checked_sub(self.image_base)?).ok()
    }

    /// Best-effort import parsing: a malformed descriptor stops the walk but keeps
    /// everything parsed so far and records a warning.
    fn parse_imports(
        &self,
        directory: DataDirectory,
        warnings: &mut Vec<String>,
    ) -> Vec<ImportModule> {
        if directory.rva == 0 {
            return Vec::new();
        }

        let mut modules = Vec::new();
        let mut descriptor_rva = directory.rva;
        for _ in 0..MAX_IMPORT_MODULES {
            let descriptor = match self.read_at_rva::<20>(descriptor_rva) {
                Ok(descriptor) => descriptor,
                Err(e) => {
                    warnings.push(format!("import descriptor at {descriptor_rva:#x}: {e}"));
                    break;
                }
            };
            if descriptor.iter().all(|&byte| byte == 0) {
                break;
            }

            let original_first_thunk =
                u32::from_le_bytes([descriptor[0], descriptor[1], descriptor[2], descriptor[3]]);
            let name_rva = u32::from_le_bytes([
                descriptor[12],
                descriptor[13],
                descriptor[14],
                descriptor[15],
            ]);
            let first_thunk = u32::from_le_bytes([
                descriptor[16],
                descriptor[17],
                descriptor[18],
                descriptor[19],
            ]);

            let name = match self.read_cstring_rva(name_rva, MAX_NAME_LEN) {
                Ok(name) => name,
                Err(e) => {
                    warnings.push(format!("import name RVA {name_rva:#x}: {e}"));
                    break;
                }
            };
            let thunk_rva = if original_first_thunk != 0 {
                original_first_thunk
            } else {
                first_thunk
            };
            let functions = if thunk_rva == 0 {
                Vec::new()
            } else {
                match self.parse_thunks(thunk_rva, first_thunk) {
                    Ok(functions) => functions,
                    Err(e) => {
                        warnings.push(format!("import thunks for {name:?}: {e}"));
                        Vec::new()
                    }
                }
            };

            let Some(iat_va) = self.image_base.checked_add(u64::from(first_thunk)) else {
                warnings.push("import address table address overflow".to_owned());
                break;
            };
            modules.push(ImportModule {
                name,
                iat_va: Address(iat_va),
                delayed: false,
                functions,
            });

            let Some(next) = descriptor_rva.checked_add(20) else {
                warnings.push("import descriptor offset overflow".to_owned());
                break;
            };
            descriptor_rva = next;
        }

        // The table was not terminated before the cap; keep what we have.
        modules
    }

    fn parse_delay_imports(&self, directory: DataDirectory) -> Vec<ImportModule> {
        if directory.rva == 0 {
            return Vec::new();
        }

        let mut modules = Vec::new();
        let mut descriptor_rva = directory.rva;
        for _ in 0..MAX_IMPORT_MODULES {
            let Ok(descriptor) = self.read_at_rva::<32>(descriptor_rva) else {
                break;
            };
            if descriptor.iter().all(|&byte| byte == 0) {
                break;
            }
            if let Ok(module) = self.parse_delay_descriptor(&descriptor) {
                modules.push(module);
            }
            let Some(next) = descriptor_rva.checked_add(32) else {
                break;
            };
            descriptor_rva = next;
        }
        modules
    }

    fn parse_delay_descriptor(&self, descriptor: &[u8; 32]) -> Result<ImportModule> {
        let attributes =
            u32::from_le_bytes([descriptor[0], descriptor[1], descriptor[2], descriptor[3]]);
        let field = |offset: usize| {
            u32::from_le_bytes([
                descriptor[offset],
                descriptor[offset + 1],
                descriptor[offset + 2],
                descriptor[offset + 3],
            ])
        };

        let name_rva = self
            .delay_rva(field(4), attributes)
            .ok_or_else(|| Error::format("invalid delay-import name RVA"))?;
        let iat_rva = self
            .delay_rva(field(12), attributes)
            .ok_or_else(|| Error::format("invalid delay-import IAT RVA"))?;
        let int_rva = self
            .delay_rva(field(16), attributes)
            .ok_or_else(|| Error::format("invalid delay-import INT RVA"))?;

        if name_rva == 0 {
            return Err(Error::format("delay-import module has no name"));
        }

        let name = self.read_cstring_rva(name_rva, MAX_NAME_LEN)?;
        let thunk_rva = if int_rva != 0 { int_rva } else { iat_rva };
        let functions = if thunk_rva == 0 {
            Vec::new()
        } else {
            self.parse_thunks(thunk_rva, iat_rva)?
        };

        Ok(ImportModule {
            name,
            iat_va: Address(
                self.image_base
                    .checked_add(u64::from(iat_rva))
                    .ok_or_else(|| Error::format("delay-import IAT address overflow"))?,
            ),
            delayed: true,
            functions,
        })
    }

    /// Parse the thunk array at `thunk_rva`, recording IAT addresses relative
    /// to `iat_rva`.
    fn parse_thunks(&self, thunk_rva: u32, iat_rva: u32) -> Result<Vec<ImportFunction>> {
        let pointer_size: usize = if self.is_64 { 8 } else { 4 };
        let first_offset = self.mapped(thunk_rva)?;
        let remaining = self.bytes.len().saturating_sub(first_offset);
        let limit = MAX_IMPORT_FUNCTIONS.min(remaining / pointer_size);

        let mut functions = Vec::new();
        for index in 0..limit {
            let offset_in_array = index
                .checked_mul(pointer_size)
                .ok_or_else(|| Error::format("import thunk offset overflow"))?;
            let entry_rva = u32::try_from(u64::from(thunk_rva) + offset_in_array as u64)
                .map_err(|_| Error::format("import thunk RVA overflow"))?;
            let value = self.read_ptr_rva(entry_rva)?;
            if value == 0 {
                break;
            }

            let is_ordinal = if self.is_64 {
                value & (1u64 << 63) != 0
            } else {
                value & 0x8000_0000 != 0
            };

            let (name, ordinal, hint) = if is_ordinal {
                (None, Some((value & 0xffff) as u16), None)
            } else {
                if value > u64::from(u32::MAX) {
                    return Err(Error::format("import name RVA is out of range"));
                }
                let hint_name_rva = value as u32;
                let hint = self.read_u16_rva(hint_name_rva)?;
                let string_rva = hint_name_rva
                    .checked_add(2)
                    .ok_or_else(|| Error::format("import name RVA overflow"))?;
                let name = self.read_cstring_rva(string_rva, MAX_HINT_NAME_LEN)?;
                (Some(name), None, Some(hint))
            };

            let iat_offset = u64::try_from(index)
                .ok()
                .and_then(|index| index.checked_mul(pointer_size as u64))
                .ok_or_else(|| Error::format("import IAT offset overflow"))?;
            let iat_va = self
                .image_base
                .checked_add(u64::from(iat_rva))
                .and_then(|base| base.checked_add(iat_offset))
                .ok_or_else(|| Error::format("import IAT address overflow"))?;

            functions.push(ImportFunction {
                name,
                ordinal,
                hint,
                iat_va: Address(iat_va),
            });
        }

        Ok(functions)
    }

    /// Best-effort exports: a broken export directory must not fail the whole load.
    fn parse_exports(&self, directory: DataDirectory, warnings: &mut Vec<String>) -> Vec<Export> {
        match self.try_parse_exports(directory) {
            Ok(exports) => exports,
            Err(e) => {
                warnings.push(format!("export table: {e}"));
                Vec::new()
            }
        }
    }

    fn try_parse_exports(&self, directory: DataDirectory) -> Result<Vec<Export>> {
        if directory.rva == 0 {
            return Ok(Vec::new());
        }

        let header = self.read_at_rva::<40>(directory.rva)?;
        let name_rva = u32::from_le_bytes([header[12], header[13], header[14], header[15]]);
        let base = u32::from_le_bytes([header[16], header[17], header[18], header[19]]);
        let number_of_functions =
            u32::from_le_bytes([header[20], header[21], header[22], header[23]]) as usize;
        let number_of_names =
            u32::from_le_bytes([header[24], header[25], header[26], header[27]]) as usize;
        let functions_rva = u32::from_le_bytes([header[28], header[29], header[30], header[31]]);
        let names_rva = u32::from_le_bytes([header[32], header[33], header[34], header[35]]);
        let ordinals_rva = u32::from_le_bytes([header[36], header[37], header[38], header[39]]);

        let _dll_name = if name_rva != 0 {
            self.read_cstring_rva(name_rva, MAX_NAME_LEN).ok()
        } else {
            None
        };

        let functions_offset = if number_of_functions > 0 {
            if functions_rva == 0 {
                return Err(Error::format("export directory has no function table"));
            }
            Some(self.mapped(functions_rva)?)
        } else {
            None
        };

        let function_capacity = functions_offset
            .map(|offset| self.bytes.len().saturating_sub(offset) / 4)
            .unwrap_or(0);
        let function_count = number_of_functions.min(MAX_EXPORTS).min(function_capacity);

        let mut names_by_index: HashMap<u32, String> = HashMap::new();
        if number_of_names > 0 {
            if names_rva == 0 || ordinals_rva == 0 {
                return Err(Error::format(
                    "export directory has names but no names/ordinals table",
                ));
            }
            let names_offset = self.mapped(names_rva)?;
            let ordinals_offset = self.mapped(ordinals_rva)?;
            let name_capacity = self.bytes.len().saturating_sub(names_offset) / 4;
            let ordinal_capacity = self.bytes.len().saturating_sub(ordinals_offset) / 2;
            let name_count = number_of_names
                .min(MAX_EXPORTS)
                .min(name_capacity)
                .min(ordinal_capacity);

            let names = self
                .bytes
                .get(names_offset..names_offset + name_count * 4)
                .ok_or_else(|| Error::format("export names table extends past EOF"))?;
            let ordinals = self
                .bytes
                .get(ordinals_offset..ordinals_offset + name_count * 2)
                .ok_or_else(|| Error::format("export ordinals table extends past EOF"))?;

            for (name_entry, ordinal_entry) in names.chunks_exact(4).zip(ordinals.chunks_exact(2)) {
                let export_name_rva = u32::from_le_bytes([
                    name_entry[0],
                    name_entry[1],
                    name_entry[2],
                    name_entry[3],
                ]);
                let index = u16::from_le_bytes([ordinal_entry[0], ordinal_entry[1]]) as u32;
                if index as usize >= function_count {
                    continue;
                }
                let name = self.read_cstring_rva(export_name_rva, MAX_NAME_LEN)?;
                names_by_index.entry(index).or_insert(name);
            }
        }

        let Some(functions_offset) = functions_offset else {
            return Ok(Vec::new());
        };
        let functions = self
            .bytes
            .get(functions_offset..functions_offset + function_count * 4)
            .ok_or_else(|| Error::format("export function table extends past EOF"))?;

        let mut exports = Vec::new();
        for (index, entry) in functions.chunks_exact(4).enumerate() {
            let function_rva = u32::from_le_bytes([entry[0], entry[1], entry[2], entry[3]]);
            if function_rva == 0 {
                continue;
            }
            let ordinal = base
                .checked_add(index as u32)
                .ok_or_else(|| Error::format("export ordinal overflow"))?;

            let inside_directory = directory.size > 0
                && u64::from(function_rva) >= u64::from(directory.rva)
                && u64::from(function_rva) < u64::from(directory.rva) + u64::from(directory.size);
            let forwarder = if inside_directory {
                self.read_cstring_rva(function_rva, MAX_NAME_LEN).ok()
            } else {
                None
            };

            exports.push(Export {
                name: names_by_index.remove(&(index as u32)),
                ordinal,
                rva: function_rva,
                forwarder,
            });
        }

        Ok(exports)
    }

    fn parse_tls(&self, directory: DataDirectory) -> Vec<Address> {
        if directory.rva == 0 {
            return Vec::new();
        }
        self.try_parse_tls(directory).unwrap_or_default()
    }

    fn try_parse_tls(&self, directory: DataDirectory) -> Result<Vec<Address>> {
        let directory_offset = self.mapped(directory.rva)?;
        let callbacks_va = if self.is_64 {
            read_u64(self.bytes, directory_offset + 24)?
        } else {
            u64::from(read_u32(self.bytes, directory_offset + 12)?)
        };
        if callbacks_va == 0 {
            return Ok(Vec::new());
        }

        let callbacks_rva = self
            .va_to_rva(callbacks_va)
            .ok_or_else(|| Error::format("TLS callback array VA is outside the image"))?;
        let callbacks_offset = self.mapped(callbacks_rva)?;
        let pointer_size: usize = if self.is_64 { 8 } else { 4 };
        let limit =
            MAX_TLS_CALLBACKS.min(self.bytes.len().saturating_sub(callbacks_offset) / pointer_size);

        let mut callbacks = Vec::new();
        for index in 0..limit {
            let offset = callbacks_offset + index * pointer_size;
            let value = if self.is_64 {
                read_u64(self.bytes, offset)?
            } else {
                u64::from(read_u32(self.bytes, offset)?)
            };
            if value == 0 {
                return Ok(callbacks);
            }
            let callback_rva = self
                .va_to_rva(value)
                .ok_or_else(|| Error::format("TLS callback VA is outside the image"))?;
            if self.rva_to_file_offset(callback_rva).is_none() {
                return Err(Error::format("TLS callback points to unmapped memory"));
            }
            callbacks.push(Address(value));
        }

        Ok(callbacks)
    }

    fn parse_pdata(&self, directory: DataDirectory) -> Vec<FunctionRange> {
        if !self.is_64 || directory.rva == 0 || directory.size < 12 {
            return Vec::new();
        }
        let Some(offset) = self.rva_to_file_offset(directory.rva) else {
            return Vec::new();
        };
        let available = self.bytes.len().saturating_sub(offset as usize) / 12;
        let count = (directory.size as usize / 12)
            .min(MAX_PDATA_ENTRIES)
            .min(available);

        let mut functions = Vec::new();
        for index in 0..count {
            let entry_offset = offset as usize + index * 12;
            let Ok(begin) = read_u32(self.bytes, entry_offset) else {
                break;
            };
            let Ok(end) = read_u32(self.bytes, entry_offset + 4) else {
                break;
            };
            if begin == 0 || end <= begin {
                continue;
            }
            let (Some(start), Some(stop)) = (
                self.image_base.checked_add(u64::from(begin)),
                self.image_base.checked_add(u64::from(end)),
            ) else {
                continue;
            };
            functions.push(FunctionRange {
                start: Address(start),
                end: Address(stop),
            });
        }
        functions
    }

    fn read_at_rva<const N: usize>(&self, rva: u32) -> Result<[u8; N]> {
        read_array::<N>(self.bytes, self.mapped(rva)?, "structure")
    }
}

fn map_rva(file_size: u64, size_of_headers: u64, sections: &[Section], rva: u32) -> Option<u32> {
    if u64::from(rva) < size_of_headers.min(file_size) {
        return Some(rva);
    }
    for section in sections {
        let start = u64::from(section.rva);
        let target = u64::from(rva);
        if target >= start && target - start < u64::from(section.raw_size) {
            let offset = u64::from(section.raw_offset) + (target - start);
            if offset < file_size {
                return u32::try_from(offset).ok();
            }
        }
    }
    None
}

fn overlay_start(sections: &[Section], file_size: u64) -> Option<u64> {
    let mut maximum_end = 0u64;
    for section in sections {
        if section.raw_size == 0 {
            continue;
        }
        let end = u64::from(section.raw_offset) + u64::from(section.raw_size);
        maximum_end = maximum_end.max(end);
    }
    (maximum_end < file_size).then_some(maximum_end)
}

fn pe_offset(bytes: &[u8]) -> Option<usize> {
    if bytes.len() < 0x40 {
        return None;
    }
    if bytes.get(..2) != Some(b"MZ".as_slice()) {
        return None;
    }
    let raw = bytes.get(0x3c..0x40)?;
    let e_lfanew = u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]);
    let offset = usize::try_from(e_lfanew).ok()?;
    // PE signature (4) + COFF header (20) + optional magic (2).
    if offset.checked_add(26)? > bytes.len() {
        return None;
    }
    Some(offset)
}

fn read_array<const N: usize>(bytes: &[u8], offset: usize, what: &str) -> Result<[u8; N]> {
    let end = offset
        .checked_add(N)
        .ok_or_else(|| Error::format("offset overflow"))?;
    let slice = bytes.get(offset..end).ok_or_else(|| {
        Error::format(format!(
            "truncated {what} at offset {offset:#x} (file is {} bytes)",
            bytes.len()
        ))
    })?;
    let mut value = [0u8; N];
    value.copy_from_slice(slice);
    Ok(value)
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(read_array::<2>(bytes, offset, "u16")?))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(read_array::<4>(bytes, offset, "u32")?))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64> {
    Ok(u64::from_le_bytes(read_array::<8>(bytes, offset, "u64")?))
}

fn read_cstring(bytes: &[u8], offset: usize, maximum: usize) -> Result<String> {
    let end = offset
        .checked_add(maximum)
        .ok_or_else(|| Error::format("string offset overflow"))?
        .min(bytes.len());
    let slice = bytes
        .get(offset..end)
        .ok_or_else(|| Error::format(format!("string at offset {offset:#x} is past EOF")))?;
    let Some(length) = slice.iter().position(|&byte| byte == 0) else {
        return Err(Error::format(format!(
            "unterminated string at offset {offset:#x}"
        )));
    };
    Ok(String::from_utf8_lossy(&slice[..length]).into_owned())
}
