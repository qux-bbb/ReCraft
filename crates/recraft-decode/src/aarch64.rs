// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! AArch64 decoding into the neutral instruction model (ADR-015, W5 spike).
//!
//! The decoder is [`yaxpeax-arm`](https://crates.io/crates/yaxpeax-arm) (0BSD, pure Rust);
//! the x86 path keeps iced and is untouched. A64 instructions are fixed-width (4 bytes), so
//! a decode that succeeds always consumes four bytes.
//!
//! Scope of this spike: mnemonic, control-flow kind, branch target and the register /
//! immediate operands needed by the CFG and the disassembly view. SIMD lane operands and
//! complex addressing modes are only partially modelled; an operand we cannot describe ends
//! the operand list rather than inventing a value (the mnemonic and branch target survive).

use recraft_common::Address;
use yaxpeax_arch::{Decoder, U8Reader};
use yaxpeax_arm::armv8::a64::{InstDecoder, Operand, SIMDSizeCode, SizeCode};

use crate::FlowKind;
use crate::neutral::{DecodedInsn, DecodedOperand};

/// Length of one A64 instruction in bytes.
pub const AARCH64_INSN_LEN: u8 = 4;

/// Decode one AArch64 instruction at `addr`.
pub fn decode_aarch64(bytes: &[u8], addr: Address) -> Option<DecodedInsn> {
    if bytes.len() < usize::from(AARCH64_INSN_LEN) {
        return None;
    }
    let mut reader = U8Reader::new(&bytes[..usize::from(AARCH64_INSN_LEN)]);
    let instruction = InstDecoder::default().decode(&mut reader).ok()?;
    let text = format!("{instruction}");
    let mut parts = text.split_whitespace();
    let mnemonic = parts.next().unwrap_or("?").to_lowercase();
    let flow = aarch64_flow(&mnemonic);
    let operands: Vec<DecodedOperand> = instruction
        .operands
        .iter()
        .take_while(|operand| !matches!(operand, Operand::Nothing))
        .map(|operand| aarch64_operand(operand, addr))
        .collect();
    let mut operands = operands;
    // `cmp`/`cmn`/`tst` write to the discarded `xzr`; yaxpeax keeps that destination in the
    // operand list while its own printer hides it.
    if matches!(mnemonic.as_str(), "cmp" | "cmn" | "tst")
        && matches!(
            operands.first(),
            Some(DecodedOperand::Register { name, .. }) if name.ends_with("zr")
        )
    {
        operands.remove(0);
    }
    // `movz`/`movn`/`movk` encode a 16-bit lane plus a shift (`hw * 16`). yaxpeax folds the
    // shift into the immediate in some encodings, which loses the lane and made the lifter
    // shift `movk` by a constant 16; rebuild both operands from the encoding word.
    if matches!(mnemonic.as_str(), "movz" | "movn" | "movk") && bytes.len() >= 4 {
        let word = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let imm16 = i64::from((word >> 5) & 0xffff);
        let shift = i64::from((word >> 21) & 0x3) * 16;
        if let Some(slot) = operands.get_mut(1)
            && matches!(slot, DecodedOperand::Immediate(_))
        {
            *slot = DecodedOperand::Immediate(imm16);
        }
        if shift != 0 {
            let shift_operand = DecodedOperand::Immediate(shift);
            match operands.len() {
                0 | 1 => {}
                2 => operands.push(shift_operand),
                _ => operands[2] = shift_operand,
            }
        }
    }

    let branch_target = if matches!(
        flow,
        FlowKind::UnconditionalBranch | FlowKind::ConditionalBranch | FlowKind::Call
    ) {
        operands.iter().find_map(|operand| match operand {
            DecodedOperand::NearBranch(target) => Some(*target),
            _ => None,
        })
    } else {
        None
    };
    let (reads, writes) = aarch64_registers(&instruction.operands);
    Some(DecodedInsn {
        addr,
        len: AARCH64_INSN_LEN,
        mnemonic,
        flow,
        branch_target,
        operands,
        reads,
        writes,
    })
}

/// Control-flow kind of an A64 mnemonic.
fn aarch64_flow(mnemonic: &str) -> FlowKind {
    match mnemonic {
        "b" | "b.al" => FlowKind::UnconditionalBranch,
        "bl" => FlowKind::Call,
        "br" => FlowKind::IndirectBranch,
        "blr" => FlowKind::IndirectCall,
        "ret" | "retaa" | "retab" => FlowKind::Return,
        "cbz" | "cbnz" | "tbz" | "tbnz" => FlowKind::ConditionalBranch,
        _ if mnemonic.starts_with("b.") => FlowKind::ConditionalBranch,
        _ => FlowKind::Next,
    }
}

