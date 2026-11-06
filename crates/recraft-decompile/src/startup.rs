// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! `main` detection.
//!
//! A statically linked MSVC program carries no `main` symbol, but its CRT startup calls it in a
//! fixed shape: `argc`, `argv` and `envp` are loaded into `ecx`/`rdx`/`r8`, the call's return
//! and matching that shape names `main` the way a signature database would — the difference is
//! that this needs no bytes from a particular CRT build.

use recraft_common::Address;
use recraft_core::Session;
use recraft_decode::iced_x86::{Mnemonic, Register};

const MAX_HOPS: usize = 3;
/// How many instructions to scan inside the startup function.
const MAX_SCAN: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    /// Console program: the startup passes `argc`/`argv`/`envp`.
    Main,
    /// GUI program: the startup passes `hInstance`/`hPrevInstance`/`lpCmdLine`/`nShowCmd`.
    WinMain,
    /// DLL: the startup passes `hinstDLL`/`fdwReason`/`lpvReserved`.
    DllMain,
}

impl EntryKind {
    pub fn name(self) -> &'static str {
        match self {
            EntryKind::Main => "main",
            EntryKind::WinMain => "WinMain",
            EntryKind::DllMain => "DllMain",
        }
    }
}

///
/// (`mainCRTStartup` → `__scrt_common_main_seh`), so the search starts by following those
/// jumps and then looks for the `argc`/`argv`/`envp` call.
pub fn find_entry(session: &mut Session) -> Option<(Address, EntryKind)> {
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

fn candidate_in(session: &mut Session, start: Address) -> Option<(Address, EntryKind)> {
    // The startup is usually a known function; when discovery missed it (a stub the
    // analyzer folded away), scan a bounded window instead.
    let end = function_end(session, start)
        .unwrap_or_else(|| Address::new(start.get().saturating_add(0x1000)));
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
        // The return value is saved and the exit path runs immediately after: that is the
        // startup calling the program's entry.
        if !saves_return(&instructions[index + 1]) {
            continue;
        }
        if instructions[index + 2].mnemonic() != Mnemonic::Call {
            continue;
        }
        let Some(kind) = classify(&instructions[..index]) else {
            continue;
        };
        return Some((target, kind));
    }
    // A DLL startup (`_DllMainCRTStartup`) calls `DllMain` instead: the BOOL result is
    // tested right after the call rather than saved.
    for index in 1..instructions.len().saturating_sub(1) {
        let call = &instructions[index];
        if call.mnemonic() != Mnemonic::Call {
            continue;
        }
        let Some(target) = call.branch_target() else {
            continue;
        };
        let after = &instructions[index + 1..(index + 5).min(instructions.len())];
        if !after.iter().any(tests_return) {
            continue;
        }
        if !sets_third_argument(&instructions[..index]) {
            continue;
        }
        return Some((target, EntryKind::DllMain));
    }
    None
}

/// Whether the argument setup before the call matches a console or a GUI entry.
fn classify(before: &[recraft_decode::Insn]) -> Option<EntryKind> {
    let window = &before[before.len().saturating_sub(8)..];
    let mut writes_ecx = false;
    let mut writes_rcx = false;
    let mut writes_r9 = false;
    for insn in window {
        if !writes_first_operand(insn) {
            continue;
        }
        match insn.op_register(0) {
            Register::ECX => writes_ecx = true,
            Register::RCX => writes_rcx = true,
            Register::R9 | Register::R9D => writes_r9 = true,
            _ => {}
        }
    }
    // `main(argc, argv, envp)`: `argc` is a 32-bit `ecx` load.
    if writes_ecx && !writes_rcx {
        return Some(EntryKind::Main);
    }
    // `WinMain(hInstance, hPrevInstance, lpCmdLine, nShowCmd)`: a 64-bit `rcx` and a fourth
    // argument in `r9`.
    if writes_rcx && writes_r9 {
        return Some(EntryKind::WinMain);
    }
    None
}

/// Whether the instruction writes its first operand (a rough "this sets the register" test).
fn writes_first_operand(insn: &recraft_decode::Insn) -> bool {
    if insn.op_count() < 1 {
        return false;
    }
    matches!(
        insn.mnemonic(),
        Mnemonic::Mov
            | Mnemonic::Movzx
            | Mnemonic::Movsx
            | Mnemonic::Lea
            | Mnemonic::Xor
            | Mnemonic::Add
            | Mnemonic::Sub
            | Mnemonic::And
            | Mnemonic::Or
            | Mnemonic::Movsxd
    ) && recraft_ir::reg::from_iced(insn.op_register(0)).is_some()
}

/// Whether the instruction tests the call's return value (`test eax, eax`).
fn tests_return(insn: &recraft_decode::Insn) -> bool {
    insn.mnemonic() == Mnemonic::Test
        && insn.op_count() >= 2
        && insn.op_register(0) == Register::EAX
}

/// Whether the third argument (`r8`) is set shortly before the call.
fn sets_third_argument(before: &[recraft_decode::Insn]) -> bool {
    let window = &before[before.len().saturating_sub(8)..];
    window
        .iter()
        .any(|insn| writes_first_operand(insn) && insn.op_register(0) == Register::R8)
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
