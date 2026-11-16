// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! IDA-style decoration of the linear listing: symbol names and xref comments.
//!
//! Two things the disassembler text is expected to show, both of which need the project:
//!
//! - **Names instead of numbers.** `call sub_140001678`, `jnz short loc_140001483`,
//!   `mov byte ptr [byte_140023B40], 1`. IDA never draws a branch target or a global as a
//!   bare address; [`ListingSymbols`] supplies the same names (stored names first, then the
//!   auto names the analysis would have generated).
//! - **Cross-reference comments.** `; CODE XREF: sub_140001474+6↑j`,
//!   `; DATA XREF: .pdata:0000000140002500↑o` — [`attach_xref_comments`] fills
//!   [`DisasmLine::xrefs`] for the rows of one listing window.

use std::collections::HashMap;
use std::sync::Arc;

use recraft_common::{Address, Result};
use recraft_decode::{FlowKind, Formatter, Insn, Symbols};

use crate::segments::SegmentKind;
use crate::session::Session;
use crate::view::{DisasmLine, FunctionInfo, StringOverride, XrefEntry};

/// `xref.type` values. They mirror `recraft_analysis::XREF_*`; core does not depend on the
/// analysis crate, so the three values it needs are repeated here.
const XREF_CALL: i64 = 1;
const XREF_JUMP: i64 = 2;
pub(crate) const XREF_DATA: i64 = 3;

/// Encoding of a UTF-16LE literal in the `string` table. Also mirrors
/// `recraft_analysis::STRING_ENC_UTF16LE`, for the same reason as the xref kinds above.
const STRING_ENC_UTF16LE: i64 = 2;

/// What the listing knows about one stored string literal.
struct StringLiteral {
    /// IDA's auto name for the item (`aEnterFlag`, `asc_140018448`).
    name: String,
    /// The literal as IDA appends it to the instruction that points at it: `"Enter flag: "`.
    quoted: String,
}

/// Name of the data at `address`, sized like [`ListingSymbols::data`].
///
/// IDA derives a literal's auto name from its words: `a` + up to 14 CamelCased
/// alphanumerics. `"[-] Wrong flag. Try again."` becomes `aWrongFlagTryAg`, `" flag format:
/// flag{…}"` becomes `aFlagFormatFlag`. A literal without letters (the `"\r\n"` between two
/// strings) gets the address-based `asc_<hex>` instead.
pub fn data_name_for_literal(text: &str, addr: Address) -> String {
    const MAX: usize = 14;
    let mut name = String::from("a");
    let mut word_start = true;
    let mut letters = 0usize;
    for ch in text.chars() {
        if !ch.is_ascii_alphanumeric() {
            word_start = true;
            continue;
        }
        if letters == MAX {
            break;
        }
        if word_start {
            name.push(ch.to_ascii_uppercase());
            word_start = false;
        } else {
            name.push(ch);
        }
        letters += 1;
    }
    if letters == 0 {
        format!("asc_{:x}", addr.get())
    } else {
        name
    }
}

/// The literal stored at `bytes`: its text and encoding.
///
/// IDA treats a referenced printable run as a string item whether or not the Strings window
/// would list it (`aEnterFlag db 'Enter flag: ',0`, `asc_140018448 db 0Dh,0Ah,0`), so the
/// listing decodes the bytes a reference lands on instead of consulting the `string` table.
fn literal_text(bytes: &[u8]) -> Option<(String, i64)> {
    // Wide text (`H\0e\0l\0l\0o\0`) would otherwise decode as the one-character `"H"`.
    if let Some(text) = utf16_literal_text(bytes) {
        return Some((text, STRING_ENC_UTF16LE));
    }
    ascii_literal_text(bytes).map(|text| (text, 1))
}

fn printable_byte(byte: u8) -> bool {
    matches!(byte, b'\t' | b'\n' | b'\r') || (0x20..0x7f).contains(&byte)
}

/// Printable run at the start of `bytes`, as an 8-bit literal.
///
/// A short run needs its NUL terminator (`"\r\n"`, `"\n"`); a longer one is a real literal
/// even when it runs past the window — only the first `PREFIX` bytes are ever looked at.
fn ascii_literal_text(bytes: &[u8]) -> Option<String> {
    let end = bytes
        .iter()
        .position(|byte| !printable_byte(*byte))
        .unwrap_or(bytes.len());
    let text = &bytes[..end];
    let terminated = bytes.get(end) == Some(&0);
    if text.is_empty() || (text.len() < 4 && !terminated) {
        return None;
    }
    Some(String::from_utf8_lossy(text).into_owned())
}