/// One A64 operand in the neutral model.
fn aarch64_operand(operand: &Operand, addr: Address) -> DecodedOperand {
    match operand {
        Operand::Register(size, number) => register_operand(*size, *number, false),
        Operand::RegisterOrSP(size, number) => register_operand(*size, *number, true),
        Operand::RegisterPair(size, number) => register_operand(*size, *number, true),
        Operand::SIMDRegister(size, number) => simd_operand(*size, *number, None, None, None),
        Operand::SIMDRegisterElements(size, number, element) => {
            simd_operand(*size, *number, Some(*element), None, None)
        }
        Operand::SIMDRegisterElementsLane(size, number, element, lane) => {
            simd_operand(*size, *number, Some(*element), Some(*lane), None)
        }
        Operand::SIMDRegisterElementsMultipleLane(size, number, element, lane, count) => {
            simd_operand(*size, *number, Some(*element), Some(*lane), Some(*count))
        }
        Operand::SIMDRegisterGroup(size, number, element, count) => {
            simd_group_operand(*size, *number, *element, *count, None)
        }
        Operand::SIMDRegisterGroupLane(number, element, count, lane) => {
            simd_group_operand(SIMDSizeCode::Q, *number, *element, *count, Some(*lane))
        }
        Operand::ConditionCode(code) => DecodedOperand::Immediate(i64::from(*code)),
        Operand::PCOffset(offset) => {
            DecodedOperand::NearBranch(Address::new(addr.get().wrapping_add(*offset as u64)))
        }
        Operand::Immediate(value) => DecodedOperand::Immediate(i64::from(*value)),
        Operand::Imm64(value) => DecodedOperand::Immediate(*value as i64),
        Operand::Imm16(value) => DecodedOperand::Immediate(i64::from(*value)),
        Operand::ImmediateDouble(value) => DecodedOperand::Immediate(*value as i64),
        Operand::ImmShift(base, shift) => DecodedOperand::Immediate(if *shift >= 64 {
            0
        } else {
            (i64::from(*base)) << shift
        }),
        Operand::RegShift(_, shift, size, number) => {
            let mut register = register_operand(*size, *number, false);
            if *shift != 0
                && let DecodedOperand::Register { name, .. } = &mut register
            {
                name.push_str(&format!(", lsl #{shift}"));
            }
            register
        }
        // Memory operands: `[xN]`, `[xN, #off]`, `[xN], #off`, `[xN, xM]`.
        Operand::RegPreIndex(base, offset, _) => memory_operand(*base, None, 0, i64::from(*offset)),
        Operand::RegPostIndex(base, offset) => memory_operand(*base, None, 0, i64::from(*offset)),
        Operand::RegPostIndexReg(base, index) => memory_operand(*base, Some(*index), 0, 0),
        Operand::RegRegOffset(base, index, _, _, shift) => {
            memory_operand(*base, Some(*index), *shift, 0)
        }
        _ => DecodedOperand::Immediate(0),
    }
}

/// A64 memory operand: `base` plus an optional index shifted by `shift`.
fn memory_operand(base: u16, index: Option<u16>, shift: u8, disp: i64) -> DecodedOperand {
    DecodedOperand::Memory {
        base: Some(if base == 31 {
            "xsp".to_owned()
        } else {
            format!("x{base}")
        }),
        index: index.map(|index| format!("x{index}")),
        scale: 1u8 << shift.min(4),
        disp,
        size: 8,
    }
}

fn register_operand(size: SizeCode, number: u16, stack_pointer: bool) -> DecodedOperand {
    let prefix = if matches!(size, SizeCode::W) {
        'w'
    } else {
        'x'
    };
    let name = if number == 31 && !stack_pointer {
        format!("{prefix}zr")
    } else if number == 31 {
        // The stack pointer is spelled `sp`/`wsp`, never `xsp`.
        if matches!(size, SizeCode::W) {
            "wsp".to_owned()
        } else {
            "sp".to_owned()
        }
    } else {
        format!("{prefix}{number}")
    };
    DecodedOperand::Register {
        name,
        size: match size {
            SizeCode::W => 4,
            SizeCode::X => 8,
        },
    }
}

/// Width of one SIMD element in bytes and the letter used in the textual form.
fn simd_element(element: SIMDSizeCode) -> (u16, char) {
    match element {
        SIMDSizeCode::B => (1, 'b'),
        SIMDSizeCode::H => (2, 'h'),
        SIMDSizeCode::S => (4, 's'),
        SIMDSizeCode::D => (8, 'd'),
        SIMDSizeCode::Q => (16, 'q'),
    }
}

/// A SIMD register operand: `v3`, `v3.4s`, `v3.s[1]` instead of a bare `v`.
fn simd_operand(
    size: SIMDSizeCode,
    number: u16,
    element: Option<SIMDSizeCode>,
    lane: Option<u8>,
    count: Option<u8>,
) -> DecodedOperand {
    let width: u16 = if matches!(size, SIMDSizeCode::D) {
        8
    } else {
        16
    };
    let mut name = format!("v{number}");
    if let Some(element) = element {
        let (bytes, letter) = simd_element(element);
        let lanes = count
            .map(u16::from)
            .unwrap_or_else(|| (width / bytes).max(1));
        name.push_str(&format!(".{lanes}{letter}"));
        if let Some(lane) = lane {
            name.push_str(&format!("[{lane}]"));
        }
    }
    DecodedOperand::Register { name, size: 16 }
}

