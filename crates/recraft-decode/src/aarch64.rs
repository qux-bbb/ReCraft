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
use yaxpeax_arm::armv8::a64::{InstDecoder, Operand, SizeCode};

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
        Operand::SIMDRegister(..)
        | Operand::SIMDRegisterElements(..)
        | Operand::SIMDRegisterElementsLane(..)
        | Operand::SIMDRegisterGroup(..)
        | Operand::SIMDRegisterElementsMultipleLane(..)
        | Operand::SIMDRegisterGroupLane(..) => DecodedOperand::Register {
            name: "v".to_owned(),
            size: 16,
        },
        Operand::ConditionCode(code) => DecodedOperand::Immediate(i64::from(*code)),
        Operand::PCOffset(offset) => {
            DecodedOperand::NearBranch(Address::new(addr.get().wrapping_add(*offset as u64)))
        }
        Operand::Immediate(value) => DecodedOperand::Immediate(i64::from(*value)),
        Operand::Imm64(value) => DecodedOperand::Immediate(*value as i64),
        Operand::Imm16(value) => DecodedOperand::Immediate(i64::from(*value)),
        Operand::ImmediateDouble(value) => DecodedOperand::Immediate(*value as i64),
        Operand::ImmShift(base, shift) => {
            DecodedOperand::Immediate((i64::from(*base)) << shift.min(&31))
        }
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
        format!("{prefix}sp")
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
    fn a_short_slice_is_not_decoded() {
        assert!(decode_aarch64(&[0x1f, 0x20], Address::new(0x1000)).is_none());
    }
}