/// `u"…"` literal: printable ASCII code units with a zero high byte.
fn utf16_literal_text(bytes: &[u8]) -> Option<String> {
    let mut units = Vec::new();
    for pair in bytes.chunks_exact(2) {
        let unit = u16::from_le_bytes([pair[0], pair[1]]);
        match u8::try_from(unit) {
            Ok(byte) if printable_byte(byte) => units.push(unit),
            _ => break,
        }
    }
    (units.len() >= 2).then(|| String::from_utf16_lossy(&units))
}

/// The literal as IDA quotes it on a referencing instruction: C-escaped, in double quotes,
/// cut at 39 cells with `...` after the closing quote (IDA's listing does exactly that for
/// long strings; `"====…"` from a 42-byte run of `=` ends `"…"` + `...`).
fn quoted_literal(text: &str, enc: i64) -> String {
    const MAX_CELLS: usize = 39;
    let mut escaped = String::with_capacity(text.len() + 2);
    for ch in text.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\r' => escaped.push_str("\\r"),
            '\n' => escaped.push_str("\\n"),
            '\t' => escaped.push_str("\\t"),
            ch if ch.is_control() => {
                let code = ch as u32;
                if code <= 0xff {
                    escaped.push_str(&format!("\\x{code:02X}"));
                } else {
                    escaped.push_str(&format!("\\u{{{code:x}}}"));
                }
            }
            ch => escaped.push(ch),
        }
    }
    let truncated = escaped.chars().count() > MAX_CELLS;
    if truncated {
        escaped = escaped.chars().take(MAX_CELLS).collect();
    }
    // IDA marks a wide literal `u"…"`, so a UTF-16 item is not mistaken for its low bytes.
    let prefix = if enc == STRING_ENC_UTF16LE { "u" } else { "" };
    let mut quoted = format!("{prefix}\"{escaped}\"");
    if truncated {
        quoted.push_str("...");
    }
    quoted
}

/// Names the listing substitutes for numeric addresses.
///
/// Built once per project revision and shared behind an [`Arc`]: every disassembly window
/// renders through it, and rebuilding the whole name/function table per window would put a
/// full-table read behind every scroll.
pub(crate) struct ListingSymbols {
    names: HashMap<u64, String>,
    /// Stored string literals by address: the name and the quoted text IDA shows.
    literals: HashMap<u64, StringLiteral>,
    /// Function starts, sorted (binary searched).
    functions: Vec<u64>,
    /// Every mapped `[start, end)` range.
    mapped: Vec<(u64, u64)>,
    /// Ranges a memory operand may point into without an explicit name.
    data: Vec<(u64, u64)>,
}

impl ListingSymbols {
    pub(crate) fn collect(session: &Session) -> Result<Self> {
        let mut names = HashMap::new();
        for (addr, name) in session.names()? {
            names.insert(addr.get(), name);
        }
        let mut functions: Vec<u64> = session
            .functions()?
            .into_iter()
            .map(|function| function.start.get())
            .collect();
        functions.sort_unstable();
        let mut mapped = Vec::new();
        let mut data = Vec::new();
        for segment in session.segments() {
            let range = (segment.range.start.get(), segment.range.end.get());
            mapped.push(range);
            // The file headers are not addressable data; a code segment is only reachable
            // through a branch, never through a memory operand.
            if !segment.perm.is_executable() && segment.kind != SegmentKind::Headers {
                data.push(range);
            }
        }
        // String literals are items in IDA's listing even though analysis stores only their
        // address and length: every one gets an auto name, and an instruction that points at
        // it carries the quoted text (`lea rcx, aEnterFlag; "Enter flag: "`, `lea rdx,
        // asc_140018448; "\r\n"`).
        //
        // Like IDA, the text is decoded from the bytes a reference lands on, not looked up
        // in the `string` table: an operand can only name what something points at, and
        // reading the whole table would make the first listing window of a large binary
        // noticeably slower (shell32 stores 124 k strings).
        let overrides: HashMap<u64, StringOverride> = session
            .string_overrides()?
            .into_iter()
            .map(|(addr, entry)| (addr.get(), entry))
            .collect();
        let targets: Vec<u64> = session
            .data_xref_targets()?
            .into_iter()
            .map(|target| target.get())
            .filter(|target| in_ranges(&data, *target))
            .collect();
        let mut literals = HashMap::with_capacity(targets.len());
        // One page of image bytes serves every target on it; a literal is at most `PREFIX`
        // bytes here, which is all an auto name and a quoted comment need.
        const WINDOW: u64 = 0x1_0000;
        const PREFIX: usize = 128;
        let mut window_start = u64::MAX;
        let mut window: Vec<u8> = Vec::new();
        let mut spill: Vec<u8> = Vec::new();
        for target in targets {
            let start = target & !(WINDOW - 1);
            if start != window_start {
                window_start = start;
                window.clear();
                window.resize(WINDOW as usize, 0);
                let read = session
                    .read_at(Address::new(start), &mut window)
                    .unwrap_or(0);
                window.truncate(read);
            }
            let offset = usize::try_from(target - start).unwrap_or(usize::MAX);
            let bytes: &[u8] = if offset.saturating_add(PREFIX) <= window.len() {
                &window[offset..offset + PREFIX]
            } else {
                // A literal that runs past the window (or ends the segment) is read on its
                // own; one item is not worth a second page.
                spill.clear();
                spill.resize(PREFIX, 0);
                let read = session
                    .read_at(Address::new(target), &mut spill)
                    .unwrap_or(0);
                spill.truncate(read);
                &spill
            };
            let (text, enc) = match overrides.get(&target).and_then(|edit| edit.text.clone()) {
                Some(text) => (text, 1),
                None => match literal_text(bytes) {
                    Some(found) => found,
                    None => continue,
                },
            };
            literals.insert(
                target,
                StringLiteral {
                    name: data_name_for_literal(&text, Address::new(target)),
                    quoted: quoted_literal(&text, enc),
                },
            );
        }
        Ok(Self {
            names,
            literals,
            functions,
            mapped,
            data,
        })
    }

