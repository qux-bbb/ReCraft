// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Function signatures: a byte pattern for the head of a function, with wildcards where a byte
//! encodes an address.
//!
//! A signature is how a name learned in one project reaches another binary built from the same
//! library — the FLIRT model, except that nothing is shipped: the user exports the functions
//! they have named and imports the file where they need it. Addresses are the only bytes that
//! differ between two builds of one library, so a relative branch, a RIP-relative displacement
//! and an immediate that is large enough to be an address are wildcarded; everything else must
//! match byte for byte.

use recraft_common::Address;
use recraft_core::Session;

/// Signatures shipped with the binary: the x64 Visual C++ static CRT set.
///
/// Patterns are build specific; `signature-export` produces more for other CRT versions.
pub const BUILTIN_SIGNATURES: &str = include_str!("builtin_signatures.json");
/// How many bytes of a function head a signature covers.
pub const SIGNATURE_BYTES: usize = 32;

/// A pattern shorter than this says too little to be worth matching.
pub const MIN_PATTERN_BYTES: usize = 8;

/// An immediate at least this large is treated as an address and wildcarded.
const ADDRESS_LIKE_IMMEDIATE: u64 = 0x1_0000;

/// One function head, as bytes that must match and bytes that may not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    pub name: String,
    /// One entry per byte: `Some` must match exactly, `None` matches anything.
    pub pattern: Vec<Option<u8>>,
}

impl Signature {
    /// Parse the space-separated hex form (`48 8B ?? …`); `??` is a wildcard.
    pub fn parse(name: &str, text: &str) -> Option<Self> {
        let mut pattern = Vec::new();
        for token in text.split_whitespace() {
            if token == "??" || token == "**" {
                pattern.push(None);
            } else {
                pattern.push(Some(u8::from_str_radix(token, 16).ok()?));
            }
        }
        if pattern.len() < MIN_PATTERN_BYTES {
            return None;
        }
        Some(Self {
            name: name.to_owned(),
            pattern,
        })
    }

    /// The space-separated hex form this signature was parsed from.
    pub fn render(&self) -> String {
        self.pattern
            .iter()
            .map(|byte| match byte {
                Some(value) => format!("{value:02X}"),
                None => "??".to_owned(),
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Whether `bytes` starts with this pattern.
    pub fn matches(&self, bytes: &[u8]) -> bool {
        bytes.len() >= self.pattern.len()
            && self
                .pattern
                .iter()
                .zip(bytes)
                .all(|(expected, actual)| expected.is_none_or(|value| value == *actual))
    }
}

/// The target of an incremental-linking thunk: a function whose body is a single `jmp`.
///
/// A thunk has no bytes worth matching (its five bytes are `E9` and a wildcarded offset), so
/// both export and match look through it to the implementation. Naming the thunk is what makes
/// its call sites read like the source; the implementation carries the same name.
pub fn thunk_target(session: &mut Session, start: Address) -> Option<Address> {
    let insn = session.insn_at(start).ok().flatten()?;
    if insn.mnemonic() != recraft_decode::iced_x86::Mnemonic::Jmp {
        return None;
    }
    let target = insn.branch_target()?;
    (target.get() != start.get()).then_some(target)
}

/// The range a signature covers: the function itself, or the head of its target for a thunk.
fn signature_range(session: &mut Session, start: Address, end: Address) -> (Address, Address) {
    match thunk_target(session, start) {
        Some(target) => (
            target,
            Address::new(target.get().saturating_add(SIGNATURE_BYTES as u64)),
        ),
        None => (start, end),
    }
}

/// Build the signature of a function head, wildcarding the bytes that encode an address.
pub fn build(session: &mut Session, start: Address, end: Address, name: &str) -> Option<Signature> {
    use recraft_decode::iced_x86::OpKind;
    let (start, end) = signature_range(session, start, end);
    let available = end.get().saturating_sub(start.get()) as usize;
    let length = available.min(SIGNATURE_BYTES);
    if length < MIN_PATTERN_BYTES {
        return None;
    }
    let bytes = session.read_bytes(start, length).ok()?;
    let mut pattern: Vec<Option<u8>> = bytes.into_iter().map(Some).collect();
    let mut address = start;
    while (address.get() - start.get()) < length as u64 {
        let offset = (address.get() - start.get()) as usize;
        let Some(insn) = session.insn_at(address).ok().flatten() else {
            break;
        };
        let len = insn.len() as usize;
        if offset + len > length {
            break;
        }
        let mut tail = 0usize;
        if insn.branch_target().is_some() {
            // `rel8` is one byte, `rel32` four.
            tail = tail.max(if len <= 2 { 1 } else { 4 });
        }
        if insn.is_ip_rel_memory_operand() {
            tail = tail.max(4);
        }
        for index in 0..insn.op_count() {
            let size: usize = match insn.op_kind(index) {
                OpKind::Immediate8
                | OpKind::Immediate8to16
                | OpKind::Immediate8to32
                | OpKind::Immediate8to64 => 1,
                OpKind::Immediate16 => 2,
                OpKind::Immediate32 | OpKind::Immediate32to64 => 4,
                OpKind::Immediate64 => 8,
                _ => 0,
            };
            if size > 0 && insn.immediate64() >= ADDRESS_LIKE_IMMEDIATE {
                tail = tail.max(size);
            }
        }
        if tail > 0 {
            for byte in pattern.iter_mut().skip(offset + len - tail).take(tail) {
                *byte = None;
            }
        }
        address = insn.next_addr();
    }
    Some(Signature {
        name: name.to_owned(),
        pattern,
    })
}

/// The name of the first signature whose pattern matches this function head.
pub fn find_match<'a>(
    session: &mut Session,
    start: Address,
    end: Address,
    signatures: &'a [Signature],
) -> Option<&'a str> {
    let (start, end) = signature_range(session, start, end);
    let available = end.get().saturating_sub(start.get()) as usize;
    let length = available.min(SIGNATURE_BYTES);
    if length < MIN_PATTERN_BYTES {
        return None;
    }
    let bytes = session.read_bytes(start, length).ok()?;
    signatures
        .iter()
        .find(|signature| signature.matches(&bytes))
        .map(|signature| signature.name.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pattern_round_trips_through_text() {
        let signature = Signature::parse("printf", "48 89 5C ?? 57 48 83 EC 20 48 8B D9").unwrap();
        assert_eq!(signature.render(), "48 89 5C ?? 57 48 83 EC 20 48 8B D9");
        assert!(Signature::parse("short", "48 89").is_none());
        assert!(Signature::parse("bad", "ZZ 89 5C 24 08 57 48 83").is_none());
    }

    #[test]
    fn a_wildcard_matches_any_byte_and_a_fixed_byte_must_match() {
        let signature = Signature::parse("f", "48 89 5C ?? 57 48 83 EC 20").unwrap();
        assert!(signature.matches(&[0x48, 0x89, 0x5C, 0xFF, 0x57, 0x48, 0x83, 0xEC, 0x20]));
        assert!(!signature.matches(&[0x48, 0x89, 0x5D, 0xFF, 0x57, 0x48, 0x83, 0xEC, 0x20]));
        // A shorter function than the pattern never matches.
        assert!(!signature.matches(&[0x48, 0x89, 0x5C]));
    }
}
