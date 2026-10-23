// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Register model: a base register plus a width/bit-offset view.
//!
//! `al`, `ax`, `eax` and `rax` are all views of [`Reg::Rax`], which is what makes
//! partial-register handling explicit instead of a pile of special cases later.

use recraft_decode::iced_x86;
use serde::{Deserialize, Serialize};

/// General-purpose and SSE registers. XMM views are only used by the lifter to keep
/// SIMD/float code out of the intrinsic fallback; their arithmetic prints lane-wise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Reg {
    Rax,
    Rcx,
    Rdx,
    Rbx,
    Rsp,
    Rbp,
    Rsi,
    Rdi,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,
    Xmm0,
    Xmm1,
    Xmm2,
    Xmm3,
    Xmm4,
    Xmm5,
    Xmm6,
    Xmm7,
    Xmm8,
    Xmm9,
    Xmm10,
    Xmm11,
    Xmm12,
    Xmm13,
    Xmm14,
    Xmm15,
}

impl Reg {
    const NAMES: [&'static str; 32] = [
        "rax", "rcx", "rdx", "rbx", "rsp", "rbp", "rsi", "rdi", "r8", "r9", "r10", "r11", "r12",
        "r13", "r14", "r15", "xmm0", "xmm1", "xmm2", "xmm3", "xmm4", "xmm5", "xmm6", "xmm7",
        "xmm8", "xmm9", "xmm10", "xmm11", "xmm12", "xmm13", "xmm14", "xmm15",
    ];

    /// 64-bit (or full XMM) name of this register.
    pub fn name64(self) -> &'static str {
        Self::NAMES[self as usize]
    }

    /// Name for a width in bytes (1, 2, 4, 8), ignoring high-byte views.
    pub fn name_width(self, width: u8) -> &'static str {
        if matches!(
            self,
            Reg::Xmm0
                | Reg::Xmm1
                | Reg::Xmm2
                | Reg::Xmm3
                | Reg::Xmm4
                | Reg::Xmm5
                | Reg::Xmm6
                | Reg::Xmm7
                | Reg::Xmm8
                | Reg::Xmm9
                | Reg::Xmm10
                | Reg::Xmm11
                | Reg::Xmm12
                | Reg::Xmm13
                | Reg::Xmm14
                | Reg::Xmm15
        ) {
            return self.name64();
        }
        match width {
            8 => self.name64(),
            4 => match self {
                Reg::Rax => "eax",
                Reg::Rcx => "ecx",
                Reg::Rdx => "edx",
                Reg::Rbx => "ebx",
                Reg::Rsp => "esp",
                Reg::Rbp => "ebp",
                Reg::Rsi => "esi",
                Reg::Rdi => "edi",
                Reg::R8 => "r8d",
                Reg::R9 => "r9d",
                Reg::R10 => "r10d",
                Reg::R11 => "r11d",
                Reg::R12 => "r12d",
                Reg::R13 => "r13d",
                Reg::R14 => "r14d",
                Reg::R15 => "r15d",
                _ => self.name64(),
            },
            2 => match self {
                Reg::Rax => "ax",
                Reg::Rcx => "cx",
                Reg::Rdx => "dx",
                Reg::Rbx => "bx",
                Reg::Rsp => "sp",
                Reg::Rbp => "bp",
                Reg::Rsi => "si",
                Reg::Rdi => "di",
                Reg::R8 => "r8w",
                Reg::R9 => "r9w",
                Reg::R10 => "r10w",
                Reg::R11 => "r11w",
                Reg::R12 => "r12w",
                Reg::R13 => "r13w",
                Reg::R14 => "r14w",
                Reg::R15 => "r15w",
                _ => self.name64(),
            },
            1 => match self {
                Reg::Rax => "al",
                Reg::Rcx => "cl",
                Reg::Rdx => "dl",
                Reg::Rbx => "bl",
                Reg::Rsp => "spl",
                Reg::Rbp => "bpl",
                Reg::Rsi => "sil",
                Reg::Rdi => "dil",
                Reg::R8 => "r8b",
                Reg::R9 => "r9b",
                Reg::R10 => "r10b",
                Reg::R11 => "r11b",
                Reg::R12 => "r12b",
                Reg::R13 => "r13b",
                Reg::R14 => "r14b",
                Reg::R15 => "r15b",
                _ => self.name64(),
            },
            _ => self.name64(),
        }
    }

    /// High-byte name (only valid for ah/ch/dh/bh).
    fn high_byte_name(self) -> Option<&'static str> {
        match self {
            Reg::Rax => Some("ah"),
            Reg::Rcx => Some("ch"),
            Reg::Rdx => Some("dh"),
            Reg::Rbx => Some("bh"),
            _ => None,
        }
    }
}

/// A view into a register: `(base, width, bit_offset)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RegView {
    pub base: Reg,
    /// Width in bytes: 1, 2, 4 or 8.
    pub width: u8,
    /// Offset in bits (0, or 8 for high-byte registers).
    pub offset: u8,
}

impl RegView {
    #[inline]
    pub const fn new(base: Reg, width: u8, offset: u8) -> Self {
        RegView {
            base,
            width,
            offset,
        }
    }

    #[inline]
    pub const fn full(base: Reg) -> Self {
        RegView {
            base,
            width: 8,
            offset: 0,
        }
    }

    /// C-style name: `rax`, `eax`, `al`, `ah`, ...
    pub fn name(self) -> &'static str {
        if self.offset == 8 && self.width == 1 {
            if let Some(name) = self.base.high_byte_name() {
                return name;
            }
        }
        self.base.name_width(self.width)
    }
}

