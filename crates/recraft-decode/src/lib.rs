// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Instruction decoding and the unified instruction model (x86/x64, iced-x86 backend).
//!
//! Part of ReCraft. See `docs/DESIGN.md` (§5) for the architecture.
//!
//! Design notes:
//! - Decoding is stateless: [`decode`] takes a byte slice + address + mode. Callers
//!   own the image bytes (from `recraft-storage`), so this crate stays dependency-free
//!   apart from `recraft-common` and `iced-x86`.
//! - An `Insn` is a cheap copy around `iced_x86::Instruction` plus ReCraft-level
//!   metadata (address, flow kind). IR/analysis code may consume the raw iced
//!   instruction via [`Insn::instruction`], or the re-exported [`iced_x86`] crate.
//! - Invalid bytes never panic: they decode to an `Insn` with `FlowKind::Invalid`
//!   (length still comes from iced), which callers may treat as data.

#![forbid(unsafe_code)]

use std::collections::{HashMap, VecDeque};
use std::fmt;

use recraft_common::Address;

pub use iced_x86;

pub mod neutral;
pub use neutral::{DecodedInsn, DecodedOperand};

pub mod aarch64;
pub use aarch64::decode_aarch64;

// The iced formatter methods live on a trait; bring it into scope under `_` so it
// does not collide with our own `Formatter` wrapper.
use iced_x86::Formatter as _;

/// Decoding mode / bitness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mode {
    /// 32-bit protected mode (`x86`).
    X86,
    /// 64-bit long mode (`x86-64`).
    X64,
}

impl Mode {
    #[inline]
    pub const fn bitness(self) -> u32 {
        match self {
            Mode::X86 => 32,
            Mode::X64 => 64,
        }
    }

    #[inline]
    pub const fn pointer_size(self) -> usize {
        match self {
            Mode::X86 => 4,
            Mode::X64 => 8,
        }
    }

    #[inline]
    pub const fn is_64(self) -> bool {
        matches!(self, Mode::X64)
    }

    #[inline]
    pub const fn from_is_64(is_64: bool) -> Self {
        if is_64 { Mode::X64 } else { Mode::X86 }
    }
}

/// Control-flow classification of one instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FlowKind {
    /// Falls through to the next instruction.
    Next,
    /// `jmp rel` (direct branch).
    UnconditionalBranch,
    /// `jmp reg/mem` (computed branch).
    IndirectBranch,
    /// `jcc rel` (conditional branch).
    ConditionalBranch,
    /// `call rel` (direct call).
    Call,
    /// `call reg/mem` (computed call).
    IndirectCall,
    /// `ret` / `retn` / `retf` / `iret`.
    Return,
    /// `int` / `int3` / `syscall` / `sysenter`.
    Interrupt,
    /// `ud2`, `hlt`, and other trap/exception instructions.
    Trap,
    /// Bytes that do not decode to a valid instruction.
    Invalid,
}

impl FlowKind {
    /// Whether execution can continue at the next address after this instruction.
    #[inline]
    pub const fn may_fall_through(self) -> bool {
        matches!(
            self,
            FlowKind::Next
                | FlowKind::ConditionalBranch
                | FlowKind::Call
                | FlowKind::IndirectCall
                | FlowKind::Interrupt
        )
    }

    /// Whether this instruction transfers control to another address.
    #[inline]
    pub const fn is_branch(self) -> bool {
        matches!(
            self,
            FlowKind::UnconditionalBranch
                | FlowKind::IndirectBranch
                | FlowKind::ConditionalBranch
                | FlowKind::Call
                | FlowKind::IndirectCall
                | FlowKind::Return
        )
    }
}

/// One decoded instruction.
///
/// Copyable and small-ish (iced's `Instruction` is a fixed-size struct); the decoder
/// cache stores these by address.
#[derive(Clone, Copy)]
pub struct Insn {
    addr: Address,
    mode: Mode,
    len: u8,
    flow: FlowKind,
    instruction: iced_x86::Instruction,
}

impl Insn {
    #[inline]
    pub const fn addr(&self) -> Address {
        self.addr
    }

    #[inline]
    pub const fn mode(&self) -> Mode {
        self.mode
    }

    /// Length in bytes (1..=15 for valid x86 instructions).
    #[inline]
    pub const fn len(&self) -> u8 {
        self.len
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        false
    }

    /// Address of the next instruction (instruction end).
    #[inline]
    pub fn next_addr(&self) -> Address {
        Address::new(self.addr.get().wrapping_add(u64::from(self.len)))
    }