    /// IDA's trailing literal comment for the item starting at `address`, quotes included
    /// (`"Enter flag: "`). An address inside a literal or between items has none.
    pub(crate) fn literal(&self, address: u64) -> Option<&str> {
        self.literals
            .get(&address)
            .map(|literal| literal.quoted.as_str())
    }

    fn is_function_start(&self, addr: u64) -> bool {
        self.functions.binary_search(&addr).is_ok()
    }
}

/// A shared symbol table usable by a [`Formatter`].
pub(crate) struct SharedSymbols(pub(crate) Arc<ListingSymbols>);

impl Symbols for SharedSymbols {
    fn code(&mut self, address: u64) -> Option<String> {
        self.0.code(address)
    }

    fn data(&mut self, address: u64, size: u32) -> Option<String> {
        self.0.data(address, size)
    }

    fn offset(&mut self, address: u64) -> Option<String> {
        self.0.offset(address)
    }
}

impl ListingSymbols {
    /// Name of the code at `address`: a stored name, else the auto name analysis would use.
    pub(crate) fn code(&self, address: u64) -> Option<String> {
        if let Some(name) = self.names.get(&address) {
            return Some(name.clone());
        }
        if !in_ranges(&self.mapped, address) {
            return None;
        }
        if self.is_function_start(address) {
            Some(format!("sub_{address:x}"))
        } else {
            Some(format!("loc_{address:x}"))
        }
    }

    /// Name of the data at `address`, sized like [`Self::data`].
    pub(crate) fn data(&self, address: u64, size: u32) -> Option<String> {
        if let Some(name) = self.names.get(&address) {
            return Some(name.clone());
        }
        if !in_ranges(&self.data, address) {
            return None;
        }
        // A literal is an item of its own whatever width the operand reads it with: IDA
        // shows `mov rax, qword ptr [aCaption]`, not a `qword_…` view of the string.
        if let Some(literal) = self.literals.get(&address) {
            return Some(literal.name.clone());
        }
        let prefix = match size {
            1 => "byte_",
            2 => "word_",
            4 => "dword_",
            8 => "qword_",
            16 => "xmmword_",
            32 => "ymmword_",
            // An operand that names an address without reading it (`lea rax, [addr]`) has no
            // width; IDA types the item as `unk_` when analysis has nothing better.
            0 => "unk_",
            _ => return None,
        };
        Some(format!("{prefix}{address:x}"))
    }

    /// Name of the item an immediate operand points at — only an existing name resolves.
    ///
    /// A plain number that happens to fall inside a data segment (`mov eax, 3000h` in a PIE
    /// image) must not become `offset unk_3000`, so this never synthesizes a name.
    pub(crate) fn offset(&self, address: u64) -> Option<String> {
        if let Some(name) = self.names.get(&address) {
            return Some(name.clone());
        }
        if !in_ranges(&self.data, address) {
            return None;
        }
        self.literals
            .get(&address)
            .map(|literal| literal.name.clone())
    }
}

fn in_ranges(ranges: &[(u64, u64)], address: u64) -> bool {
    ranges
        .iter()
        .any(|(start, end)| address >= *start && address < *end)
}

/// IDA's trailing literal comment for one instruction: the quoted text of the first operand
/// that points at a string literal (`"Enter flag: "`), if any.
pub(crate) fn literal_comment(symbols: &ListingSymbols, insn: &Insn) -> Option<String> {
    insn.data_operand_addresses()
        .into_iter()
        .find_map(|target| symbols.literal(target))
        .map(str::to_owned)
}

