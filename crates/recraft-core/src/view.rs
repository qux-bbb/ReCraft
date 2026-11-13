// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Read-only view types shared by the CLI, UI and MCP integrations.

use recraft_common::{Address, Result};
use recraft_decode::{Insn, Mode};
use recraft_format::{Arch, Image};

use crate::command::StructDef;
use crate::flags::Flags;
use crate::segments::Segment;

/// Everything rendering needs to read from a project, behind `&self`.
///
/// [`crate::Session`] implements this directly, so a project can be rendered while it is
/// borrowed immutably. [`crate::SessionSnapshot`] implements it over a preloaded copy and is
/// `Sync`, which is what lets function-level work (lint, golden writes) run on several
/// threads without a `&mut Session` each.
pub trait SessionView {
    fn image(&self) -> &Image;

    fn segments(&self) -> &[Segment];

    fn mode(&self) -> Mode;

    fn arch(&self) -> Arch;

    /// Container format the project was imported from (PE, ELF, ...). Rendering needs it
    /// to pick the x64 calling convention (SysV on ELF, Microsoft x64 on PE).
    fn format(&self) -> recraft_format::Format;

    fn entry_point(&self) -> Address;

    /// All defined functions, ordered by start address.
    fn functions(&self) -> Result<Vec<FunctionInfo>>;

    fn name_at(&self, addr: Address) -> Result<Option<String>>;

    fn prototype_at(&self, addr: Address) -> Result<Option<String>>;

    /// Struct layout bound to a function argument slot (`0` = first argument).
    fn struct_for_arg(&self, func: Address, slot: u8) -> Result<Option<StructDef>>;

    /// Decode the instruction at `addr` (cached).
    fn insn_at(&self, addr: Address) -> Result<Option<Insn>>;

    /// Decode one instruction through the backend-neutral model.
    ///
    /// The default adapts [`Self::insn_at`] (the x86 backend); AArch64 sessions override it
    /// because an A64 instruction has no iced counterpart.
    fn decoded_at(&self, addr: Address) -> Result<Option<recraft_decode::DecodedInsn>> {
        Ok(self.insn_at(addr)?.map(|insn| insn.neutral()))
    }

    /// Copy up to `out.len()` image bytes starting at virtual address `addr`.
    fn read_at(&self, addr: Address, out: &mut [u8]) -> Result<usize>;

    /// Length and encoding of the stored string that starts at `addr`, if any.
    fn string_meta_at(&self, addr: Address) -> Result<Option<(u32, i64)>>;

    /// User names for the stack variables of the function at `func`, keyed by stack offset.
    /// The L2 renderer prints them instead of the generated `var_<offset>` slots.
    fn var_names(&self, _func: Address) -> std::collections::BTreeMap<i64, String> {
        std::collections::BTreeMap::new()
    }

    fn segment_containing(&self, addr: Address) -> Option<&Segment> {
        self.segments()
            .iter()
            .find(|segment| segment.contains(addr))
    }

    fn read_bytes(&self, addr: Address, len: usize) -> Result<Vec<u8>> {
        let mut buffer = vec![0u8; len];
        let read = self.read_at(addr, &mut buffer)?;
        buffer.truncate(read);
        Ok(buffer)
    }

    /// Stored string text at `addr`, decoded from its encoding (1 = ASCII, 2 = UTF-16LE).
    ///
    /// Returns `None` when the address is not the start of a stored string.
    fn string_at(&self, addr: Address) -> Result<Option<String>> {
        let Some((len, enc)) = self.string_meta_at(addr)? else {
            return Ok(None);
        };
        let mut bytes = vec![0u8; len.min(4096) as usize];
        let read = self.read_at(addr, &mut bytes)?;
        bytes.truncate(read);
        let text = if enc == 2 {
            let units: Vec<u16> = bytes
                .chunks_exact(2)
                .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
                .take_while(|unit| *unit != 0)
                .collect();
            String::from_utf16_lossy(&units)
        } else {
            let end = bytes
                .iter()
                .position(|byte| *byte == 0)
                .unwrap_or(bytes.len());
            String::from_utf8_lossy(&bytes[..end]).into_owned()
        };
        Ok(Some(text))
    }
}

/// One rendered line of the linear disassembly view.
#[derive(Debug, Clone)]
pub struct DisasmLine {
    pub addr: Address,
    /// Instruction length in bytes (1..=15).
    pub len: u8,
    /// Up to 15 instruction bytes, zero-padded; use [`DisasmLine::bytes`].
    pub raw: [u8; 15],
    pub mnemonic: String,
    pub operands: String,
    pub name: Option<String>,
    pub comment: Option<String>,
    pub flags: Flags,
}

impl DisasmLine {
    /// The instruction bytes (`len` bytes).
    pub fn bytes(&self) -> &[u8] {
        &self.raw[..usize::from(self.len.min(15))]
    }
}

/// A function record (start .. end, half-open).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FunctionInfo {
    pub start: Address,
    pub end: Address,
}

/// One extracted string, as stored in the `string` table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringEntry {
    pub addr: Address,
    /// Length of the literal's text in bytes (the terminator is not counted).
    pub len: u32,
    /// Encoding: see `recraft_analysis` string encoding constants.
    pub enc: i64,
    pub kind: i64,
}

/// A user edit on one extracted string: replacement text and/or a deletion marker.
///
/// Analysis owns the derived `string` table; this record is user data, so a rebuild keeps
/// the correction. `text` replaces the decoded bytes in the Strings view and in the read
/// tools, while the decompiler keeps reading the real bytes (a literal's meaning must not
/// depend on an annotation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringOverride {
    /// Text the user typed, or `None` to show the decoded bytes.
    pub text: Option<String>,
    /// The user deleted this string from the list.
    pub hidden: bool,
}

/// One cross-reference, as stored in the `xref` table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XrefEntry {
    pub from: Address,
    pub to: Address,
    /// Kind: see `recraft_analysis` xref kind constants.
    pub kind: i64,
}
