// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! AArch64 decoding into the neutral instruction model (ADR-015).
//!
//! The backend is [capstone](https://www.capstone-engine.org/) behind the `capstone` crate
//! (BSD-3-Clause, C, built by `capstone-sys`); the x86 path keeps iced and is untouched.
//! A64 instructions are fixed-width (4 bytes), so a decode that succeeds always consumes four
//! bytes.
//!
//! Capstone is used for the AArch64 side because its operand detail and printed forms match
//! `objdump`/IDA, and because the alternative (yaxpeax-arm, 51 GitHub stars) is a one-person
//! project: the switch keeps the risk inside this adapter, since everything downstream only
//! sees [`DecodedInsn`].
//!
//! Scope: mnemonic, control-flow kind, branch target and the register / immediate / memory
//! operands the CFG, the disassembly view and the A64 lifter need. Lane-level SIMD detail is
//! rendered as text but not modelled semantically.

use std::cell::RefCell;

use capstone::arch::arm64::{Arm64OperandType, Arm64Shift};
use capstone::prelude::*;
use recraft_common::Address;

use crate::FlowKind;
use crate::neutral::{DecodedInsn, DecodedOperand};

/// Length of one A64 instruction in bytes.
pub const AARCH64_INSN_LEN: u8 = 4;

thread_local! {
    /// One capstone handle per thread: creating it per instruction costs milliseconds, and a
    /// `Capstone` is not `Sync`.
    static A64: RefCell<Option<Capstone>> = const { RefCell::new(None) };
}

/// Build (once per thread) the AArch64 capstone handle.
fn with_capstone<T>(f: impl FnOnce(&Capstone) -> Option<T>) -> Option<T> {
    A64.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.is_none() {
            *slot = Capstone::new()
                .arm64()
                .mode(capstone::arch::arm64::ArchMode::Arm)
                .detail(true)
                .build()
                .ok();
        }
        let capstone = slot.as_ref()?;
        f(capstone)
    })
}

/// Decode one AArch64 instruction at `addr`.
pub fn decode_aarch64(bytes: &[u8], addr: Address) -> Option<DecodedInsn> {
    if bytes.len() < usize::from(AARCH64_INSN_LEN) {
        return None;
    }
    let word_bytes = &bytes[..usize::from(AARCH64_INSN_LEN)];
    with_capstone(|capstone| decode_with(capstone, word_bytes, addr)).or_else(|| {
        // capstone rejects `udf #imm` (it is not a valid instruction on every ARMv8 profile),
        // but the analyzer and the lifter use `udf` as an intrinsic and as a walk terminator.
        let word = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        (word & 0xFFFF_0000 == 0).then(|| DecodedInsn {
            addr,
            len: AARCH64_INSN_LEN,
            mnemonic: "udf".to_owned(),
            flow: FlowKind::Next,
            branch_target: None,
            operands: vec![DecodedOperand::Immediate(i64::from(word & 0xffff))],
            reads: Vec::new(),
            writes: Vec::new(),
        })
    })
}

