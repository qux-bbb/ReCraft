// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Virtual address space: segments derived from the loader output.

use recraft_common::{Address, AddressRange};
use recraft_format::Image;
use serde::{Deserialize, Serialize};

/// What a segment is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SegmentKind {
    Headers,
    Code,
    Data,
    /// Virtual-only data (`.bss`) with no file backing.
    Bss,
}

/// Access permissions of a mapped segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Perm {
    None,
    Read,
    ReadWrite,
    ReadExecute,
    ReadWriteExecute,
}

impl Perm {
    pub const fn from_flags(read: bool, write: bool, execute: bool) -> Self {
        match (read, write, execute) {
            (true, true, true) => Perm::ReadWriteExecute,
            (true, true, false) => Perm::ReadWrite,
            (true, false, true) => Perm::ReadExecute,
            (true, false, false) => Perm::Read,
            _ => Perm::None,
        }
    }

    pub const fn is_readable(self) -> bool {
        !matches!(self, Perm::None)
    }

    pub const fn is_writable(self) -> bool {
        matches!(self, Perm::ReadWrite | Perm::ReadWriteExecute)
    }

    pub const fn is_executable(self) -> bool {
        matches!(self, Perm::ReadExecute | Perm::ReadWriteExecute)
    }
}

/// One contiguous mapped region.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    pub name: String,
    pub range: AddressRange,
    /// File offset of the first byte, for file-backed segments.
    pub file_offset: u64,
    /// Number of bytes backed by file data (raw size; 0 for pure virtual data).
    pub file_size: u64,
    pub kind: SegmentKind,
    pub perm: Perm,
}

impl Segment {
    #[inline]
    pub fn contains(&self, addr: Address) -> bool {
        self.range.contains(addr)
    }

    #[inline]
    pub fn len(&self) -> u64 {
        self.range.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.range.is_empty()
    }

    /// Whether `[addr, addr+len)` is fully inside this segment.
    #[inline]
    pub fn contains_range(&self, range: AddressRange) -> bool {
        self.range.contains_range(range)
    }
}

/// Build the initial segment list from a parsed image.
///
/// Node: PE sections may overlap in the wild; we keep them as-is (sorted by address)
/// and let callers pick the first match. The headers segment is only added when it
/// does not overlap the first section.
pub fn from_image(image: &Image) -> Vec<Segment> {
    let image_base = image.image_base.get();
    let image_end = image_base.saturating_add(image.size_of_image);
    let mut segments = Vec::new();

    if image.size_of_headers > 0 {
        let start = image_base;
        let end = start
            .saturating_add(image.size_of_headers)
            .min(image_end.max(start + 1));
        segments.push(Segment {
            name: "HEADER".to_owned(),
            range: AddressRange::new(Address::new(start), Address::new(end)),
            file_offset: 0,
            file_size: image.size_of_headers.min(end - start),
            kind: SegmentKind::Headers,
            perm: Perm::Read,
        });
    }

    for section in &image.sections {
        let virtual_size = section.virtual_size;
        let raw_size = u64::from(section.raw_size);
        let size = virtual_size.max(raw_size);
        if size == 0 {
            continue;
        }
        let start = section.va.get();
        let end = start.saturating_add(size).min(image_end);
        if end <= start {
            continue;
        }
        let kind = if section.is_executable() {
            SegmentKind::Code
        } else if virtual_size > raw_size {
            SegmentKind::Bss
        } else {
            SegmentKind::Data
        };
        let perm = Perm::from_flags(
            section.is_readable(),
            section.is_writable(),
            section.is_executable(),
        );
        segments.push(Segment {
            name: section.name.clone(),
            range: AddressRange::new(Address::new(start), Address::new(end)),
            file_offset: u64::from(section.raw_offset),
            file_size: raw_size.min(size),
            kind,
            perm,
        });
    }

    segments.sort_by_key(|segment| segment.range.start);
    // Real ELF objects alias addresses: `.tbss` starts exactly where `.bss` does, and
    // a note section can sit inside the header range. The address space is a single
    // mapping, so keep the first (lowest) view and drop later segments that start
    // inside it; adjacent segments (start == previous end) are kept.
    let mut deduped: Vec<Segment> = Vec::with_capacity(segments.len());
    for segment in segments {
        let overlaps = deduped
            .last()
            .is_some_and(|previous| segment.range.start < previous.range.end);
        if overlaps {
            continue;
        }
        deduped.push(segment);
    }
    deduped
}