    #[inline]
    pub const fn flow(&self) -> FlowKind {
        self.flow
    }

    #[inline]
    pub const fn is_invalid(&self) -> bool {
        matches!(self.flow, FlowKind::Invalid)
    }

    /// Raw iced instruction (operands, registers, memory operands, ...).
    #[inline]
    pub const fn instruction(&self) -> &iced_x86::Instruction {
        &self.instruction
    }

    #[inline]
    pub fn mnemonic(&self) -> iced_x86::Mnemonic {
        self.instruction.mnemonic()
    }

    /// Direct near branch/call target, if this instruction has one.
    #[inline]
    pub fn branch_target(&self) -> Option<Address> {
        use iced_x86::OpKind;
        match self.instruction.op0_kind() {
            OpKind::NearBranch16 | OpKind::NearBranch32 | OpKind::NearBranch64 => {
                Some(Address::new(self.instruction.near_branch_target()))
            }
            _ => None,
        }
    }

    /// Whether the instruction reads/writes memory.
    #[inline]
    pub fn has_memory_operand(&self) -> bool {
        (0..self.instruction.op_count())
            .any(|index| self.instruction.op_kind(index) == iced_x86::OpKind::Memory)
    }

    // ---- raw iced accessors (operands, memory, immediates) -------------------
    // Thin delegations so IR/analysis code does not need to reach through
    // `instruction()` for the most common queries.

    #[inline]
    pub fn op_count(&self) -> u32 {
        self.instruction.op_count()
    }

    #[inline]
    pub fn op_kind(&self, index: u32) -> iced_x86::OpKind {
        self.instruction.op_kind(index)
    }

    #[inline]
    pub fn op_register(&self, index: u32) -> iced_x86::Register {
        self.instruction.op_register(index)
    }

    #[inline]
    pub fn memory_segment(&self) -> iced_x86::Register {
        self.instruction.memory_segment()
    }

    #[inline]
    pub fn memory_base(&self) -> iced_x86::Register {
        self.instruction.memory_base()
    }

    #[inline]
    pub fn memory_index(&self) -> iced_x86::Register {
        self.instruction.memory_index()
    }

    #[inline]
    pub fn memory_index_scale(&self) -> u32 {
        self.instruction.memory_index_scale()
    }

    #[inline]
    pub fn memory_displacement64(&self) -> u64 {
        self.instruction.memory_displacement64()
    }

    /// True for a memory operand with no base/index register, or a RIP-relative
    /// operand: absolute addressing. iced resolves RIP-relative operands to an
    /// absolute address, so [`Insn::memory_displacement64`] holds the target.
    #[inline]
    pub fn memory_is_direct(&self) -> bool {
        let base = self.instruction.memory_base();
        self.has_memory_operand()
            && (base == iced_x86::Register::None || base == iced_x86::Register::RIP)
            && self.instruction.memory_index() == iced_x86::Register::None
    }

    /// Indexed absolute/RIP-relative memory, i.e. `[table + index*scale]`.
    ///
    /// Returns `(table_address, scale)`. Used for jump-table recognition:
    /// `jmp qword ptr [rax*8 + table]` / `[table + rcx*4]`.
    #[inline]
    pub fn memory_indexed_table(&self) -> Option<(u64, u32)> {
        if !self.has_memory_operand() {
            return None;
        }
        let base = self.instruction.memory_base();
        if base != iced_x86::Register::None && base != iced_x86::Register::RIP {
            return None;
        }
        if self.instruction.memory_index() == iced_x86::Register::None {
            return None;
        }
        Some((
            self.instruction.memory_displacement64(),
            self.instruction.memory_index_scale(),
        ))
    }

    #[inline]
    pub fn memory_size(&self) -> iced_x86::MemorySize {
        self.instruction.memory_size()
    }

    /// Addresses this instruction's operands name as *data*.
    ///
    /// IDA types every operand that points at data: `lea rcx, aEnterFlag`,
    /// `push offset aUsage`, `dq offset aCaption`. The listing uses these addresses to find
    /// the item behind the operand and print the literal it holds (`; "Enter flag: "`).
    /// Nothing is checked against the program map here: the caller filters.
    ///
    /// A memory operand contributes its direct address (RIP-relative operands are already
    /// absolute) or an indexed table's base; an address-wide immediate contributes its value
    /// and is the one case IDA prints as `offset …`.
    pub fn data_operand_addresses(&self) -> Vec<u64> {
        use iced_x86::OpKind;
        let instruction = &self.instruction;
        let mut addresses = Vec::new();
        if self.memory_is_direct() {
            addresses.push(self.memory_displacement64());
        } else if let Some((table, _)) = self.memory_indexed_table() {
            addresses.push(table);
        }
        for index in 0..instruction.op_count() {
            match instruction.op_kind(index) {
                OpKind::Immediate32 | OpKind::Immediate32to64 | OpKind::Immediate64 => {
                    addresses.push(instruction.immediate(index));
                }
                _ => {}
            }
        }
        addresses
    }