fn decode_with(capstone: &Capstone, bytes: &[u8], addr: Address) -> Option<DecodedInsn> {
    let instructions = capstone.disasm_all(bytes, addr.get()).ok()?;
    let instruction = instructions.iter().next()?;
    let mnemonic = instruction.mnemonic()?.to_lowercase();
    let flow = aarch64_flow(&mnemonic);
    let detail = capstone.insn_detail(instruction).ok()?;
    let architecture = detail.arch_detail();
    let arm64 = architecture.arm64()?;

    let mut operands = Vec::new();
    for operand in arm64.operands() {
        let shift = match operand.shift {
            Arm64Shift::Lsl(amount) => Some(amount),
            _ => None,
        };
        match operand.op_type {
            Arm64OperandType::Reg(reg) => {
                let name = canonical_register(capstone.reg_name(reg).unwrap_or_default());
                if name.is_empty() {
                    continue;
                }
                operands.push(DecodedOperand::Register {
                    size: register_size(&name),
                    name,
                });
            }
            Arm64OperandType::Imm(value) => operands.push(DecodedOperand::Immediate(value)),
            Arm64OperandType::Mem(memory) => {
                let base = memory_reg(capstone, memory.base());
                let index = memory_reg(capstone, memory.index());
                operands.push(DecodedOperand::Memory {
                    base,
                    index,
                    scale: 1,
                    disp: i64::from(memory.disp()),
                    size: 8,
                });
            }
            _ => {}
        }
        // `movk`/`movz`/`movn` carry the lane shift on the immediate operand; hand it to the
        // lifter as its own operand so it can shift by 0/16/32/48 instead of a constant.
        if matches!(mnemonic.as_str(), "movk" | "movz" | "movn")
            && let Some(amount) = shift
            && let Some(DecodedOperand::Immediate(_)) = operands.last()
        {
            operands.push(DecodedOperand::Immediate(i64::from(amount)));
        }
    }

    // Branch targets arrive as absolute immediates; promote the last one so the CFG and the
    // call graph see the same shape the x86 backend produces.
    if matches!(
        flow,
        FlowKind::UnconditionalBranch | FlowKind::ConditionalBranch | FlowKind::Call
    ) && let Some(last) = operands.last_mut()
        && let DecodedOperand::Immediate(target) = last
    {
        let target = *target as u64;
        *last = DecodedOperand::NearBranch(Address::new(target));
    }
    let branch_target = operands.iter().find_map(|operand| match operand {
        DecodedOperand::NearBranch(target) => Some(*target),
        _ => None,
    });

    let mut reads = register_names(capstone, detail.regs_read());
    let mut writes = register_names(capstone, detail.regs_write());
    // capstone reports *implicit* registers here; the explicit destination (the first register
    // operand of a data-processing instruction) has to be added, or the xrefs and the "what
    // does this function touch" views lose every written register.
    let writes_destination = !(mnemonic.starts_with("st")
        || mnemonic.starts_with("cmp")
        || mnemonic.starts_with("cmn")
        || mnemonic.starts_with("tst")
        || matches!(
            flow,
            FlowKind::UnconditionalBranch
                | FlowKind::ConditionalBranch
                | FlowKind::Call
                | FlowKind::IndirectBranch
                | FlowKind::IndirectCall
                | FlowKind::Return
        ));
    for (index, operand) in operands.iter().enumerate() {
        let DecodedOperand::Register { name, .. } = operand else {
            continue;
        };
        let destination = index == 0 && writes_destination;
        let target = if destination { &mut writes } else { &mut reads };
        if !target.contains(name) {
            target.push(name.clone());
        }
    }
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

/// Capstone prints the architectural aliases (`lr`, `fp`, `ip0`, `ip1`); the lifter and the
/// rest of ReCraft use the numbered registers.
fn canonical_register(name: String) -> String {
    match name.as_str() {
        "lr" => "x30".to_owned(),
        "fp" => "x29".to_owned(),
        "ip0" => "x16".to_owned(),
        "ip1" => "x17".to_owned(),
        _ => name,
    }
}

/// Register width in bytes, inferred from the capstone register name.
fn register_size(name: &str) -> u8 {
    match name {
        "sp" => 8,
        "wsp" => 4,
        "xzr" => 8,
        "wzr" => 4,
        _ => match name.chars().next() {
            Some('x') => 8,
            Some('w') => 4,
            Some('v') | Some('q') => 16,
            Some('d') => 8,
            Some('s') => 4,
            Some('h') => 2,
            Some('b') => 1,
            _ => 8,
        },
    }
}

/// A base/index register of a memory operand, if capstone reports one.
fn memory_reg(capstone: &Capstone, reg: RegId) -> Option<String> {
    if reg.0 == 0 {
        return None;
    }
    capstone.reg_name(reg).filter(|name| !name.is_empty())
}

/// Map capstone register ids to names, dropping invalid entries and duplicates.
fn register_names(capstone: &Capstone, registers: &[RegId]) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for reg in registers {
        let Some(name) = capstone.reg_name(*reg) else {
            continue;
        };
        if name.is_empty() || names.contains(&name) {
            continue;
        }
        names.push(name);
    }
    names
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
        // operands, otherwise the lifter would shift every lane by the same amount.
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
        // sub x29, sp, #8
        let insn = decode(0xd10023fd);
        let text = insn.operand_text();
        assert!(text.contains("sp"), "{text}");
        assert!(!text.contains("xsp"), "{text}");
    }

    #[test]
    fn a_load_exposes_its_memory_operand() {
        // ldr x0, [x1, #0x18]
        let insn = decode(0xf9400c20);
        assert_eq!(insn.mnemonic, "ldr");
        assert!(
            insn.operands.iter().any(|operand| matches!(
                operand,
                DecodedOperand::Memory {
                    base: Some(base),
                    disp: 0x18,
                    ..
                } if base == "x1"
            )),
            "{:?}",
            insn.operands
        );
    }

    #[test]
    fn a_short_slice_is_not_decoded() {
        assert!(decode_aarch64(&[0x1f, 0x20], Address::new(0x1000)).is_none());
    }
}
