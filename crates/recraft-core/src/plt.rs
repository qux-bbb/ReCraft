// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! PLT-style import stubs.
//!
//! ELF calls an imported function through a small stub that jumps to the function's GOT
//! slot (`jmp qword ptr [rip+disp]` on x86-64, `jmp dword ptr [disp]` or
//! `jmp dword ptr [ebx+disp]` on i386); the call itself is a direct `call <stub>`, so the
//! caller never touches the slot. PE spells the same thing with `call [IAT]`, which the
//! decompiler already resolves. This module finds the stub ranges and their import, so
//! both the API-tag pass and the decompiler can render `puts(...)` instead of `sub_10b0()`.

use std::collections::BTreeMap;

use recraft_common::{Address, Result};
use recraft_format::Image;

use crate::view::SessionView;

/// One `jmp qword ptr [import_slot]` stub.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PltStub {
    /// First byte of the stub (the address a `call` lands on).
    pub start: Address,
    /// Half-open end: the next stub start, or the 16-byte PLT stride.
    pub end: Address,
    /// The import slot the stub jumps through (GOT entry on ELF, IAT slot on PE).
    pub slot: Address,
    /// Imported function name without the module (`puts`); empty when the slot does not
    /// resolve to a named import.
    pub name: String,
    /// `module!name`, for API tags and disassembly comments; empty with `name`.
    pub qualified: String,
}

impl PltStub {
    /// True when `addr` is the stub entry or one of the prefix bytes an earlier
    /// instruction may have called (`endbr64`/`bnd`/`notrack` sit before the jump).
    pub fn covers(&self, addr: Address) -> bool {
        addr >= self.start && addr < self.end
    }
}

/// Find every PLT-style import stub in the image.
///
/// Only executable segments are scanned, and only jumps whose target is a known import
/// slot are named, so unrelated `ff 25`/`ff a3` byte pairs in code are ignored.
pub fn stubs(view: &dyn SessionView) -> Result<Vec<PltStub>> {
    let image = view.image();
    let slots = import_slots(image);
    let is_64 = image.is_64;
    // i386 PIC stubs jump through `[ebx + disp]`, where `ebx` holds the address of
    // `_GLOBAL_OFFSET_TABLE_` — the start of `.got.plt` in the usual layout. Without
    // that base the displacement alone means nothing.
    let got_base = if is_64 { None } else { got_base(image) };
    let mut found: BTreeMap<u64, PltStub> = BTreeMap::new();
    for segment in view.segments() {
        // Only the PLT sections are stub tables; a `jmp [rip+disp]` elsewhere is code.
        if !segment.perm.is_executable()
            || segment.file_size == 0
            || !segment.name.starts_with(".plt")
        {
            continue;
        }
        let size = segment.file_size.min(segment.len()) as usize;
        if size < 6 {
            continue;
        }
        let Ok(bytes) = view.read_bytes(segment.range.start, size) else {
            continue;
        };
        // `.plt` starts with the resolver stub (PLT0), which is not an import stub.
        let skip = if segment.name == ".plt" {
            16.min(size)
        } else {
            0
        };
        scan_segment(
            &bytes[skip..],
            segment.range.start.get(),
            segment.range.end.get(),
            skip as u64,
            is_64,
            got_base,
            &slots,
            &mut found,
        );
    }
    let mut stubs: Vec<PltStub> = found.into_values().collect();
    stubs.sort_by_key(|stub| stub.start);
    // The PLT stride is 16 bytes; a stub that is not followed by another one inside the
    // segment keeps that stride so the range stays a single stub, capped at the section's
    // end (the scan already clamped it) and at the next stub.
    for index in 0..stubs.len() {
        let stride_end = stubs[index].start.get().saturating_add(16);
        let next_start = stubs
            .get(index + 1)
            .map_or(stride_end, |next| next.start.get());
        let section_end = stubs[index].end.get();
        let end = stride_end.min(next_start).min(section_end);
        stubs[index].end = Address::new(end.max(stubs[index].start.get() + 1));
    }
    stubs.retain(|stub| stub.end > stub.start);
    Ok(stubs)
}