    #[inline]
    pub fn is_ip_rel_memory_operand(&self) -> bool {
        self.instruction.is_ip_rel_memory_operand()
    }

    #[inline]
    pub fn immediate8(&self) -> u8 {
        self.instruction.immediate8()
    }

    #[inline]
    pub fn immediate8to16(&self) -> i16 {
        self.instruction.immediate8to16()
    }

    #[inline]
    pub fn immediate8to32(&self) -> i32 {
        self.instruction.immediate8to32()
    }

    #[inline]
    pub fn immediate8to64(&self) -> i64 {
        self.instruction.immediate8to64()
    }

    #[inline]
    pub fn immediate16(&self) -> u16 {
        self.instruction.immediate16()
    }

    #[inline]
    pub fn immediate32(&self) -> u32 {
        self.instruction.immediate32()
    }

    #[inline]
    pub fn immediate32to64(&self) -> i64 {
        self.instruction.immediate32to64()
    }

    #[inline]
    pub fn immediate64(&self) -> u64 {
        self.instruction.immediate64()
    }

    /// Format as Intel syntax, e.g. `mov rax, rbx` (allocates; for UI/CLI/tests).
    pub fn asm(&self) -> String {
        let mut formatter = Formatter::new();
        formatter.format(self).to_owned()
    }
}

impl fmt::Debug for Insn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Insn")
            .field("addr", &self.addr)
            .field("len", &self.len)
            .field("flow", &self.flow)
            .field("asm", &self.asm())
            .finish()
    }
}

/// Decode one instruction at `bytes[0]`, pretending it lives at `ip`.
///
/// Returns `None` only when there is nothing to decode (empty slice). Invalid bytes
/// return an [`Insn`] with [`FlowKind::Invalid`].
pub fn decode(bytes: &[u8], ip: Address, mode: Mode) -> Option<Insn> {
    if bytes.is_empty() {
        return None;
    }
    let mut decoder = iced_x86::Decoder::with_ip(
        mode.bitness(),
        bytes,
        ip.get(),
        iced_x86::DecoderOptions::NONE,
    );
    let instruction = decoder.decode();
    let len = instruction.len();
    if len == 0 {
        return None;
    }
    let flow = classify_flow(&instruction);
    Some(Insn {
        addr: ip,
        mode,
        len: len as u8,
        flow,
        instruction,
    })
}

fn classify_flow(instruction: &iced_x86::Instruction) -> FlowKind {
    use iced_x86::FlowControl;
    if instruction.is_invalid() {
        return FlowKind::Invalid;
    }
    match instruction.flow_control() {
        FlowControl::Next => FlowKind::Next,
        FlowControl::UnconditionalBranch => FlowKind::UnconditionalBranch,
        FlowControl::IndirectBranch => FlowKind::IndirectBranch,
        FlowControl::ConditionalBranch => FlowKind::ConditionalBranch,
        FlowControl::Return => FlowKind::Return,
        FlowControl::Call => FlowKind::Call,
        FlowControl::IndirectCall => FlowKind::IndirectCall,
        FlowControl::Interrupt => FlowKind::Interrupt,
        FlowControl::Exception => FlowKind::Trap,
        FlowControl::XbeginXabortXend => FlowKind::Next,
    }
}

/// Bounded decode cache: insertion order is used for eviction (FIFO).
///
/// Decoding is cheap, but the analysis pipeline decodes the same bytes many times
/// (recursive descent + xrefs + formatting). The cache is keyed by address: callers
/// must invalidate entries when bytes change (patches, user code/data edits).
///
/// A hit is a plain hash lookup. An earlier revision also moved the key to the back of
/// `order` on every hit to approximate LRU, but that scan is O(cache size) and, once the
/// cache filled up, dominated every analysis pass (a 3 261-function Go sample needed ~20
/// minutes for `recraft lint` almost entirely in that scan). FIFO keeps the working set of
/// one function, which is what matters here, without the per-hit cost.
pub struct DecodeCache {
    entries: HashMap<u64, Insn>,
    order: VecDeque<u64>,
    capacity: usize,
}

