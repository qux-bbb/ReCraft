// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Read-only view types shared by the CLI, UI and MCP integrations.

use recraft_common::Address;

use crate::flags::Flags;

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
