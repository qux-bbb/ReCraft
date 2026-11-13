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
    /// AArch64 general-purpose registers `x0`-`x30` (`w0`-`w30` as 32-bit).
    A64X0,
    A64X1,
    A64X2,
    A64X3,
    A64X4,
    A64X5,
    A64X6,
    A64X7,
    A64X8,
    A64X9,
    A64X10,
    A64X11,
    A64X12,
    A64X13,
    A64X14,
    A64X15,
    A64X16,
    A64X17,
    A64X18,
    A64X19,
    A64X20,
    A64X21,
    A64X22,
    A64X23,
    A64X24,
    A64X25,
    A64X26,
    A64X27,
    A64X28,
    A64X29,
    A64X30,
    /// AArch64 stack pointer (`sp`/`wsp`).
    A64Sp,
    /// AArch64 zero register (`xzr`/`wzr`).
    A64Zr,
}

impl Reg {
    const NAMES: [&'static str; 32] = [
        "rax", "rcx", "rdx", "rbx", "rsp", "rbp", "rsi", "rdi", "r8", "r9", "r10", "r11", "r12",
        "r13", "r14", "r15", "xmm0", "xmm1", "xmm2", "xmm3", "xmm4", "xmm5", "xmm6", "xmm7",
        "xmm8", "xmm9", "xmm10", "xmm11", "xmm12", "xmm13", "xmm14", "xmm15",
    ];

    const A64_X: [&'static str; 31] = [
        "x0", "x1", "x2", "x3", "x4", "x5", "x6", "x7", "x8", "x9", "x10", "x11", "x12", "x13",
        "x14", "x15", "x16", "x17", "x18", "x19", "x20", "x21", "x22", "x23", "x24", "x25", "x26",
        "x27", "x28", "x29", "x30",
    ];

    const A64_W: [&'static str; 31] = [
        "w0", "w1", "w2", "w3", "w4", "w5", "w6", "w7", "w8", "w9", "w10", "w11", "w12", "w13",
        "w14", "w15", "w16", "w17", "w18", "w19", "w20", "w21", "w22", "w23", "w24", "w25", "w26",
        "w27", "w28", "w29", "w30",
    ];

    /// Name-table index for the AArch64 xN/wN variants.
    const fn a64_index(self) -> Option<usize> {
        let index = self as usize;
        if index >= 32 && index < 63 {
            Some(index - 32)
        } else {
            None
        }
    }

    /// 64-bit (or full XMM) name of this register.
    pub fn name64(self) -> &'static str {
        match self {
            Reg::A64Sp => "sp",
            Reg::A64Zr => "xzr",
            other => match other.a64_index() {
                Some(index) => Self::A64_X[index],
                None => Self::NAMES[other as usize],
            },
        }
    }

    /// True for the AArch64 namespace (x86 code never sees these).
    pub const fn is_aarch64(self) -> bool {
        (self as usize) >= 32
    }

    /// AArch64 register view: `xN`/`wN`, or `sp`/`wsp` and `xzr`/`wzr` for 31.
    ///
    /// `stack_pointer` selects which meaning 31 has: AArch64 encodes `sp` and `xzr`
    /// with the same number, and whether it is one or the other depends on the instruction.
    pub fn aarch64(number: u16, width: u8, stack_pointer: bool) -> RegView {
        let base = if number >= 31 {
            if stack_pointer {
                Reg::A64Sp
            } else {
                Reg::A64Zr
            }
        } else {
            Self::a64_gpr(number)
        };
        RegView::new(base, width, 0)
    }

    fn a64_gpr(number: u16) -> Reg {
        match number {
            1 => Reg::A64X1,
            2 => Reg::A64X2,
            3 => Reg::A64X3,
            4 => Reg::A64X4,
            5 => Reg::A64X5,
            6 => Reg::A64X6,
            7 => Reg::A64X7,
            8 => Reg::A64X8,
            9 => Reg::A64X9,
            10 => Reg::A64X10,
            11 => Reg::A64X11,
            12 => Reg::A64X12,
            13 => Reg::A64X13,
            14 => Reg::A64X14,
            15 => Reg::A64X15,
            16 => Reg::A64X16,
            17 => Reg::A64X17,
            18 => Reg::A64X18,
            19 => Reg::A64X19,
            20 => Reg::A64X20,
            21 => Reg::A64X21,
            22 => Reg::A64X22,
            23 => Reg::A64X23,
            24 => Reg::A64X24,
            25 => Reg::A64X25,
            26 => Reg::A64X26,
            27 => Reg::A64X27,
            28 => Reg::A64X28,
            29 => Reg::A64X29,
            30 => Reg::A64X30,
            _ => Reg::A64X0,
        }
    }

    /// Name for a width in bytes (1, 2, 4, 8), ignoring high-byte views.
    pub fn name_width(self, width: u8) -> &'static str {
        match self {
            Reg::A64Sp => {
                if width == 4 {
                    "wsp"
                } else {
                    "sp"
                }
            }
            Reg::A64Zr => {
                if width == 4 {
                    "wzr"
                } else {
                    "xzr"
                }
            }
            other if other.is_aarch64() => {
                let index = (other as usize) - 32;
                if width == 4 {
                    Self::A64_W[index]
                } else {
                    Self::A64_X[index]
                }
            }
            _ => self.x86_name_width(width),
        }
    }

    fn x86_name_width(self, width: u8) -> &'static str {
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
