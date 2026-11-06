// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! `main` detection.
//!
//! A statically linked MSVC program carries no `main` symbol, but its CRT startup calls it in a
//! fixed shape: `argc`, `argv` and `envp` are loaded into `ecx`/`rdx`/`r8`, the call's return
//! value is saved, and the exit path runs right after. Following the entry point to the startup
//! and matching that shape names `main` the way a signature database would — the difference is
//! that this needs no bytes from a particular CRT build.

use recraft_common::Address;
use recraft_core::Session;
use recraft_decode::iced_x86::{Mnemonic, Register};

/// How many tail jumps to follow from the entry point before giving up.
const MAX_HOPS: usize = 3;
/// How many instructions to scan inside the startup function.
const MAX_SCAN: usize = 4096;

/// The function the CRT startup passes `argc`/`argv`/`envp` to, i.e. `main`.
///
/// The entry point of a static CRT program is a short stub that tail-jumps into the startup
/// (`mainCRTStartup` → `__scrt_common_main_seh`), so the search starts by following those
/// jumps and then looks for the `argc`/`argv`/`envp` call.
pub fn find_main(session: &mut Session) -> Option<Address> {
    let mut start = session.entry_point();
    for _ in 0..MAX_HOPS {
        let Some(next) = tail_jump(session, start) else {
            break;
        };
        start = next;
    }
    candidate_in(session, start)
}

/// The target of a function's last instruction, when that instruction is a `jmp` (tail call).
fn tail_jump(session: &mut Session, start: Address) -> Option<Address> {
    let end = function_end(session, start)?;
    let mut address = start;
    let mut last = None;
    while address < end {
        let Some(insn) = session.insn_at(address).ok().flatten() else {
            break;
        };
        address = insn.next_addr();
        last = Some(insn);
    }
    let insn = last?;
    if insn.mnemonic() != Mnemonic::Jmp {
        return None;
    }
    insn.branch_target()
}

/// The end address of the function starting at `start`.
fn function_end(session: &Session, start: Address) -> Option<Address> {
    session
        .functions()
        .ok()?
        .into_iter()
        .find(|function| function.start == start)
        .map(|function| function.end)
}

/// The `main` call inside the startup function, if its shape is the expected one.
fn candidate_in(session: &mut Session, start: Address) -> Option<Address> {
    let end = function_end(session, start)?;
    let mut instructions = Vec::new();
    let mut address = start;
    while address < end && instructions.len() < MAX_SCAN {
        let Some(insn) = session.insn_at(address).ok().flatten() else {
            break;
        };
        address = insn.next_addr();
        instructions.push(insn);
    }
    for index in 1..instructions.len().saturating_sub(2) {
        let call = &instructions[index];
        if call.mnemonic() != Mnemonic::Call {
            continue;
        }
        let Some(target) = call.branch_target() else {
            continue;
        };
        // `argc` is loaded into `ecx` right before the call ...
        if !loads_argc(&instructions[index - 1]) {
            continue;
        }
        // ... its return value is saved, and the exit path runs immediately after.
        if !saves_return(&instructions[index + 1]) {
            continue;
        }
        if instructions[index + 2].mnemonic() != Mnemonic::Call {
            continue;
        }
        return Some(target);
    }
    None
}

/// Whether the instruction loads a value into `ecx` (the `argc` argument).
fn loads_argc(insn: &recraft_decode::Insn) -> bool {
    if !matches!(
        insn.mnemonic(),
        Mnemonic::Mov | Mnemonic::Movzx | Mnemonic::Movsx
    ) {
        return false;
    }
    insn.op_count() >= 1 && insn.op_register(0) == Register::ECX
}

/// Whether the instruction saves the call's return value (`mov r32, eax`).
fn saves_return(insn: &recraft_decode::Insn) -> bool {
    if insn.mnemonic() != Mnemonic::Mov || insn.op_count() < 2 {
        return false;
    }
    if insn.op_register(1) != Register::EAX {
        return false;
    }
    recraft_ir::reg::from_iced(insn.op_register(0)).is_some_and(|view| view.width == 4)
}
