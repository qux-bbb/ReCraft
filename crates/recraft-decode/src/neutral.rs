// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Backend-neutral instruction description (ADR-015).
//!
//! The x86 lifter keeps using iced's typed API: that is the x86 contract, and rewriting it
//! would gain nothing. What the analysis side (CFG, xrefs, call graph) and the future ARM64
//! backend need is one *shape* that does not depend on a disassembler crate: address,
//! length, operands, branch target and the registers an instruction reads and writes.
//!
//! `DecodedInsn` is that shape. Registers are names (`"rcx"`, `"x0"`), so an ARM64 adapter
//! fills the same struct without sharing a register enum with iced; the x86 lifter keeps
//! its typed `Reg`/`RegView` and is not affected.

use recraft_common::Address;

use crate::{FlowKind, Insn};

/// One operand of a decoded instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodedOperand {
    /// A register operand, by name and width in bytes.
    Register { name: String, size: u8 },
    /// An immediate value (already sign-extended by the decoder).
    Immediate(i64),
    /// A memory operand: `base + index * scale + disp`, `size` bytes wide.
    Memory {
        base: Option<String>,
        index: Option<String>,
        scale: u8,
        disp: i64,
        size: u8,
    },
    /// A direct branch target (relative call/jump).
    NearBranch(Address),
}

/// A decoded instruction as any backend can describe it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedInsn {
    pub addr: Address,
    /// Instruction length in bytes.
    pub len: u8,
    /// Backend mnemonic, lower-case (`"mov"`, `"add"`, `"b.eq"`).
    pub mnemonic: String,
    pub flow: FlowKind,
    /// Direct branch target, when the instruction transfers control to a constant address.
    pub branch_target: Option<Address>,
    pub operands: Vec<DecodedOperand>,
    /// Registers the instruction reads (explicit and implicit), de-duplicated.
    pub reads: Vec<String>,
    /// Registers the instruction writes (explicit and implicit), de-duplicated.
    pub writes: Vec<String>,
}

impl Insn {
    /// Describe this instruction without exposing the backend's types.
    pub fn neutral(&self) -> DecodedInsn {
        let instruction = self.instruction();
        let operands = (0..self.op_count())
            .filter_map(|index| self.neutral_operand(index))
            .collect();
        let (reads, writes) = neutral_registers(instruction);
        DecodedInsn {
            addr: self.addr(),
            len: self.len(),
            mnemonic: format!("{:?}", self.mnemonic()).to_lowercase(),
            flow: self.flow(),
            branch_target: self.branch_target(),
            operands,
            reads,
            writes,
        }
    }

    fn neutral_operand(&self, index: u32) -> Option<DecodedOperand> {
        use iced_x86::OpKind;
        let instruction = self.instruction();
        match instruction.op_kind(index) {
            OpKind::Register => {
                let register = instruction.op_register(index);
                Some(DecodedOperand::Register {
                    name: register_name(register),
                    size: register_size(instruction, index),
                })
            }
            OpKind::Memory => Some(DecodedOperand::Memory {
                base: Some(instruction.memory_base())
                    .filter(|register| !is_none_register(*register))
                    .map(register_name),
                index: Some(instruction.memory_index())
                    .filter(|register| !is_none_register(*register))
                    .map(register_name),
                scale: instruction.memory_index_scale() as u8,
                disp: instruction.memory_displacement64() as i64,
                size: instruction.memory_size().size() as u8,
            }),
            kind if is_immediate(kind) => Some(DecodedOperand::Immediate(immediate_value(
                instruction,
                index,
            ))),
            kind if is_near_branch(kind) => Some(DecodedOperand::NearBranch(Address::new(
                instruction.near_branch_target(),
            ))),
            _ => None,
        }
    }
}

pub(crate) fn register_name(register: iced_x86::Register) -> String {
    format!("{register:?}").to_lowercase()
}

fn is_none_register(register: iced_x86::Register) -> bool {
    register == iced_x86::Register::None
}

fn register_size(instruction: &iced_x86::Instruction, index: u32) -> u8 {
    instruction.op_register(index).size() as u8
}

fn is_immediate(kind: iced_x86::OpKind) -> bool {
    use iced_x86::OpKind;
    matches!(
        kind,
        OpKind::Immediate8
            | OpKind::Immediate8to16
            | OpKind::Immediate8to32
            | OpKind::Immediate8to64
            | OpKind::Immediate16
            | OpKind::Immediate32
            | OpKind::Immediate32to64
            | OpKind::Immediate64
    )
}

fn is_near_branch(kind: iced_x86::OpKind) -> bool {
    use iced_x86::OpKind;
    matches!(
        kind,
        OpKind::NearBranch16 | OpKind::NearBranch32 | OpKind::NearBranch64
    )
}

fn immediate_value(instruction: &iced_x86::Instruction, index: u32) -> i64 {
    use iced_x86::OpKind;
    match instruction.op_kind(index) {
        OpKind::Immediate8 => i64::from(instruction.immediate8()),
        OpKind::Immediate8to16 => i64::from(instruction.immediate8to16()),
        OpKind::Immediate8to32 => i64::from(instruction.immediate8to32()),
        OpKind::Immediate8to64 => instruction.immediate8to64(),
        OpKind::Immediate16 => i64::from(instruction.immediate16()),
        OpKind::Immediate32 => i64::from(instruction.immediate32()),
        OpKind::Immediate32to64 => instruction.immediate32to64(),
        _ => instruction.immediate64() as i64,
    }
}