impl DecodeCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: HashMap::with_capacity(capacity.min(4096)),
            order: VecDeque::with_capacity(capacity.min(4096)),
            capacity,
        }
    }

    pub fn get(&mut self, addr: Address) -> Option<Insn> {
        self.entries.get(&addr.get()).copied()
    }

    pub fn insert(&mut self, insn: Insn) {
        let key = insn.addr().get();
        if let std::collections::hash_map::Entry::Occupied(mut entry) = self.entries.entry(key) {
            entry.insert(insn);
            return;
        }
        while self.order.len() >= self.capacity {
            if let Some(evicted) = self.order.pop_front() {
                self.entries.remove(&evicted);
            } else {
                break;
            }
        }
        self.entries.insert(key, insn);
        self.order.push_back(key);
    }

    /// Drop the entry at `addr` (after a patch or a code/data change).
    pub fn invalidate(&mut self, addr: Address) {
        if self.entries.remove(&addr.get()).is_some() {
            if let Some(position) = self.order.iter().position(|&key| key == addr.get()) {
                self.order.remove(position);
            }
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn get_or_decode(&mut self, bytes: &[u8], addr: Address, mode: Mode) -> Option<Insn> {
        if let Some(insn) = self.get(addr) {
            return Some(insn);
        }
        let insn = decode(bytes, addr, mode)?;
        self.insert(insn);
        Some(insn)
    }
}

/// Source of the names the disassembler substitutes for numeric addresses.
///
/// IDA's listing never prints a call target or a global as a bare number: it prints the
/// symbol (`call sub_140001678`, `mov cs:byte_140023B40, 1`). The formatter asks this trait
/// while it renders, so callers that have a project can hand it one and callers that only
/// own bytes ([`Insn::asm`]) keep the plain numbers.
pub trait Symbols {
    /// Name of the code at `address` — a call or branch target.
    fn code(&mut self, address: u64) -> Option<String>;

    /// Name of the data at `address`, addressed with `size` bytes (`1`, `2`, `4`, `8`, …).
    fn data(&mut self, address: u64, size: u32) -> Option<String>;

    /// Name of the data an *immediate* operand names (`push offset aUsage`).
    ///
    /// Only an item that already has a name resolves here. A plain number that happens to
    /// fall inside a data segment (`mov eax, 3000h` in a PIE image) must stay a number, so
    /// the address-based `unk_…`/`dword_…` fallback is not used for immediates.
    fn offset(&mut self, address: u64) -> Option<String> {
        self.data(address, 0)
    }
}

/// Bridges [`Symbols`] to iced's resolver, which is consulted for branch targets, memory
/// displacements and immediates alike. Immediates are left alone: `add rsp, 28h` must stay a
/// number, so only branch and memory operands are resolved.
struct SymbolAdapter {
    symbols: Box<dyn Symbols>,
}

impl iced_x86::SymbolResolver for SymbolAdapter {
    fn symbol(
        &mut self,
        instruction: &iced_x86::Instruction,
        _operand: u32,
        instruction_operand: Option<u32>,
        address: u64,
        _address_size: u32,
    ) -> Option<iced_x86::SymbolResult<'_>> {
        let index = instruction_operand?;
        let text = match instruction.op_kind(index) {
            iced_x86::OpKind::NearBranch16
            | iced_x86::OpKind::NearBranch32
            | iced_x86::OpKind::NearBranch64
            | iced_x86::OpKind::FarBranch16
            | iced_x86::OpKind::FarBranch32 => self.symbols.code(address)?,
            iced_x86::OpKind::Memory => {
                let size = instruction.memory_size().size() as u32;
                self.symbols.data(address, size)?
            }
            // An address-wide immediate is a data reference in IDA (`push offset aUsage`);
            // only an item that already has a name resolves (see [`Symbols::offset`]).
            iced_x86::OpKind::Immediate32
            | iced_x86::OpKind::Immediate32to64
            | iced_x86::OpKind::Immediate64 => {
                format!("offset {}", self.symbols.offset(address)?)
            }
            _ => return None,
        };
        Some(iced_x86::SymbolResult {
            address,
            text: iced_x86::SymResTextInfo::with_string(text, iced_x86::FormatterTextKind::Label),
            flags: iced_x86::SymbolFlags::NONE,
            symbol_size: None,
        })
    }
}

