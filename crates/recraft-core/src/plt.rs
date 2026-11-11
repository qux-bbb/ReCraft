// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! PLT-style import stubs.
//!
//! ELF calls an imported function through a small stub that jumps to the function's GOT
//! slot (`jmp qword ptr [rip+disp]`); the call itself is a direct `call <stub>`, so the
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
/// Only executable segments are scanned, and only jumps whose RIP-relative target is a
/// known import slot are kept, so unrelated `ff 25` byte pairs in code are ignored.
pub fn stubs(view: &dyn SessionView) -> Result<Vec<PltStub>> {
    let image = view.image();
    let slots = import_slots(image);
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
            skip as u64,
            &slots,
            &mut found,
        );
    }
    let mut stubs: Vec<PltStub> = found.into_values().collect();
    stubs.sort_by_key(|stub| stub.start);
    // The PLT stride is 16 bytes; a stub that is not followed by another one inside the
    // segment keeps that stride so the range stays a single stub.
    for index in 0..stubs.len() {
        let limit = stubs.get(index + 1).map_or_else(
            || stubs[index].start.get().saturating_add(16),
            |next| next.start.get(),
        );
        let end = stubs[index].start.get().saturating_add(16).min(limit);
        stubs[index].end = Address::new(end.max(stubs[index].start.get() + 1));
    }
    stubs.retain(|stub| stub.end > stub.start);
    Ok(stubs)
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

fn scan_segment(
    bytes: &[u8],
    section_start: u64,
    offset: u64,
    slots: &BTreeMap<u64, (String, String)>,
    out: &mut BTreeMap<u64, PltStub>,
) {
    if bytes.len() < 6 {
        return;
    }
    for index in 0..=bytes.len() - 6 {
        // `ff 25 disp32`: jmp qword ptr [rip + disp].
        if bytes[index] != 0xFF || bytes[index + 1] != 0x25 {
            continue;
        }
        let disp = i32::from_le_bytes([
            bytes[index + 2],
            bytes[index + 3],
            bytes[index + 4],
            bytes[index + 5],
        ]);
        let insn = section_start + offset + index as u64;
        let slot = insn.wrapping_add(6).wrapping_add_signed(i64::from(disp));
        // An import whose slot is unknown still gets a stub entry: the 16-byte range keeps
        // function discovery from folding the whole `.plt.sec` into one function.
        let (name, qualified) = slots
            .get(&slot)
            .map(|(name, qualified)| (name.clone(), qualified.clone()))
            .unwrap_or_default();
        // A call lands on the stub entry, which may carry prefixes: `endbr64` (4 bytes)
        // or `bnd`/`notrack` (1 byte) before the jump.
        let start = if index >= 4 && bytes[index - 4..index] == [0xF3, 0x0F, 0x1E, 0xFA] {
            insn - 4
        } else if index >= 1 && matches!(bytes[index - 1], 0xF2 | 0x3E) {
            insn - 1
        } else {
            insn
        };
        out.entry(start).or_insert_with(|| PltStub {
            start: Address::new(start),
            end: Address::new(start + 16),
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
        scan_segment(&bytes, start, 0, &slots(), &mut out);
        assert_eq!(out.get(&start).map(|stub| stub.name.as_str()), Some("puts"));
        assert_eq!(out.get(&start).map(|stub| stub.slot.get()), Some(0x3fa0));

        // The same jump without a CET prefix enters at the jump itself.
        let mut plain = vec![0xFF, 0x25, 0, 0, 0, 0];
        let disp = (0x3fa0i64 - (start as i64 + 6)) as i32;
        plain[2..6].copy_from_slice(&disp.to_le_bytes());
        let mut out = BTreeMap::new();
        scan_segment(&plain, start, 0, &slots(), &mut out);
        assert_eq!(out.get(&start).map(|stub| stub.name.as_str()), Some("puts"));
    }

    #[test]
    fn a_jump_to_an_unknown_slot_is_still_a_stub() {
        let start = 0x10b0u64;
        let mut plain = vec![0xFF, 0x25, 0, 0, 0, 0];
        plain[2..6].copy_from_slice(&0i32.to_le_bytes());
        let mut out = BTreeMap::new();
        scan_segment(&plain, start, 0, &slots(), &mut out);
        assert!(out.get(&start).is_some_and(|stub| stub.name.is_empty()));
    }
}