/// Registers read and written by the instruction, implicit ones included.
fn neutral_registers(instruction: &iced_x86::Instruction) -> (Vec<String>, Vec<String>) {
    use iced_x86::OpAccess;
    use std::collections::BTreeSet;
    let mut factory = iced_x86::InstructionInfoFactory::new();
    let info = factory.info(instruction);
    let mut reads = BTreeSet::new();
    let mut writes = BTreeSet::new();
    for used in info.used_registers() {
        let name = register_name(used.register());
        match used.access() {
            OpAccess::Read | OpAccess::CondRead | OpAccess::ReadWrite | OpAccess::ReadCondWrite => {
                reads.insert(name.clone());
                if matches!(used.access(), OpAccess::ReadWrite | OpAccess::ReadCondWrite) {
                    writes.insert(name);
                }
            }
            OpAccess::Write | OpAccess::CondWrite => {
                writes.insert(name);
            }
            _ => {}
        }
    }
    (reads.into_iter().collect(), writes.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Mode, decode};

    fn neutral(bytes: &[u8]) -> DecodedInsn {
        decode(bytes, Address::new(0x1000), Mode::X64)
            .expect("decodes")
            .neutral()
    }

    #[test]
    fn a_register_write_names_its_source_and_destination() {
        // mov rcx, [rsp+8]
        let insn = neutral(&[0x48, 0x8b, 0x4c, 0x24, 0x08]);
        assert_eq!(insn.addr, Address::new(0x1000));
        assert_eq!(insn.len, 5);
        assert_eq!(insn.mnemonic, "mov");
        assert!(insn.writes.iter().any(|name| name == "rcx"));
        assert!(insn.reads.iter().any(|name| name == "rsp"));
        assert!(matches!(
            insn.operands.as_slice(),
            [
                DecodedOperand::Register { .. },
                DecodedOperand::Memory { disp: 8, .. }
            ]
        ));
    }

    #[test]
    fn a_relative_call_exposes_its_target() {
        // call rel32 +0x10
        let insn = neutral(&[0xe8, 0x00, 0x00, 0x00, 0x00]);
        assert_eq!(insn.flow, FlowKind::Call);
        assert_eq!(insn.branch_target, Some(Address::new(0x1005)));
        assert!(matches!(
            insn.writes.as_slice(),
            [..] if insn.writes.iter().any(|name| name == "rsp")
        ));
    }

    #[test]
    fn an_implicit_register_is_reported() {
        // div rcx reads rax and rdx and writes both.
        let insn = neutral(&[0x48, 0xf7, 0xf1]);
        assert_eq!(insn.mnemonic, "div");
        for name in ["rax", "rdx"] {
            assert!(insn.reads.iter().any(|read| read == name), "{name}");
            assert!(insn.writes.iter().any(|write| write == name), "{name}");
        }
    }

    #[test]
    fn an_indirect_jump_has_no_branch_target() {
        // jmp rax
        let insn = neutral(&[0xff, 0xe0]);
        assert_eq!(insn.flow, FlowKind::IndirectBranch);
        assert_eq!(insn.branch_target, None);
        assert!(insn.reads.iter().any(|name| name == "rax"));
    }

    #[test]
    fn every_operand_of_a_mixed_instruction_is_described() {
        // add qword ptr [rbx+rcx*4+0x10], 1
        let insn = neutral(&[0x48, 0x83, 0x44, 0x8b, 0x10, 0x01]);
        assert_eq!(insn.mnemonic, "add");
        assert_eq!(insn.operands.len(), 2);
        assert!(matches!(
            insn.operands[0],
            DecodedOperand::Memory {
                scale: 4,
                disp: 0x10,
                size: 8,
                ..
            }
        ));
        assert!(matches!(insn.operands[1], DecodedOperand::Immediate(1)));
    }

    #[test]
    fn the_neutral_view_agrees_with_the_decoder_on_operand_count() {
        // A small mixed slice: prologue, arithmetic, branch, call, return.
        let bytes: &[u8] = &[
            0x55, 0x48, 0x89, 0xe5, 0x48, 0x83, 0xec, 0x20, 0x48, 0x8b, 0x45, 0x08, 0x48, 0x01,
            0xc8, 0x74, 0x05, 0xe8, 0x10, 0x00, 0x00, 0x00, 0xc9, 0xc3,
        ];
        let mut offset = 0usize;
        while offset < bytes.len() {
            let Some(insn) = decode(
                &bytes[offset..],
                Address::new(0x1000 + offset as u64),
                Mode::X64,
            ) else {
                break;
            };
            let neutral = insn.neutral();
            assert_eq!(neutral.len, insn.len());
            assert_eq!(neutral.operands.len(), insn.op_count() as usize);
            offset += usize::from(insn.len());
        }
    }
}