/// Intel-syntax formatter.
///
/// [`Formatter::new`] keeps the C-style `0x` hex that the IR and `__asm("…")` text use;
/// [`Formatter::ida`] switches to IDA's listing syntax (bare `h`-suffixed hex) and resolves
/// symbol names.
pub struct Formatter {
    inner: iced_x86::IntelFormatter,
    output: String,
    /// Whether the output is IDA's listing text, which drops the brackets around an
    /// absolute memory operand (`mov rax, cs:qword_140023B38`, not `[qword_…]`).
    ida: bool,
}

impl Formatter {
    /// C-style hex (`0x401000`) with no symbol resolution.
    pub fn new() -> Self {
        let mut inner = iced_x86::IntelFormatter::new();
        Self::configure(&mut inner, false);
        Self {
            inner,
            output: String::with_capacity(64),
            ida: false,
        }
    }

    /// IDA listing syntax: `28h`, `0FFFFFFFFFFFFFFFFh`, `sub_140001678`.
    ///
    /// `symbols` names branch and data targets; `None` keeps the numbers.
    pub fn ida(symbols: Option<Box<dyn Symbols>>) -> Self {
        let resolver: Option<Box<dyn iced_x86::SymbolResolver>> = symbols.map(|symbols| {
            Box::new(SymbolAdapter { symbols }) as Box<dyn iced_x86::SymbolResolver>
        });
        let mut inner = iced_x86::IntelFormatter::with_options(resolver, None);
        Self::configure(&mut inner, true);
        Self {
            inner,
            output: String::with_capacity(64),
            ida: true,
        }
    }

    fn configure(inner: &mut iced_x86::IntelFormatter, ida: bool) {
        let options = inner.options_mut();
        options.set_space_after_operand_separator(true);
        if ida {
            // IDA writes `28h`, `0Ah` (a leading 0 when the first digit is a letter) and
            // no `0x` prefix; iced's formatter supplies that leading zero itself.
            options.set_hex_prefix("");
            options.set_hex_suffix("h");
            options.set_uppercase_hex(true);
        } else {
            options.set_hex_prefix("0x");
            options.set_hex_suffix("");
            options.set_uppercase_hex(false);
        }
        options.set_leading_zeros(false);
        // IDA prints 0–9 in decimal (`mov al, 1`) and everything wider in hex (`28h`).
        options.set_small_hex_numbers_in_decimal(ida);
        options.set_branch_leading_zeros(false);
    }

    /// Full Intel-syntax text: `mov rax, rbx`, `call 0x401000`, `ret`.
    pub fn format(&mut self, insn: &Insn) -> &str {
        self.output.clear();
        // `format` takes the instruction by reference and appends to the output.
        self.inner.format(insn.instruction(), &mut self.output);
        if self.ida && insn.memory_is_direct() {
            self.output = absolute_memory_operand(&self.output, insn);
        }
        &self.output
    }

    /// Split into `(mnemonic, operands)` for column-based UIs.
    pub fn format_parts(&mut self, insn: &Insn) -> (&str, &str) {
        let text = self.format(insn);
        match text.split_once(' ') {
            Some((mnemonic, operands)) => (mnemonic, operands),
            None => (text, ""),
        }
    }
}

impl Default for Formatter {
    fn default() -> Self {
        Self::new()
    }
}

/// IDA's spelling of an absolute memory operand: no brackets.
///
/// iced always brackets one (`mov rax, [qword_140023B38]`, `lea rcx, [aEnterFlag]`,
/// `mov rax, gs:[30h]`). IDA brackets only register-relative operands: an absolute read or
/// write becomes `cs:qword_140023B38` (the segment IDA otherwise leaves implicit), and a
/// `lea`, which computes an address instead of touching memory, drops the brackets and the
/// segment alike (`lea rcx, aEnterFlag`).
fn absolute_memory_operand(text: &str, insn: &Insn) -> String {
    let Some(open) = text.find('[') else {
        return text.to_owned();
    };
    let Some(close) = text[open..].find(']').map(|offset| open + offset) else {
        return text.to_owned();
    };
    let (before, inner, after) = (&text[..open], &text[open + 1..close], &text[close + 1..]);
    // The operand is a name when it starts with a letter or underscore: IDA puts the item's
    // type in the name itself (`byte_…`, `qword_…`) and drops iced's `… ptr` keyword.
    let bare_address = inner
        .chars()
        .next()
        .is_none_or(|ch| ch.is_ascii_digit() || ch == '-');
    let mut rewritten = String::with_capacity(text.len());
    rewritten.push_str(if bare_address {
        before
    } else {
        without_size_keyword(before)
    });
    // A segment iced already printed (`gs:[30h]`) stays; it is the more specific override.
    // IDA prints the segment in front of every memory access, and in front of a `lea` only
    // when the operand is a bare address instead of an item's name (`lea rdi,
    // cs:140000000h` vs `lea rcx, aEnterFlag`).
    let computes_address = matches!(insn.mnemonic(), iced_x86::Mnemonic::Lea);
    if (!computes_address || bare_address) && !before.ends_with(':') {
        // 64-bit code runs in one flat code segment; a 32-bit data reference defaults to
        // `ds`. Either is what IDA prints in front of an absolute address.
        rewritten.push_str(
            if insn.instruction().code_size() == iced_x86::CodeSize::Code64 {
                "cs:"
            } else {
                "ds:"
            },
        );
    }
    rewritten.push_str(inner);
    rewritten.push_str(after);
    rewritten
}