/// Fill `line.xrefs` with IDA's `CODE XREF:` / `DATA XREF:` comments.
///
/// Only the visible window is asked: one indexed query for the whole span, then the
/// referencing instructions and names it mentions.
pub(crate) fn attach_xref_comments(session: &Session, lines: &mut [DisasmLine]) -> Result<()> {
    let (Some(first), Some(last)) = (lines.first(), lines.last()) else {
        return Ok(());
    };
    let start = first.addr;
    let end = Address::new(last.addr.get() + u64::from(last.len.max(1)));
    let xrefs = session.xrefs_to_range(start, end)?;
    if xrefs.is_empty() {
        return Ok(());
    }
    let functions = session.functions()?;
    let mut sites: HashMap<u64, String> = HashMap::new();
    let mut by_target: HashMap<u64, Vec<String>> = HashMap::new();
    for xref in &xrefs {
        let target = xref.to.get();
        let site = match sites.get(&xref.from.get()) {
            Some(site) => site.clone(),
            None => {
                let site = site_label(session, &functions, *xref)?;
                sites.insert(xref.from.get(), site.clone());
                site
            }
        };
        let arrow = if xref.from < xref.to { '↑' } else { '↓' };
        let letter = xref_letter(session, *xref)?;
        let prefix = match xref.kind {
            XREF_DATA => "DATA XREF",
            _ => "CODE XREF",
        };
        by_target
            .entry(target)
            .or_default()
            .push(format!("{prefix}: {site}{arrow}{letter}"));
    }
    for line in lines.iter_mut() {
        if let Some(comments) = by_target.get(&line.addr.get()) {
            line.xrefs = comments.clone();
        }
    }
    Ok(())
}

/// How IDA names the referencing instruction: its symbol, `function+offset`,
/// `function:loc_<hex>` for a location that is itself a jump target, or `segment:offset`
/// for an address that is none of those.
fn site_label(session: &Session, functions: &[FunctionInfo], xref: XrefEntry) -> Result<String> {
    let from = xref.from;
    if let Some(name) = session.name_at(from)? {
        return Ok(name);
    }
    if let Some(function) = function_at(functions, from) {
        let name = session
            .name_at(function.start)?
            .unwrap_or_else(|| format!("sub_{:x}", function.start.get()));
        // IDA labels a jump target inside a function `loc_…` and writes the referencing
        // site as `main:loc_140001130`; an unlabeled address stays `main+120`.
        let labeled = session
            .xrefs_to(from)?
            .iter()
            .any(|xref| xref.kind == XREF_JUMP);
        if labeled && from != function.start {
            return Ok(format!("{name}:loc_{:x}", from.get()));
        }
        let offset = from.get().saturating_sub(function.start.get());
        return Ok(if offset == 0 {
            name
        } else {
            format!("{name}+{offset:x}")
        });
    }
    let segment = session
        .segment_containing(from)
        .map(|segment| segment.name.as_str())
        .unwrap_or("?");
    Ok(format!("{segment}:{:016x}", from.get()))
}

/// Single-letter reference type: IDA's `j` jump, `c` call, `o` address taken, `r` read,
/// `w` write.
fn xref_letter(session: &Session, xref: XrefEntry) -> Result<char> {
    if xref.kind != XREF_DATA {
        return Ok(match xref.kind {
            XREF_CALL => 'c',
            XREF_JUMP => 'j',
            _ => '?',
        });
    }
    let Some(insn) = session.insn_at(xref.from)? else {
        return Ok('r');
    };
    if insn.flow() != FlowKind::Next {
        return Ok('j');
    }
    let mut formatter = Formatter::new();
    let (mnemonic, operands) = formatter.format_parts(&insn);
    let mnemonic = mnemonic.to_ascii_lowercase();
    if mnemonic == "lea" {
        return Ok('o');
    }
    const STORES: [&str; 13] = [
        "mov", "movzx", "movsx", "movsxd", "movaps", "movups", "movdqa", "movdqu", "movss",
        "movsd", "movq", "xchg", "cmpxchg",
    ];
    let destination = operands.split(',').next().unwrap_or_default();
    Ok(
        if destination.contains('[') && STORES.contains(&mnemonic.as_str()) {
            'w'
        } else {
            'r'
        },
    )
}

fn function_at(functions: &[FunctionInfo], addr: Address) -> Option<&FunctionInfo> {
    let index = functions.partition_point(|function| function.start <= addr);
    let index = index.checked_sub(1)?;
    (addr < functions[index].end).then_some(&functions[index])
}