/// A group of consecutive SIMD registers: `{v2.2d, v3.2d, v4.2d}[1]`.
fn simd_group_operand(
    size: SIMDSizeCode,
    number: u16,
    element: SIMDSizeCode,
    count: u8,
    lane: Option<u8>,
) -> DecodedOperand {
    let mut names = Vec::new();
    for index in 0..count.max(1) {
        let one = simd_operand(size, number + u16::from(index), Some(element), lane, None);
        if let DecodedOperand::Register { name, .. } = one {
            names.push(name);
        }
    }
    DecodedOperand::Register {
        name: format!("{{{}}}", names.join(", ")),
        size: 16,
    }
}

/// Registers read and written, from the explicit operands only.
///
/// A64 has no fixed destination register, so the first operand is a destination for the
/// data-processing forms and the branch forms write `x30`; implicit effects (flags, `ret`
/// reading `x30`) are not modelled yet and stay with the ARM64 lifter work.
fn aarch64_registers(operands: &[Operand; 4]) -> (Vec<String>, Vec<String>) {
    let mut reads = Vec::new();
    let mut writes = Vec::new();
    for (index, operand) in operands.iter().enumerate() {
        let (Operand::Register(size, number)
        | Operand::RegisterOrSP(size, number)
        | Operand::RegisterPair(size, number)) = operand
        else {
            continue;
        };
        let DecodedOperand::Register { name, .. } = register_operand(*size, *number, true) else {
            continue;
        };
        if index == 0 {
            writes.push(name);
        } else {
            reads.push(name);
        }
    }
    (reads, writes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(word: u32) -> DecodedInsn {
        decode_aarch64(&word.to_le_bytes(), Address::new(0x1000)).expect("decodes")
    }

    #[test]
    fn a_return_is_recognised() {
        // ret
        let insn = decode(0xd65f_03c0);
        assert_eq!(insn.mnemonic, "ret");
        assert_eq!(insn.flow, FlowKind::Return);
        assert_eq!(insn.len, 4);
    }

    #[test]
    fn an_add_carries_its_register_and_immediate_operands() {
        // add x0, x0, #1
        let insn = decode(0x9100_0400);
        assert_eq!(insn.mnemonic, "add");
        assert_eq!(
            insn.operands,
            vec![
                DecodedOperand::Register {
                    name: "x0".to_owned(),
                    size: 8
                },
                DecodedOperand::Register {
                    name: "x0".to_owned(),
                    size: 8
                },
                DecodedOperand::Immediate(1),
            ]
        );
        assert!(insn.writes.iter().any(|name| name == "x0"));
        assert!(insn.reads.iter().any(|name| name == "x0"));
    }

    #[test]
    fn a_branch_exposes_its_target() {
        // b +0x10
        let insn = decode(0x1400_0004);
        assert_eq!(insn.mnemonic, "b");
        assert_eq!(insn.flow, FlowKind::UnconditionalBranch);
        assert_eq!(insn.branch_target, Some(Address::new(0x1010)));
    }

    #[test]
    fn a_bl_is_a_call() {
        // bl +0x20
        let insn = decode(0x9400_0008);
        assert_eq!(insn.mnemonic, "bl");
        assert_eq!(insn.flow, FlowKind::Call);
        assert_eq!(insn.branch_target, Some(Address::new(0x1020)));
    }

    #[test]
    fn a_conditional_branch_is_not_unconditional() {
        // b.eq +0x8 (imm19 = 2)
        let insn = decode(0x5400_0040);
        assert_eq!(insn.mnemonic, "b.eq");
        assert_eq!(insn.flow, FlowKind::ConditionalBranch);
        assert_eq!(insn.branch_target, Some(Address::new(0x1008)));
    }

    #[test]
    fn an_indirect_branch_has_no_static_target() {
        // br x1
        let insn = decode(0xd61f_0020);
        assert_eq!(insn.mnemonic, "br");
        assert_eq!(insn.flow, FlowKind::IndirectBranch);
        assert_eq!(insn.branch_target, None);
    }

    #[test]
    fn a_movk_keeps_its_lane_and_shift() {
        // movk x6, #0x38e3, lsl #32 (hw = 2): the immediate and the shift are separate
        // operands, otherwise the lifter shifted every lane by 16.
        let insn = decode(0xf2c7_1c66);
        assert_eq!(insn.mnemonic, "movk");
        assert_eq!(
            insn.operand_text(),
            "x6, #0x38e3, #0x20",
            "{}",
            insn.operand_text()
        );
    }

    #[test]
    fn the_stack_pointer_is_spelled_sp() {
        // sub x29, sp, #8 (RegisterOrSP 31 in an X context)
        let insn = decode(0xd10023fd);
        assert!(
            insn.operand_text().contains("sp"),
            "{}",
            insn.operand_text()
        );
        assert!(
            !insn.operand_text().contains("xsp"),
            "{}",
            insn.operand_text()
        );
    }

    #[test]
    fn a_short_slice_is_not_decoded() {
        assert!(decode_aarch64(&[0x1f, 0x20], Address::new(0x1000)).is_none());
    }
}