/// `mov byte ptr ` → `mov ` (iced's size keyword, which a named item makes redundant).
fn without_size_keyword(before: &str) -> &str {
    let Some(head) = before.strip_suffix(" ptr ") else {
        return before;
    };
    match head.rfind(' ') {
        Some(index) => &head[..index + 1],
        None => before,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode64(bytes: &[u8], ip: u64) -> Insn {
        decode(bytes, Address::new(ip), Mode::X64).expect("decoded")
    }

    fn decode32(bytes: &[u8], ip: u64) -> Insn {
        decode(bytes, Address::new(ip), Mode::X86).expect("decoded")
    }

    #[test]
    fn empty_input_returns_none() {
        assert!(decode(&[], Address::ZERO, Mode::X64).is_none());
    }

    #[test]
    fn basic_instruction_lengths_and_flow() {
        let push = decode64(&[0x55], 0x1000);
        assert_eq!(push.len(), 1);
        assert_eq!(push.flow(), FlowKind::Next);
        assert_eq!(push.asm(), "push rbp");
        assert_eq!(push.next_addr(), Address::new(0x1001));

        let mov = decode64(&[0x48, 0x89, 0xe5], 0x1000);
        assert_eq!(mov.len(), 3);
        assert_eq!(mov.asm(), "mov rbp, rsp");

        let ret = decode64(&[0xc3], 0x1000);
        assert_eq!(ret.flow(), FlowKind::Return);
        assert!(ret.branch_target().is_none());

        let int3 = decode64(&[0xcc], 0x1000);
        assert_eq!(int3.flow(), FlowKind::Interrupt);
    }

    #[test]
    fn direct_call_and_branch_targets() {
        // call +0x10 from 0x1000: next = 0x1005, target = 0x1015
        let call = decode64(&[0xe8, 0x10, 0x00, 0x00, 0x00], 0x1000);
        assert_eq!(call.flow(), FlowKind::Call);
        assert_eq!(call.branch_target(), Some(Address::new(0x1015)));
        assert!(call.flow().may_fall_through());

        // jmp -2 (infinite loop)
        let jmp = decode64(&[0xeb, 0xfe], 0x2000);
        assert_eq!(jmp.flow(), FlowKind::UnconditionalBranch);
        assert_eq!(jmp.branch_target(), Some(Address::new(0x2000)));
        assert!(!jmp.flow().may_fall_through());

        // je +5
        let je = decode64(&[0x74, 0x05], 0x3000);
        assert_eq!(je.flow(), FlowKind::ConditionalBranch);
        assert_eq!(je.branch_target(), Some(Address::new(0x3007)));
        assert!(je.flow().may_fall_through());
    }

    #[test]
    fn indirect_jump_and_call() {
        // jmp qword ptr [rip]
        let jmp = decode64(&[0xff, 0x25, 0x00, 0x00, 0x00, 0x00], 0x1000);
        assert_eq!(jmp.flow(), FlowKind::IndirectBranch);
        assert_eq!(jmp.len(), 6);
        assert!(jmp.branch_target().is_none());
        assert!(jmp.has_memory_operand());

        // call rax
        let call = decode64(&[0xff, 0xd0], 0x1000);
        assert_eq!(call.flow(), FlowKind::IndirectCall);
    }

    #[test]
    fn mode_changes_decoding() {
        // 0x40 is `inc eax` in 32-bit, but a REX prefix in 64-bit.
        let inc = decode32(&[0x40, 0x90], 0x1000);
        assert_eq!(inc.len(), 1);
        assert_eq!(inc.mnemonic(), iced_x86::Mnemonic::Inc);

        let rex_nop = decode64(&[0x40, 0x90], 0x1000);
        assert_eq!(rex_nop.len(), 2);
        assert_eq!(rex_nop.mnemonic(), iced_x86::Mnemonic::Nop);

        assert_eq!(Mode::X64.bitness(), 64);
        assert_eq!(Mode::X86.pointer_size(), 4);
    }

    #[test]
    fn invalid_bytes_do_not_panic() {
        let insn =
            decode(&[0x0f, 0xff], Address::new(0x1000), Mode::X64).expect("some instruction");
        assert!(!insn.is_empty());
        // Whether iced calls it invalid or a valid-but-odd encoding, we must not panic.
        let _ = insn.asm();
    }

    #[test]
    fn trap_instructions() {
        // ud2
        let ud2 = decode64(&[0x0f, 0x0b], 0x1000);
        assert_eq!(ud2.flow(), FlowKind::Trap);
    }

    #[test]
    fn formatter_splits_mnemonic_and_operands() {
        let insn = decode64(&[0x48, 0x89, 0xe5], 0x1000);
        let mut formatter = Formatter::new();
        let (mnemonic, operands) = formatter.format_parts(&insn);
        assert_eq!(mnemonic, "mov");
        assert_eq!(operands, "rbp, rsp");

        let ret = decode64(&[0xc3], 0x1000);
        assert_eq!(formatter.format(&ret), "ret");
        let (mnemonic, operands) = formatter.format_parts(&ret);
        assert_eq!(mnemonic, "ret");
        assert_eq!(operands, "");
    }

    #[test]
    fn ida_syntax_writes_h_suffixed_hex() {
        let mut formatter = Formatter::ida(None);
        // `sub rsp, 28h` — IDA's suffix form, not iced's default `0x28`.
        let sub = decode64(&[0x48, 0x83, 0xec, 0x28], 0x140001424);
        assert_eq!(formatter.format(&sub), "sub rsp, 28h");
        // A first digit that is a letter gets IDA's leading zero: `and al, 0Ah`.
        let and = decode64(&[0x24, 0x0a], 0x140001000);
        assert_eq!(formatter.format(&and), "and al, 0Ah");
        // `0x` stays for the IR/`__asm` text.
        assert_eq!(Formatter::new().format(&sub), "sub rsp, 0x28");
    }

    /// Names the formatter may substitute; counts how often it is asked.
    struct Names {
        map: Vec<(u64, &'static str)>,
    }

    impl Symbols for Names {
        fn code(&mut self, address: u64) -> Option<String> {
            self.map
                .iter()
                .find(|(target, _)| *target == address)
                .map(|(_, name)| (*name).to_owned())
        }

        fn data(&mut self, address: u64, _size: u32) -> Option<String> {
            self.code(address)
        }
    }

    #[test]
    fn ida_syntax_resolves_symbols() {
        let symbols = Names {
            map: vec![
                (0x140001678, "sub_140001678"),
                (0x140001466, "loc_140001466"),
                (0x140023b40, "byte_140023B40"),
            ],
        };
        let mut formatter = Formatter::ida(Some(Box::new(symbols)));

        // `call rel32` prints the target's name instead of the number.
        let call = decode64(&[0xe8, 0x4b, 0x02, 0x00, 0x00], 0x140001428);
        assert_eq!(formatter.format(&call), "call sub_140001678");

        // `je rel8` keeps iced's `short` prefix and takes the label.
        let je = decode64(&[0x74, 0x21], 0x140001443);
        assert_eq!(formatter.format(&je), "je short loc_140001466");

        // A RIP-relative memory operand (how x64 reaches a global) becomes the data name,
        // spelled the way IDA does: `cs:` instead of brackets, and no `byte ptr` keyword
        // because the name already carries the item's size.
        let mov = decode64(&[0xc6, 0x05, 0xc5, 0x26, 0x02, 0x00, 0x01], 0x140001474);
        assert_eq!(formatter.format(&mov), "mov cs:byte_140023B40, 1");
    }

    #[test]
    fn ida_syntax_drops_the_brackets_of_an_absolute_operand() {
        /// A `lea` reaches a literal; a read reaches a global.
        struct Globals;

        impl Symbols for Globals {
            fn code(&mut self, _address: u64) -> Option<String> {
                None
            }

            fn data(&mut self, address: u64, size: u32) -> Option<String> {
                match (address, size) {
                    (0x140018438, _) => Some("aEnterFlag".to_owned()),
                    (0x140023b38, 8) => Some("qword_140023B38".to_owned()),
                    // A `lea` has no access width: IDA types the item `unk_…`.
                    (0x140024bd0, 0) => Some("unk_140024BD0".to_owned()),
                    _ => None,
                }
            }
        }

        let mut formatter = Formatter::ida(Some(Box::new(Globals)));
        // `lea rcx, [rip+disp]` — IDA: the item's name alone, no segment, no brackets.
        let lea = decode64(&[0x48, 0x8d, 0x0d, 0xd1, 0x73, 0x01, 0x00], 0x140001060);
        assert_eq!(formatter.format(&lea), "lea rcx, aEnterFlag");
        // A read keeps the segment prefix: `lock cmpxchg cs:qword_…, rcx`.
        let cmpxchg = decode64(
            &[0xf0, 0x48, 0x0f, 0xb1, 0x0d, 0xd4, 0x26, 0x02, 0x00],
            0x14000145b,
        );
        assert_eq!(
            formatter.format(&cmpxchg),
            "lock cmpxchg cs:qword_140023B38, rcx"
        );
        // An absolute address with no item keeps the number but not the brackets, and the
        // segment says the operand is memory.
        let untyped = decode64(&[0x48, 0x8d, 0x05, 0xc9, 0x3b, 0x02, 0x00], 0x140001000);
        assert_eq!(formatter.format(&untyped), "lea rax, unk_140024BD0");
        // Register-relative operands keep IDA's brackets.
        let stack = decode64(&[0x48, 0x89, 0x5c, 0x24, 0x08], 0x140001010);
        assert_eq!(formatter.format(&stack), "mov [rsp+8], rbx");
        // The C-style formatter (IR/`__asm` text) keeps iced's plain output.
        assert_eq!(Formatter::new().format(&stack), "mov [rsp+0x8], rbx");
    }

    #[test]
    fn cache_hits_and_evicts() {
        let bytes = [0x90u8; 16];
        let mut cache = DecodeCache::new(4);
        let a = decode(&bytes, Address::new(0x1000), Mode::X64).unwrap();
        cache.insert(a);
        assert_eq!(cache.len(), 1);
        assert!(cache.get(Address::new(0x1000)).is_some());
        assert!(cache.get(Address::new(0x2000)).is_none());

        for i in 0..10u64 {
            let insn = decode(&bytes, Address::new(0x1000 + i), Mode::X64).unwrap();
            cache.insert(insn);
        }
        assert_eq!(cache.len(), 4);
        assert_eq!(cache.capacity(), 4);

        cache.invalidate(Address::new(0x1009));
        assert_eq!(cache.len(), 3);
        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn get_or_decode_populates_cache() {
        let bytes = [0x90u8; 4];
        let mut cache = DecodeCache::new(8);
        let insn = cache
            .get_or_decode(&bytes, Address::new(0x4000), Mode::X64)
            .unwrap();
        assert_eq!(insn.len(), 1);
        assert_eq!(cache.len(), 1);
        let again = cache
            .get_or_decode(&bytes, Address::new(0x4000), Mode::X64)
            .unwrap();
        assert_eq!(again.addr(), Address::new(0x4000));
        assert_eq!(cache.len(), 1);
    }

    /// Manual throughput benchmark, excluded from CI:
    /// `cargo test -p recraft-decode --release -- --ignored bench_decode --nocapture`
    #[test]
    #[ignore = "manual performance benchmark"]
    fn bench_decode_throughput() {
        let bytes = [0x90u8; 64 * 1024]; // NOPs: max decode rate, 1 byte each
        let iterations = 1000u64;
        let mut total: u64 = 0;
        let start = std::time::Instant::now();
        for _ in 0..iterations {
            let mut offset = 0usize;
            let mut ip = 0x1000u64;
            while offset < bytes.len() {
                let insn = decode(&bytes[offset..], Address::new(ip), Mode::X64).unwrap();
                offset += insn.len() as usize;
                ip += u64::from(insn.len());
                total += 1;
            }
        }
        let elapsed = start.elapsed();
        let rate = total as f64 / elapsed.as_secs_f64() / 1e6;
        println!(
            "decoded {total} instructions in {elapsed:?} ({rate:.1} M insn/s, stateless decode one-at-a-time)"
        );
    }
}
