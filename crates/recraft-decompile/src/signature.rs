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

/// The name a function already has, whether it was renamed or declared through a prototype.
///
/// The default `sub_<address>` name does not count: a signature may replace it.
pub fn user_name(session: &Session, addr: Address) -> recraft_common::Result<Option<String>> {
    if let Some(name) = session.name_at(addr)?
        && name != format!("sub_{:x}", addr.get())
    {
        return Ok(Some(name));
    }
    // A prototype carries the name the user declared: `int printf(const char* format, ...)`.
    let Some(text) = session.prototype_at(addr)? else {
        return Ok(None);
    };
    let Some(open) = text.find('(') else {
        return Ok(None);
    };
    Ok(text[..open].split_whitespace().last().map(str::to_owned))
}

/// Parse a signature document: `{"format": 1, "signatures": [{"name": …, "pattern": …}]}`.
pub fn parse_document(text: &str) -> Result<Vec<Signature>, String> {
    let document: serde_json::Value =
        serde_json::from_str(text).map_err(|error| error.to_string())?;
    let Some(entries) = document
        .get("signatures")
        .and_then(|value| value.as_array())
    else {
        return Err("no `signatures` array".to_owned());
    };
    let mut signatures = Vec::with_capacity(entries.len());
    for entry in entries {
        let (Some(name), Some(pattern)) = (
            entry.get("name").and_then(|value| value.as_str()),
            entry.get("pattern").and_then(|value| value.as_str()),
        ) else {
            continue;
        };
        if let Some(signature) = Signature::parse(name, pattern) {
            signatures.push(signature);
        }
    }
    Ok(signatures)
}

/// What [`apply`] did.
#[derive(Debug, Default)]
pub struct ApplyReport {
    /// Addresses that were renamed, with the name they got.
    pub renamed: Vec<(Address, String)>,
    /// Matches skipped because the project already uses the name.
    pub skipped: usize,
}

/// Name every unnamed function that matches one of `signatures`.
///
/// A thunk and its target carry the same signature; the thunk is what call sites name, so it
/// takes the name and the target is left alone (names are unique per project). A name the
/// project already uses is never reused, and with `dry_run` nothing is written.
pub fn apply(
    session: &mut Session,
    signatures: &[Signature],
    dry_run: bool,
) -> recraft_common::Result<ApplyReport> {
    use recraft_core::Command;

    let functions = session.functions()?;
    let mut matches: Vec<(Address, String)> = Vec::new();
    for function in &functions {
        if user_name(session, function.start)?.is_some() {
            continue;
        }
        if let Some(name) = find_match(session, function.start, function.end, signatures) {
            matches.push((function.start, name.to_owned()));
        }
    }
    matches.sort_by_key(|(addr, _)| thunk_target(session, *addr).is_none());
    let mut taken: std::collections::BTreeSet<String> = functions
        .iter()
        .filter_map(|function| user_name(session, function.start).ok().flatten())
        .collect();
    let mut report = ApplyReport::default();
    for (addr, name) in matches {
        if taken.contains(&name) {
            report.skipped += 1;
            continue;
        }
        taken.insert(name.clone());
        if !dry_run {
            session.execute(Command::Rename {
                addr,
                name: Some(name.clone()),
            })?;
        }
        report.renamed.push((addr, name));
    }
    Ok(report)
}

/// [`apply`] with the signatures compiled into the binary.
pub fn apply_builtin(session: &mut Session, dry_run: bool) -> recraft_common::Result<ApplyReport> {
    let signatures = parse_document(BUILTIN_SIGNATURES).unwrap_or_default();
    apply(session, &signatures, dry_run)
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
