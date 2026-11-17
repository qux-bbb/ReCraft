// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Per-function shape for the list windows: the numbers and flag letters IDA's Functions
//! window shows next to a function's name.
//!
//! IDA takes all of them from the function record built during auto-analysis: `frsize`
//! (locals plus saved registers), `argsize` (stack argument bytes), and the flag bits it
//! prints as `R`/`B`/`T`/`L`. ReCraft derives the same three facts from the code instead of
//! storing them, so they are always in sync with what the listing decodes:
//!
//! - **frame size** - the prologue's `push`es plus its `sub rsp, N` (verified against IDA:
//!   crackme `main` is `mov [rsp+arg_0], rbx; push rdi; sub rsp, 120h` and IDA reports
//!   `frsize` 0x128 = 8 + 0x120);
//! - **frame pointer** - `push rbp; mov rbp, rsp` in the same prologue;
//! - **returns to caller** - the body holds a `ret`, IDA's `R` (cleared for noreturn
//!   functions).

use recraft_common::Address;
use recraft_core::Session;
use recraft_decode::iced_x86::{MemorySize, Mnemonic, OpKind, Register};
use recraft_decode::{Insn, Mode};

/// Instructions the prologue scan walks before giving up on the frame.
const PROLOGUE_LIMIT: usize = 16;
/// Instructions the return scan walks, so a 45 KB function cannot stall the list refresh.
const RETURN_LIMIT: usize = 4096;

/// What the Functions window shows about a function besides its name and range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FunctionShape {
    /// Locals plus saved registers (IDA's `frsize`). `None` when the prologue does not
    /// decode (a data-in-function start, or an architecture without an iced decoder).
    pub frame_size: Option<u32>,
    /// The prologue builds a frame pointer (`push rbp; mov rbp, rsp`), IDA's `B` flag.
    pub bp_frame: bool,
    /// The body holds a `ret` instruction, IDA's `R` flag.
    pub returns: bool,
}

/// Read the shape of the function at `[start, end)`.
pub fn function_shape(session: &Session, start: Address, end: Address) -> FunctionShape {
    let pointer = if session.mode() == Mode::X64 { 8 } else { 4 };
    let (frame_size, bp_frame) =
        scan_frame(|addr| session.insn_at(addr).ok().flatten(), start, pointer);
    let returns = scan_returns(|addr| session.insn_at(addr).ok().flatten(), start, end);
    FunctionShape {
        frame_size,
        bp_frame,
        returns,
    }
}

/// Walk the prologue: saved-register pushes, home-area stores, the frame-pointer setup and
/// the stack allocation. Returns the frame size (pushes + `sub`, IDA's `frsize`) and whether
/// the prologue set up `rbp`.
fn scan_frame(
    mut insn_at: impl FnMut(Address) -> Option<Insn>,
    start: Address,
    pointer: u32,
) -> (Option<u32>, bool) {
    let mut pushed = 0u32;
    let mut bp_frame = false;
    let mut saw_instruction = false;
    let mut addr = start;
    for _ in 0..PROLOGUE_LIMIT {
        let Some(insn) = insn_at(addr) else {
            break;
        };
        saw_instruction = true;
        match insn.mnemonic() {
            Mnemonic::Push => pushed += pointer,
            // `mov rbp, rsp` (the frame pointer), or a home-area store such as
            // `mov [rsp+arg_0], rbx` that MSVC emits before the allocation.
            Mnemonic::Mov => {
                if is_frame_pointer(insn.op_register(0)) && is_stack_pointer(insn.op_register(1)) {
                    bp_frame = true;
                } else if !stores_to_stack(&insn) {
                    break;
                }
            }
            Mnemonic::Sub => {
                if is_stack_pointer(insn.op_register(0)) {
                    let Some(size) = immediate(&insn) else {
                        break;
                    };
                    return (Some(pushed + size), bp_frame);
                }
                break;
            }
            // `lea rsp, [rsp-N]` allocates like `sub rsp, N`; `and rsp, -16` aligns without
            // adding locals, so it neither ends the scan nor grows the frame.
            Mnemonic::Lea => {
                if is_stack_pointer(insn.op_register(0)) {
                    if let Some(size) = lea_stack_adjust(&insn) {
                        return (Some(pushed.saturating_add(size)), bp_frame);
                    }
                }
                break;
            }
            Mnemonic::And => {
                if !is_stack_pointer(insn.op_register(0)) {
                    break;
                }
            }
            // Padding before the prologue (`int3`, `nop`) is walked; the scan stops at the
            // first branch, call or return, which is where the prologue ends.
            Mnemonic::Nop => {}
            Mnemonic::Ret | Mnemonic::Retf | Mnemonic::Jmp | Mnemonic::Call | Mnemonic::Int3 => {
                break;
            }
            _ => break,
        }
        let next = insn.next_addr();
        if next <= addr {
            break;
        }
        addr = next;
    }
    // A framed function without an allocation still owns its saved registers.
    if saw_instruction {
        (Some(pushed), bp_frame)
    } else {
        (None, false)
    }
}

/// Whether the body contains a `ret`. A linear walk may decode embedded data (jump tables)
/// as instructions, but a stray `ret` byte only makes the answer more permissive, which is
/// what IDA's `R` means for the overwhelming majority of functions.
fn scan_returns(
    mut insn_at: impl FnMut(Address) -> Option<Insn>,
    start: Address,
    end: Address,
) -> bool {
    let mut addr = start;
    for _ in 0..RETURN_LIMIT {
        if addr >= end {
            return false;
        }
        let Some(insn) = insn_at(addr) else {
            return false;
        };
        if matches!(insn.mnemonic(), Mnemonic::Ret | Mnemonic::Retf) {
            return true;
        }
        let next = insn.next_addr();
        if next <= addr {
            return false;
        }
        addr = next;
    }
    false
}