/// The address i386 PIC code keeps in `ebx`: the GOT base, conventionally `.got.plt`.
fn got_base(image: &Image) -> Option<u64> {
    image
        .sections
        .iter()
        .find(|section| section.name == ".got.plt")
        .or_else(|| image.sections.iter().find(|section| section.name == ".got"))
        .map(|section| section.va.get())
}

/// Import slot address -> (plain name, `module!name`).
fn import_slots(image: &Image) -> BTreeMap<u64, (String, String)> {
    let mut slots = BTreeMap::new();
    for module in &image.imports {
        for function in &module.functions {
            let Some(name) = function
                .name
                .clone()
                .or_else(|| function.ordinal.map(|ordinal| format!("ord_{ordinal}")))
            else {
                continue;
            };
            if function.iat_va.get() == 0 {
                continue;
            }
            let qualified = format!("{}!{name}", module.name);
            slots.insert(function.iat_va.get(), (name, qualified));
        }
    }
    slots
}

#[allow(clippy::too_many_arguments)]
fn scan_segment(
    bytes: &[u8],
    section_start: u64,
    section_end: u64,
    offset: u64,
    is_64: bool,
    got_base: Option<u64>,
    slots: &BTreeMap<u64, (String, String)>,
    out: &mut BTreeMap<u64, PltStub>,
) {
    if bytes.len() < 6 {
        return;
    }
    for index in 0..=bytes.len() - 6 {
        if bytes[index] != 0xFF {
            continue;
        }
        let insn = section_start + offset + index as u64;
        let disp = i32::from_le_bytes([
            bytes[index + 2],
            bytes[index + 3],
            bytes[index + 4],
            bytes[index + 5],
        ]);
        let slot = match (is_64, bytes[index + 1]) {
            // `ff 25 disp32`: jmp qword ptr [rip + disp] on x86-64, and the absolute
            // `jmp dword ptr [disp32]` form on i386 non-PIE images.
            (true, 0x25) => insn.wrapping_add(6).wrapping_add_signed(i64::from(disp)),
            (false, 0x25) => u64::from(disp as u32),
            // `ff a3 disp32`: jmp dword ptr [ebx + disp], the i386 PIC form, where ebx
            // holds the GOT base.
            (false, 0xa3) => match got_base {
                Some(base) => base.wrapping_add_signed(i64::from(disp)),
                None => 0,
            },
            _ => continue,
        };
        // An import whose slot is unknown still gets a stub entry: the 16-byte range keeps
        // function discovery from folding the whole `.plt.sec` into one function.
        let (name, qualified) = slots
            .get(&slot)
            .map(|(name, qualified)| (name.clone(), qualified.clone()))
            .unwrap_or_default();
        // A call lands on the stub entry, which may carry prefixes: `endbr64`/`endbr32`
        // (4 bytes) or `bnd`/`notrack` (1 byte) before the jump.
        let endbr = [0xF3, 0x0F, 0x1E, 0xFA];
        let endbr32 = [0xF3, 0x0F, 0x1E, 0xFB];
        let start = if index >= 4
            && (bytes[index - 4..index] == endbr || bytes[index - 4..index] == endbr32)
        {
            insn - 4
        } else if index >= 1 && matches!(bytes[index - 1], 0xF2 | 0x3E) {
            insn - 1
        } else {
            insn
        };
        out.entry(start).or_insert_with(|| PltStub {
            start: Address::new(start),
            // The stride is a maximum: a short trailing section (`.plt.got` is often
            // eight bytes) must not claim bytes that belong to the next segment.
            end: Address::new((start + 16).min(section_end.max(start + 1))),
            slot: Address::new(slot),
            name: name.clone(),
            qualified: qualified.clone(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slots() -> BTreeMap<u64, (String, String)> {
        let mut slots = BTreeMap::new();
        slots.insert(0x3fa0, ("puts".to_owned(), "libc.so.6!puts".to_owned()));
        slots
    }

    #[test]
    fn stubs_are_found_with_and_without_endbr() {
        let start = 0x10b0u64;
        // `endbr64; jmp qword ptr [rip+disp]`.
        let mut bytes = vec![0xF3, 0x0F, 0x1E, 0xFA, 0xFF, 0x25, 0, 0, 0, 0];
        let disp = (0x3fa0i64 - (start as i64 + 4 + 6)) as i32;
        bytes[6..10].copy_from_slice(&disp.to_le_bytes());
        let mut out = BTreeMap::new();
        scan_segment(
            &bytes,
            start,
            start + 0x10,
            0,
            true,
            None,
            &slots(),
            &mut out,
        );
        assert_eq!(out.get(&start).map(|stub| stub.name.as_str()), Some("puts"));
        assert_eq!(out.get(&start).map(|stub| stub.slot.get()), Some(0x3fa0));

        // The same jump without a CET prefix enters at the jump itself.
        let mut plain = vec![0xFF, 0x25, 0, 0, 0, 0];
        let disp = (0x3fa0i64 - (start as i64 + 6)) as i32;
        plain[2..6].copy_from_slice(&disp.to_le_bytes());
        let mut out = BTreeMap::new();
        scan_segment(
            &plain,
            start,
            start + 0x10,
            0,
            true,
            None,
            &slots(),
            &mut out,
        );
        assert_eq!(out.get(&start).map(|stub| stub.name.as_str()), Some("puts"));
    }

    #[test]
    fn a_short_section_clamps_the_stub_end() {
        // `.plt.got` is often eight bytes: the stub must not claim the next section's
        // first bytes (they belong to `.text`).
        let start = 0x1090u64;
        let bytes = [0xFF, 0x25, 0, 0, 0, 0, 0, 0];
        let mut out = BTreeMap::new();
        scan_segment(&bytes, start, start + 8, 0, true, None, &slots(), &mut out);
        assert_eq!(out.get(&start).map(|stub| stub.end.get()), Some(start + 8));
    }

    #[test]
    fn i386_absolute_and_got_relative_stubs_are_found() {
        let mut i386 = BTreeMap::new();
        i386.insert(
            0x0804_a000u64,
            ("puts".to_owned(), "libc.so.6!puts".to_owned()),
        );

        // Non-PIE: `endbr32; jmp dword ptr [0x0804a000]` (absolute disp32).
        let start = 0x0804_9000u64;
        let mut bytes = vec![0xF3, 0x0F, 0x1E, 0xFB, 0xFF, 0x25, 0, 0, 0, 0];
        bytes[6..10].copy_from_slice(&0x0804_a000u32.to_le_bytes());
        let mut out = BTreeMap::new();
        scan_segment(&bytes, start, start + 0x10, 0, false, None, &i386, &mut out);
        assert_eq!(out.get(&start).map(|stub| stub.name.as_str()), Some("puts"));
        assert_eq!(
            out.get(&start).map(|stub| stub.slot.get()),
            Some(0x0804_a000)
        );

        // PIC: `jmp dword ptr [ebx + disp]`, with ebx = `.got.plt` at 0x0804b000.
        let got = 0x0804_b000u64;
        let mut pic = vec![0xFF, 0xA3, 0, 0, 0, 0];
        let disp = (0x0804_a000i64 - got as i64) as i32;
        pic[2..6].copy_from_slice(&disp.to_le_bytes());
        let mut out = BTreeMap::new();
        scan_segment(
            &pic,
            start,
            start + 0x10,
            0,
            false,
            Some(got),
            &i386,
            &mut out,
        );
        assert_eq!(out.get(&start).map(|stub| stub.name.as_str()), Some("puts"));

        // Without a GOT base the stub is still recognised; only the name is unknown.
        let mut out = BTreeMap::new();
        scan_segment(&pic, start, start + 0x10, 0, false, None, &i386, &mut out);
        assert!(out.get(&start).is_some_and(|stub| stub.name.is_empty()));
    }

    #[test]
    fn a_jump_to_an_unknown_slot_is_still_a_stub() {
        let start = 0x10b0u64;
        let mut plain = vec![0xFF, 0x25, 0, 0, 0, 0];
        plain[2..6].copy_from_slice(&0i32.to_le_bytes());
        let mut out = BTreeMap::new();
        scan_segment(
            &plain,
            start,
            start + 0x10,
            0,
            true,
            None,
            &slots(),
            &mut out,
        );
        assert!(out.get(&start).is_some_and(|stub| stub.name.is_empty()));
    }
}