#[cfg(test)]
mod tests {
    use super::*;
    use recraft_format::Section;

    fn image(sections: Vec<Section>) -> Image {
        Image {
            is_64: true,
            image_base: Address::new(0x1400_0000),
            entry_point: Address::new(0x1400_1000),
            size_of_image: 0x3000,
            size_of_headers: 0x400,
            subsystem: 3,
            timestamp: 0,
            sections,
            imports: Vec::new(),
            exports: Vec::new(),
            tls_callbacks: Vec::new(),
            pdata_functions: Vec::new(),
            symbols: Vec::new(),
            overlay_file_offset: None,
            file_size: 0x3000,
            warnings: Vec::new(),
        }
    }

    fn section(name: &str, va: u64, virtual_size: u64, raw_size: u32) -> Section {
        Section {
            name: name.to_owned(),
            va: Address::new(va),
            virtual_size,
            raw_offset: 0x400,
            raw_size,
            characteristics: 0x4000_0040,
            ..Section::default()
        }
    }

    #[test]
    fn builds_headers_and_sections() {
        let image = image(vec![
            Section {
                name: ".text".to_owned(),
                va: Address::new(0x1400_1000),
                virtual_size: 0x1000,
                raw_offset: 0x400,
                raw_size: 0x1000,
                characteristics: 0x6000_0020,
                ..Section::default()
            },
            Section {
                name: ".data".to_owned(),
                va: Address::new(0x1400_2000),
                virtual_size: 0x800,
                raw_offset: 0x1400,
                raw_size: 0x400,
                characteristics: 0xC000_0040,
                ..Section::default()
            },
        ]);
        let segments = from_image(&image);
        assert_eq!(segments.len(), 3);

        let headers = &segments[0];
        assert_eq!(headers.name, "HEADER");
        assert_eq!(headers.kind, SegmentKind::Headers);
        assert!(headers.perm.is_readable());

        let text = &segments[1];
        assert_eq!(text.kind, SegmentKind::Code);
        assert!(text.perm.is_executable());
        assert_eq!(text.file_offset, 0x400);
        assert!(text.contains(Address::new(0x1400_1000)));
        assert!(text.contains(Address::new(0x1400_1fff)));
        assert!(!text.contains(Address::new(0x1400_2000)));

        let data = &segments[2];
        assert_eq!(data.kind, SegmentKind::Bss); // virtual size > raw size
        assert!(data.perm.is_writable());
    }

    #[test]
    fn aliased_sections_do_not_duplicate_a_segment_start() {
        // `.tbss` starts exactly where `.bss` does in real ELF objects (and PE can
        // alias too); the address space is one mapping, so only the first survives.
        let segments = from_image(&image(vec![
            section(".data", 0x1400_1000, 0x200, 0x200),
            section(".bss", 0x1400_2000, 0x400, 0),
            section(".tbss", 0x1400_2000, 0x400, 0),
            section(".edata", 0x1400_2600, 0x100, 0x100),
        ]));
        let starts: Vec<u64> = segments
            .iter()
            .map(|segment| segment.range.start.get())
            .collect();
        let mut unique = starts.clone();
        unique.dedup();
        assert_eq!(starts, unique, "duplicate segment starts: {starts:?}");
        assert!(segments.iter().any(|segment| segment.name == ".edata"));
        assert!(segments.iter().any(|segment| segment.name == ".bss"));
        assert!(!segments.iter().any(|segment| segment.name == ".tbss"));
    }
}