fn is_stack_pointer(register: Register) -> bool {
    matches!(register, Register::RSP | Register::ESP | Register::SP)
}

fn is_frame_pointer(register: Register) -> bool {
    matches!(register, Register::RBP | Register::EBP | Register::BP)
}

/// `mov [rsp+N], reg` / `mov [rbp-N], reg`: MSVC's home-area and saved-register spills.
fn stores_to_stack(insn: &Insn) -> bool {
    insn.op_kind(0) == OpKind::Memory
        && is_stack_pointer(insn.memory_base())
        && insn.memory_size() != MemorySize::Unknown
        && insn.op_register(1) != Register::None
}

/// Immediate of `sub rsp, N` / `sub esp, N`.
fn immediate(insn: &Insn) -> Option<u32> {
    match insn.op_kind(1) {
        OpKind::Immediate8 => Some(u32::from(insn.immediate8())),
        OpKind::Immediate8to32 => Some(insn.immediate8to32() as u32),
        OpKind::Immediate8to64 => Some(insn.immediate8to64() as u32),
        OpKind::Immediate32 => Some(insn.immediate32()),
        OpKind::Immediate32to64 => Some(insn.immediate32to64() as u32),
        _ => None,
    }
}

/// `lea rsp, [rsp-N]`: the allocation as a displacement (positive bytes).
fn lea_stack_adjust(insn: &Insn) -> Option<u32> {
    let displacement = insn.memory_displacement64();
    let negative = 0u64.wrapping_sub(displacement);
    (negative <= 0x1000_0000).then_some(negative as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode_all(bytes: &[u8], base: Address, mode: Mode) -> impl FnMut(Address) -> Option<Insn> {
        let bytes = bytes.to_vec();
        move |addr| {
            let offset = addr.get().checked_sub(base.get())? as usize;
            recraft_decode::decode(&bytes[offset..], addr, mode)
        }
    }

    /// `mov [rsp+arg_0], rbx; push rdi; sub rsp, 0x120` is crackme's `main` prologue, and IDA
    /// reports `frsize` 0x128 for it.
    #[test]
    fn frame_size_is_the_pushes_plus_the_allocation() {
        let base = Address::new(0x1000);
        let bytes = [
            0x48, 0x89, 0x5C, 0x24, 0x08, // mov [rsp+8], rbx
            0x57, // push rdi
            0x48, 0x81, 0xEC, 0x20, 0x01, 0x00, 0x00, // sub rsp, 0x120
        ];
        let (frame, bp) = scan_frame(decode_all(&bytes, base, Mode::X64), base, 8);
        assert_eq!(frame, Some(0x128));
        assert!(!bp, "no frame pointer in this prologue");
    }

    /// A 32-bit framed prologue: `push ebp; mov ebp, esp; sub esp, 0x40`.
    #[test]
    fn a_32_bit_frame_pointer_prologue_counts_the_push() {
        let base = Address::new(0x2000);
        let bytes = [
            0x55, // push ebp
            0x8B, 0xEC, // mov ebp, esp
            0x83, 0xEC, 0x40, // sub esp, 0x40
        ];
        let (frame, bp) = scan_frame(decode_all(&bytes, base, Mode::X86), base, 4);
        assert_eq!(frame, Some(0x44));
        assert!(bp, "push ebp / mov ebp, esp is IDA's B flag");
    }

    /// A leaf function with no prologue reports an empty frame, not "unknown".
    #[test]
    fn a_leaf_function_has_a_zero_frame() {
        let base = Address::new(0x3000);
        let bytes = [0x31, 0xC0, 0xC3]; // xor eax, eax; ret
        let (frame, bp) = scan_frame(decode_all(&bytes, base, Mode::X64), base, 8);
        assert_eq!(frame, Some(0));
        assert!(!bp);
    }

    /// IDA's `R`: the body holds a `ret`.
    #[test]
    fn a_returning_body_is_recognised() {
        let base = Address::new(0x4000);
        let bytes = [0x31, 0xC0, 0xC3]; // xor eax, eax; ret
        assert!(scan_returns(
            decode_all(&bytes, base, Mode::X64),
            base,
            Address::new(0x4003)
        ));
        let noreturn = [0xCC, 0xCC]; // int3 padding only
        assert!(!scan_returns(
            decode_all(&noreturn, base, Mode::X64),
            base,
            Address::new(0x4002)
        ));
    }

    /// IDA's own numbers for the corpus crackme: `main` (0x140001010) has `frsize` 0x128 and
    /// `start` (0x140001424) has 0x28, computed from the same prologues this scan walks.
    #[test]
    fn frame_sizes_match_ida_on_the_crackme() {
        use std::path::PathBuf;

        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../corpus/build/real/crackme/crackme.exe");
        let Ok(bytes) = std::fs::read(&path) else {
            eprintln!("skipping: {} is missing", path.display());
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("shape.rcraft");
        let mut session =
            recraft_core::Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let result = crate::analyze(&mut session).unwrap();
        let shape_of = |address: u64| {
            let function = result
                .functions
                .iter()
                .find(|function| function.start.get() == address)
                .unwrap_or_else(|| panic!("no function at {address:#x}"));
            function_shape(&session, function.start, function.end)
        };
        let main = shape_of(0x140001010);
        assert_eq!(main.frame_size, Some(0x128), "IDA: main frsize 0x128");
        assert!(main.returns, "main returns to its caller");
        let start = shape_of(0x140001424);
        assert_eq!(start.frame_size, Some(0x28), "IDA: start frsize 0x28");
    }
}