/// Convert an iced register into a ReCraft register view, if it is a GPR.
pub fn from_iced(register: iced_x86::Register) -> Option<RegView> {
    use Reg::*;
    use iced_x86::Register as R;
    let (base, width, offset) = match register {
        R::RAX => (Rax, 8, 0),
        R::EAX => (Rax, 4, 0),
        R::AX => (Rax, 2, 0),
        R::AL => (Rax, 1, 0),
        R::AH => (Rax, 1, 8),
        R::RCX => (Rcx, 8, 0),
        R::ECX => (Rcx, 4, 0),
        R::CX => (Rcx, 2, 0),
        R::CL => (Rcx, 1, 0),
        R::CH => (Rcx, 1, 8),
        R::RDX => (Rdx, 8, 0),
        R::EDX => (Rdx, 4, 0),
        R::DX => (Rdx, 2, 0),
        R::DL => (Rdx, 1, 0),
        R::DH => (Rdx, 1, 8),
        R::RBX => (Rbx, 8, 0),
        R::EBX => (Rbx, 4, 0),
        R::BX => (Rbx, 2, 0),
        R::BL => (Rbx, 1, 0),
        R::BH => (Rbx, 1, 8),
        R::RSP => (Rsp, 8, 0),
        R::ESP => (Rsp, 4, 0),
        R::SP => (Rsp, 2, 0),
        R::SPL => (Rsp, 1, 0),
        R::RBP => (Rbp, 8, 0),
        R::EBP => (Rbp, 4, 0),
        R::BP => (Rbp, 2, 0),
        R::BPL => (Rbp, 1, 0),
        R::RSI => (Rsi, 8, 0),
        R::ESI => (Rsi, 4, 0),
        R::SI => (Rsi, 2, 0),
        R::SIL => (Rsi, 1, 0),
        R::RDI => (Rdi, 8, 0),
        R::EDI => (Rdi, 4, 0),
        R::DI => (Rdi, 2, 0),
        R::DIL => (Rdi, 1, 0),
        R::R8 => (R8, 8, 0),
        R::R8D => (R8, 4, 0),
        R::R8W => (R8, 2, 0),
        R::R8L => (R8, 1, 0),
        R::R9 => (R9, 8, 0),
        R::R9D => (R9, 4, 0),
        R::R9W => (R9, 2, 0),
        R::R9L => (R9, 1, 0),
        R::R10 => (R10, 8, 0),
        R::R10D => (R10, 4, 0),
        R::R10W => (R10, 2, 0),
        R::R10L => (R10, 1, 0),
        R::R11 => (R11, 8, 0),
        R::R11D => (R11, 4, 0),
        R::R11W => (R11, 2, 0),
        R::R11L => (R11, 1, 0),
        R::R12 => (R12, 8, 0),
        R::R12D => (R12, 4, 0),
        R::R12W => (R12, 2, 0),
        R::R12L => (R12, 1, 0),
        R::R13 => (R13, 8, 0),
        R::R13D => (R13, 4, 0),
        R::R13W => (R13, 2, 0),
        R::R13L => (R13, 1, 0),
        R::R14 => (R14, 8, 0),
        R::R14D => (R14, 4, 0),
        R::R14W => (R14, 2, 0),
        R::R14L => (R14, 1, 0),
        R::R15 => (R15, 8, 0),
        R::R15D => (R15, 4, 0),
        R::R15W => (R15, 2, 0),
        R::R15L => (R15, 1, 0),
        R::XMM0 => (Xmm0, 16, 0),
        R::XMM1 => (Xmm1, 16, 0),
        R::XMM2 => (Xmm2, 16, 0),
        R::XMM3 => (Xmm3, 16, 0),
        R::XMM4 => (Xmm4, 16, 0),
        R::XMM5 => (Xmm5, 16, 0),
        R::XMM6 => (Xmm6, 16, 0),
        R::XMM7 => (Xmm7, 16, 0),
        R::XMM8 => (Xmm8, 16, 0),
        R::XMM9 => (Xmm9, 16, 0),
        R::XMM10 => (Xmm10, 16, 0),
        R::XMM11 => (Xmm11, 16, 0),
        R::XMM12 => (Xmm12, 16, 0),
        R::XMM13 => (Xmm13, 16, 0),
        R::XMM14 => (Xmm14, 16, 0),
        R::XMM15 => (Xmm15, 16, 0),
        _ => return None,
    };
    Some(RegView::new(base, width, offset))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn views_name_correctly() {
        assert_eq!(RegView::full(Reg::Rax).name(), "rax");
        assert_eq!(RegView::new(Reg::Rax, 4, 0).name(), "eax");
        assert_eq!(RegView::new(Reg::Rax, 2, 0).name(), "ax");
        assert_eq!(RegView::new(Reg::Rax, 1, 0).name(), "al");
        assert_eq!(RegView::new(Reg::Rax, 1, 8).name(), "ah");
        assert_eq!(RegView::new(Reg::R8, 4, 0).name(), "r8d");
        assert_eq!(RegView::new(Reg::Rsp, 1, 0).name(), "spl");
    }

    #[test]
    fn maps_iced_registers() {
        assert_eq!(
            from_iced(iced_x86::Register::EAX),
            Some(RegView::new(Reg::Rax, 4, 0))
        );
        assert_eq!(
            from_iced(iced_x86::Register::R15W),
            Some(RegView::new(Reg::R15, 2, 0))
        );
        assert_eq!(
            from_iced(iced_x86::Register::XMM0),
            Some(RegView::new(Reg::Xmm0, 16, 0))
        );
        assert_eq!(from_iced(iced_x86::Register::MM0), None);
    }
}
