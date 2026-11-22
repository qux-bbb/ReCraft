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

use crate::flags::{CodeData, Flags};
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

// ---------------------------------------------------------------------------------------
// The listing walk
// ---------------------------------------------------------------------------------------

/// One stored string as the listing walk needs it: where it starts, the text length, the
/// encoding, and the text the project shows for it (decoded bytes, or the user's edit).
///
/// A front end passes these in because it may display a replacement text the project does
/// not store ([`StringOverride`]); [`Session::strings`] has the raw rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingString {
    pub addr: Address,
    /// Text length in bytes, terminator excluded.
    pub len: u32,
    pub enc: i64,
    pub text: String,
}

/// What the listing walk needs but cannot read from the project on its own.
#[derive(Debug, Clone, Copy, Default)]
pub struct ListingHints<'a> {
    /// Strings the caller already resolved (text and user overrides included).
    pub strings: &'a [ListingString],
    /// Short literals the decompiler saw: data items below the extractor's minimum length.
    pub literals: &'a [(Address, String)],
}

impl DisasmLine {
    /// IDA's character annotation for a one-byte `db` item: `Some('=')` for `db  3Dh ; =`.
    ///
    /// A space is reported so the caller can quote it (the bare space would be invisible);
    /// control bytes get none, and a defined literal (`db 'a',0`) already is its own text.
    pub fn byte_char(&self) -> Option<char> {
        if self.mnemonic != "db" || self.len != 1 || self.operands.starts_with('\'') {
            return None;
        }
        match self.raw.first().copied() {
            Some(byte) if byte.is_ascii_graphic() || byte == b' ' => Some(byte as char),
            _ => None,
        }
    }

    /// IDA's type comment for a named item (`; const char aXxx[]`), for the character arrays
    /// the listing draws: a stored literal, or a named one-byte `db` item. IDA prints it on
    /// the line above the item.
    pub fn type_comment(&self) -> Option<String> {
        if !matches!(self.flags.kind, CodeData::Data | CodeData::Undefined) {
            return None;
        }
        let name = self.name.as_deref()?;
        // The address-named `asc_…` runs stay one line.
        if name.starts_with("asc_") {
            return None;
        }
        if self.operands.starts_with('\'') {
            let ty = if self.mnemonic == "dw" {
                "const wchar_t"
            } else {
                "const char"
            };
            Some(format!("; {ty} {name}[]"))
        } else if self.mnemonic == "db" && self.len == 1 {
            Some(format!("; const char {name}[]"))
        } else {
            None
        }
    }
}

impl Session {
    /// Rows of IDA's listing window, starting at `start`.
    ///
    /// The walk follows the address space instead of one kind of row: instructions in
    /// executable segments, stored literals as `db '…',0`, `align` rows for zero padding,
    /// typed items where a cross reference reads a 2/4/8/16-byte value, and one raw `db` row
    /// per remaining byte. A byte the user classified (`C`/`D`/`U`) wins over every derived
    /// source, the way IDA's item list does.
    pub fn listing_rows(
        &mut self,
        start: Address,
        count: usize,
        hints: ListingHints<'_>,
    ) -> Result<Vec<DisasmLine>> {
        Ok(self
            .listing_rows_inner(start, count, hints)
            .unwrap_or_default())
    }

    /// Next row boundary after `addr` — the address the listing draws next.
    pub fn next_listing_row(
        &mut self,
        addr: Address,
        hints: ListingHints<'_>,
    ) -> Result<Option<Address>> {
        Ok(self.next_row_start(addr, hints))
    }

    /// Step the listing by `rows` rows of its address walk, returning the new anchor.
    ///
    /// Forward steps follow the row walk from `start`. Backward steps walk the row boundaries
    /// from the nearest function start (or section start), so they land on the rows the view
    /// renders instead of on instruction boundaries inside a literal's padding, and they keep
    /// going into the previous section.
    pub fn step_listing(
        &mut self,
        start: Address,
        rows: i32,
        hints: ListingHints<'_>,
    ) -> Result<Option<Address>> {
        Ok(self.step_listing_inner(start, rows, hints))
    }

    /// Effective item classification at `addr`: the user's `C`/`D`/`U` decision when one is
    /// stored, otherwise [`CodeData::Unknown`], which lets the derived sources decide.
    fn item_kind(&mut self, addr: Address) -> CodeData {
        self.flags_at(addr)
            .map(|flags| flags.kind)
            .unwrap_or_default()
    }

    /// IDA's name for an anonymous data byte: `unk_<addr>` while the byte is undefined,
    /// `byte_<addr>` once the user defined it as data. Only bytes that something references
    /// get one — that is what makes the listing read `unk_1400183C0 db  3Dh ; =` instead of a
    /// bare `db`, and it is the same name the operand and the xref text already print.
    fn data_byte_name(&self, addr: Address, prefix: &str) -> Option<String> {
        if let Some(name) = self.name_at(addr).ok().flatten() {
            return Some(name);
        }
        let referenced = self
            .xrefs_to(addr)
            .ok()
            .is_some_and(|xrefs| !xrefs.is_empty());
        referenced.then(|| format!("{prefix}{:x}", addr.get()))
    }

    /// Data row for the stored string at `index`, starting at `addr`.
    ///
    /// `addr` is usually the literal's own start; when a literal points inside a longer run
    /// the row shows the tail from the requested address, like IDA's data view, not the run's
    /// own (earlier) start.
    fn string_data_row(
        &self,
        hints: ListingHints<'_>,
        index: usize,
        addr: Address,
    ) -> Option<DisasmLine> {
        let mut row = hints.strings.get(index)?.clone();
        let offset = addr.get().checked_sub(row.addr.get())?;
        let mut raw = [0u8; 15];
        if offset > 0 {
            let text = if row.enc == STRING_ENC_UTF16LE {
                if offset % 2 != 0 {
                    return None;
                }
                let skip = usize::try_from(offset / 2).ok()?;
                row.text.chars().skip(skip).collect()
            } else {
                row.text.get(usize::try_from(offset).ok()?..)?.to_owned()
            };
            row.addr = addr;
            row.len = row.len.saturating_sub(offset as u32);
            row.text = text;
            self.read_at(row.addr, &mut raw).unwrap_or(0);
        } else {
            let read = self.read_at(row.addr, &mut raw).unwrap_or(0);
            if read == 0 {
                return None;
            }
        }
        let escaped = row.text.replace('\\', "\\\\").replace('\'', "\\'");
        let (mnemonic, operands) = if row.enc == STRING_ENC_UTF16LE {
            ("dw", format!("'{escaped}',0"))
        } else {
            ("db", format!("'{escaped}',0"))
        };
        Some(DisasmLine {
            addr: row.addr,
            len: row.len.min(254) as u8,
            raw,
            mnemonic: mnemonic.to_owned(),
            operands,
            name: self
                .name_at(row.addr)
                .ok()
                .flatten()
                .or_else(|| Some(data_name_for_literal(&row.text, row.addr))),
            comment: None,
            literal: None,
            segment: self
                .segment_containing(row.addr)
                .map(|segment| segment.name.clone())
                .unwrap_or_default(),
            xrefs: Vec::new(),
            flags: Flags::data(),
        })
    }

    /// Quoted literal cached for `addr`, with the bytes the decompiler's escapes name.
    fn literal_data_bytes<'a>(
        &self,
        hints: ListingHints<'a>,
        addr: Address,
    ) -> Option<(&'a str, Vec<u8>)> {
        let (_, literal) = hints
            .literals
            .iter()
            .rev()
            .find(|(known, _)| *known == addr)?;
        let inner = literal.strip_prefix('"')?.strip_suffix('"')?;
        let bytes = c_literal_bytes(inner)?;
        (!bytes.is_empty()).then_some((literal.as_str(), bytes))
    }

    /// Data row for a short literal reached from the decompile view that no stored string
    /// covers. Such literals (`"\r\n"`, `")"`) fall below the extraction minimum but still
    /// name data, so their quoted text is enough to show the row.
    fn literal_data_row(&self, hints: ListingHints<'_>, addr: Address) -> Option<DisasmLine> {
        let (literal, bytes) = self.literal_data_bytes(hints, addr)?;
        let mut raw = [0u8; 15];
        let copied = bytes.len().min(15);
        raw[..copied].copy_from_slice(&bytes[..copied]);
        Some(DisasmLine {
            addr,
            len: (bytes.len() + 1).min(254) as u8,
            raw,
            mnemonic: "db".to_owned(),
            operands: format!("{literal},0"),
            name: self.name_at(addr).ok().flatten(),
            comment: None,
            literal: None,
            segment: self
                .segment_containing(addr)
                .map(|segment| segment.name.clone())
                .unwrap_or_default(),
            xrefs: Vec::new(),
            flags: Flags::data(),
        })
    }

    /// Length (terminator included) of the *referenced* short literal at `addr`.
    ///
    /// IDA makes an item of a short literal only when something points at it; the same bytes
    /// in unreferenced padding stay a raw byte row. The check keeps the listing's walk and
    /// the rows it renders in agreement.
    fn short_literal_len_at(&self, addr: Address) -> Option<usize> {
        let mut raw = [0u8; 8];
        let read = self.read_at(addr, &mut raw).ok()?;
        let len = short_literal_len(&raw[..read])?;
        if self.xrefs_to(addr).ok()?.is_empty() {
            return None;
        }
        Some(len)
    }

    /// End of the alignment padding that starts at `addr`.
    ///
    /// IDA draws a whole run of zero padding as one `align 4/8/10h` line; a zero *item* (a
    /// null table entry) keeps its `db 0` rows. The two are told apart by what follows the
    /// run: padding ends right where a named item (literal, short literal or named address)
    /// begins on an aligned boundary.
    fn padding_run(&self, hints: ListingHints<'_>, addr: Address) -> Option<Address> {
        let segment_end = self.segment_containing(addr)?.range.end;
        let mut raw = [0u8; 16];
        let read = self.read_at(addr, &mut raw).ok()?;
        if read == 0 || raw[0] != 0 {
            return None;
        }
        let run = raw[..read].iter().take_while(|byte| **byte == 0).count();
        let next = Address::new(addr.get() + run as u64);
        // Gaps shorter than three bytes are not the padding IDA draws (`db 0` rows there);
        // the boundary must let a 4/8/16-byte item start.
        if run < 3 || next >= segment_end || next.get() % 4 != 0 {
            return None;
        }
        let item_here = listing_string_at(hints.strings, next).is_some()
            || self.literal_data_bytes(hints, next).is_some()
            || self.short_literal_len_at(next).is_some()
            || self.wide_data_access(next).is_some()
            || self.name_at(next).ok().flatten().is_some();
        item_here.then_some(next)
    }

    /// Width of the widest direct access a cross reference makes to `addr`.
    ///
    /// IDA types a data item by how it is used: a 16-byte read becomes
    /// `xmmword_1400184C0 xmmword 0FFFF…h`, an 8-byte read a `dq`, a 4-byte read a `dd`.
    /// Returns `None` for `addr` that nothing reads as a 2/4/8/16-byte value.
    fn wide_data_access(&self, addr: Address) -> Option<usize> {
        let mut width = None;
        for xref in self.xrefs_to(addr).ok()? {
            let Some(insn) = self.insn_at(xref.from).ok()? else {
                continue;
            };
            // Direct operands name the address themselves; an indexed operand
            // (`[table + index*scale]`) reaches the item through its table base.
            let references = if insn.memory_is_direct() {
                Address::new(insn.memory_displacement64()) == addr
            } else if let Some((table, _)) = insn.memory_indexed_table() {
                Address::new(table) == addr
            } else {
                false
            };
            if !references {
                continue;
            }
            let size = insn.memory_size().size();
            if matches!(size, 2 | 4 | 8 | 16) && width.is_none_or(|current| size > current) {
                width = Some(size);
            }
        }
        width
    }

    /// Raw `db` row covering the bytes at `addr` up to the next row boundary.
    fn raw_data_row(&mut self, hints: ListingHints<'_>, addr: Address) -> Option<DisasmLine> {
        let literal_len = self.short_literal_len_at(addr);
        let wide = self.wide_data_access(addr);
        let end = self.segment_containing(addr)?.range.end;
        let next = self.next_row_start(addr, hints)?.min(end);
        let len = next
            .get()
            .saturating_sub(addr.get())
            .min(u64::from(u8::MAX)) as u8;
        // Sixteen bytes: an `xmmword` item needs the whole row; `raw` keeps the first 15.
        let mut bytes = [0u8; 16];
        let read = self.read_at(addr, &mut bytes).unwrap_or(0);
        let mut raw = [0u8; 15];
        let copied = read.min(15);
        raw[..copied].copy_from_slice(&bytes[..copied]);
        let shown = read.min(usize::from(len));
        // A referenced short literal (`"\r\n"`) is an item of its own; a wide read types the
        // item (`xmmword_…`, `dq …`); a multi-byte run of zero padding becomes IDA's `align`
        // line; everything else is one byte per row.
        let (mnemonic, operands, name) = if let Some(len) = literal_len.filter(|len| *len <= shown)
        {
            let text: String = bytes[..len - 1].iter().map(|byte| *byte as char).collect();
            (
                "db".to_owned(),
                format_db_bytes(&bytes[..len - 1]),
                Some(data_name_for_literal(&text, addr)),
            )
        } else if let Some(size) = wide.filter(|size| *size <= shown) {
            let (_, mut prefix) = typed_item_kind(size);
            let value_bytes = &bytes[..size];
            let operands = match (size, data_offset_target(value_bytes)) {
                (4 | 8, Some(target)) if self.segment_containing(target).is_some() => {
                    match self.name_at(target).ok().flatten() {
                        Some(name) => {
                            // IDA names an item holding an address `off_<hex>`.
                            prefix = "off_";
                            format!("offset {name}")
                        }
                        None => hex_value(value_bytes),
                    }
                }
                _ => hex_value(value_bytes),
            };
            let name = self
                .name_at(addr)
                .ok()
                .flatten()
                .or_else(|| Some(format!("{prefix}{:x}", addr.get())));
            (typed_item_kind(size).0.to_owned(), operands, name)
        } else if len > 1 {
            (
                "align".to_owned(),
                align_operand(next.get().saturating_sub(addr.get())),
                None,
            )
        } else {
            (
                "db".to_owned(),
                db_byte_operand(bytes.first().copied().unwrap_or(0)),
                self.data_byte_name(addr, "unk_"),
            )
        };
        Some(DisasmLine {
            addr,
            len,
            raw,
            mnemonic,
            operands,
            name,
            comment: None,
            literal: None,
            segment: self
                .segment_containing(addr)
                .map(|segment| segment.name.clone())
                .unwrap_or_default(),
            xrefs: Vec::new(),
            flags: Flags::data(),
        })
    }

    /// One-byte row for a byte the user classified, with no derived grouping: IDA leaves
    /// undefined bytes as one-byte items, and `D` turns an item into plain bytes instead of
    /// keeping the literal or typed item analysis derived from xrefs.
    fn classified_byte_row(&self, addr: Address, flags: Flags) -> Option<DisasmLine> {
        let mut raw = [0u8; 15];
        let read = self.read_at(addr, &mut raw).ok()?;
        if read == 0 {
            return None;
        }
        let prefix = if flags.is_undefined() {
            "unk_"
        } else {
            "byte_"
        };
        Some(DisasmLine {
            addr,
            len: 1,
            raw,
            mnemonic: "db".to_owned(),
            operands: db_byte_operand(raw[0]),
            name: self.data_byte_name(addr, prefix),
            comment: None,
            literal: None,
            segment: self
                .segment_containing(addr)
                .map(|segment| segment.name.clone())
                .unwrap_or_default(),
            xrefs: Vec::new(),
            flags,
        })
    }

    fn next_row_start(&mut self, addr: Address, hints: ListingHints<'_>) -> Option<Address> {
        let segment = self.segment_containing(addr)?.clone();
        let end = segment.range.end;
        if addr >= end {
            return self.next_segment_start(addr);
        }
        let kind = self.item_kind(addr);
        let next = if kind == CodeData::Undefined {
            // Undefined bytes are one-byte items: nothing derived gives them a length.
            addr.get() + 1
        } else if kind == CodeData::Code {
            // `C` forces a decode even outside an executable section; a byte that does not
            // decode still gets its own row, so the walk never stalls on it.
            self.insn_at(addr)
                .ok()?
                .map_or(addr.get() + 1, |insn| insn.next_addr().get())
        } else if let Some((index, _)) = listing_string_at(hints.strings, addr) {
            let row = hints.strings.get(index)?;
            row.addr.get() + u64::from(row.len) + string_terminator_len(row.enc)
        } else if kind == CodeData::Data {
            // `D` made the byte plain: one row, with no xref-derived typing.
            addr.get() + 1
        } else if let Some((_, bytes)) = self.literal_data_bytes(hints, addr) {
            addr.get() + bytes.len() as u64 + 1
        } else if segment.perm.is_executable() {
            self.insn_at(addr).ok()??.next_addr().get()
        } else if let Some(len) = self.short_literal_len_at(addr) {
            // A referenced short literal (`"\r\n"`) is its own item; the bytes after its
            // terminator are padding, not part of the same raw row.
            addr.get() + len as u64
        } else if let Some(size) = self.wide_data_access(addr) {
            // A wide read types the data item: it covers `size` bytes as one row.
            addr.get() + size as u64
        } else if let Some(padding) = self.padding_run(hints, addr) {
            padding.get()
        } else {
            // Undefined data is one byte per row, as in IDA's listing.
            addr.get() + 1
        };
        if next >= end.get() {
            return self.next_segment_start(addr);
        }
        (next > addr.get()).then(|| Address::new(next))
    }

    /// First mapped address after `addr` (crossing the gap between two segments).
    fn next_segment_start(&self, addr: Address) -> Option<Address> {
        self.segments()
            .iter()
            .map(|segment| segment.range.start)
            .filter(|start| *start > addr)
            .min()
    }

    fn listing_rows_inner(
        &mut self,
        start: Address,
        count: usize,
        hints: ListingHints<'_>,
    ) -> Option<Vec<DisasmLine>> {
        let mut rows = Vec::with_capacity(count.min(4096));
        let mut addr = start;
        while rows.len() < count {
            let Some(segment) = self.segment_containing(addr).cloned() else {
                // Between two segments: continue at the next mapped address, or stop. An
                // unmapped start still shows nothing.
                if rows.is_empty() {
                    break;
                }
                match self.next_segment_start(addr) {
                    Some(next) => addr = next,
                    None => break,
                }
                continue;
            };
            let end = segment.range.end;
            let kind = self.item_kind(addr);
            match kind {
                CodeData::Undefined => {
                    // Nothing derived may claim an undefined byte: not the string table,
                    // not the literal cache, not the instruction sweep.
                    let Some(row) = self.classified_byte_row(addr, Flags::undefined()) else {
                        break;
                    };
                    rows.push(row);
                }
                CodeData::Data => {
                    // `D` replaced the item with plain bytes. A stored string survives only
                    // when `A` marked it (which keeps the item), because the key that
                    // replaced the item also removed it from the string table.
                    if let Some((index, _)) = listing_string_at(hints.strings, addr) {
                        let Some(row) = self.string_data_row(hints, index, addr) else {
                            break;
                        };
                        rows.push(row);
                    } else {
                        let Some(row) = self.classified_byte_row(addr, Flags::data()) else {
                            break;
                        };
                        rows.push(row);
                    }
                }
                CodeData::Code => {
                    // One item at a time, so a forced-code run stops at the next byte the
                    // user marked data or undefined instead of sweeping over it.
                    let decoded = match self.disasm(addr, 1) {
                        Ok(mut lines) => lines.pop().filter(|line| line.addr < end),
                        Err(_) => None,
                    };
                    match decoded {
                        Some(line) => {
                            addr = Address::new(line.addr.get() + u64::from(line.len).max(1));
                            rows.push(line);
                        }
                        // A byte the user forced to code that does not decode keeps its own
                        // row, so one bad byte cannot blank the rest of the listing.
                        None => {
                            let Some(row) = self.classified_byte_row(addr, Flags::code()) else {
                                break;
                            };
                            addr = Address::new(addr.get() + 1);
                            rows.push(row);
                        }
                    }
                    continue;
                }
                CodeData::Unknown => {
                    if let Some((index, _)) = listing_string_at(hints.strings, addr) {
                        let Some(row) = self.string_data_row(hints, index, addr) else {
                            break;
                        };
                        rows.push(row);
                    } else if let Some(row) = self.literal_data_row(hints, addr) {
                        rows.push(row);
                    } else if segment.perm.is_executable() {
                        let remaining = count - rows.len();
                        let Ok(mut lines) = self.disasm(addr, remaining) else {
                            break;
                        };
                        lines.retain(|line| line.addr < end);
                        let Some(last) = lines.last() else { break };
                        addr = Address::new(last.addr.get() + u64::from(last.len));
                        rows.append(&mut lines);
                        continue;
                    } else {
                        let Some(row) = self.raw_data_row(hints, addr) else {
                            break;
                        };
                        rows.push(row);
                    }
                }
            }
            match self.next_row_start(addr, hints) {
                Some(next) => addr = next,
                None => break,
            }
        }
        (!rows.is_empty()).then_some(rows)
    }

    fn step_listing_inner(
        &mut self,
        start: Address,
        rows: i32,
        hints: ListingHints<'_>,
    ) -> Option<Address> {
        if rows == 0 {
            return None;
        }
        if rows > 0 {
            let mut addr = start;
            let mut moved = false;
            for _ in 0..rows {
                match self.next_row_start(addr, hints) {
                    Some(next) if next > addr => {
                        addr = next;
                        moved = true;
                    }
                    _ => break,
                }
            }
            return moved.then_some(addr);
        }
        let mut remaining = rows.unsigned_abs() as usize;
        let mut cursor = start;
        let mut result = None;
        // Each pass covers one anchor..cursor span; crossing a section needs the next
        // anchor, hence the (bounded) loop.
        for _ in 0..64 {
            let Some(anchor) = self.backward_anchor(cursor) else {
                break;
            };
            if anchor >= cursor {
                break;
            }
            let mut starts: std::collections::VecDeque<Address> =
                std::collections::VecDeque::with_capacity(remaining.min(64));
            let mut probe = anchor;
            let mut steps = 0usize;
            while probe < cursor && steps < DATA_WALK_LIMIT {
                starts.push_back(probe);
                if starts.len() > remaining {
                    starts.pop_front();
                }
                match self.next_row_start(probe, hints) {
                    Some(next) if next > probe => probe = next,
                    _ => break,
                }
                steps += 1;
            }
            let Some(first) = starts.front().copied() else {
                break;
            };
            remaining = remaining.saturating_sub(starts.len());
            result = Some(first);
            cursor = first;
            if remaining == 0 {
                break;
            }
        }
        result
    }

    /// Where a backward row walk starts: the nearest function start (or the section start)
    /// before `addr`, or — when `addr` already sits on it — the last function of the
    /// preceding section, so scrolling up can cross into earlier code the way IDA does.
    fn backward_anchor(&self, addr: Address) -> Option<Address> {
        let segments = self.segments();
        let index = segments
            .iter()
            .position(|segment| addr >= segment.range.start && addr < segment.range.end)?;
        let current = segments.get(index)?;
        if addr > current.range.start {
            if !current.perm.is_executable() {
                return Some(current.range.start);
            }
            let functions = self.functions().ok()?;
            return functions
                .iter()
                .filter(|function| function.start >= current.range.start)
                .filter(|function| function.start < addr)
                .map(|function| function.start)
                .max()
                .or(Some(current.range.start));
        }
        let previous = segments.get(index.checked_sub(1)?)?;
        if !previous.perm.is_executable() {
            return Some(previous.range.start);
        }
        let functions = self.functions().ok()?;
        functions
            .iter()
            .filter(|function| {
                function.start >= previous.range.start && function.start < previous.range.end
            })
            .map(|function| function.start)
            .max()
            .or(Some(previous.range.start))
    }
}

/// Upper bound on the rows a backward walk probes in one section.
const DATA_WALK_LIMIT: usize = 200_000;

/// Index of the stored string covering `addr`, plus the byte offset into its text.
///
/// A literal does not have to start a stored string: compiler string pooling makes `"===="`
/// a suffix of an `"O====…"` run, so an address inside a run still names a string. The rows
/// are in address order and do not overlap, so the only candidate is the last literal that
/// starts at or before `addr`; a binary search keeps the walk cheap.
fn listing_string_at(strings: &[ListingString], addr: Address) -> Option<(usize, u32)> {
    let after = strings.partition_point(|row| row.addr <= addr);
    let index = after.checked_sub(1)?;
    let row = strings.get(index)?;
    let offset = addr.get().checked_sub(row.addr.get())?;
    (offset < u64::from(row.len)).then_some((index, offset as u32))
}

/// Bytes a literal occupies beyond its text: UTF-16LE literals end in two NUL bytes.
pub fn string_terminator_len(enc: i64) -> u64 {
    if enc == STRING_ENC_UTF16LE { 2 } else { 1 }
}

/// Length of a short NUL-terminated literal at the start of `bytes`, terminator included.
///
/// Below the Strings window's minimum length such runs never reach the `string` table, but
/// IDA still shows the referenced ones as items (`asc_140018448 db 0Dh,0Ah,0`) instead of
/// folding them into a raw byte row.
pub fn short_literal_len(bytes: &[u8]) -> Option<usize> {
    let end = bytes.iter().take(8).position(|byte| *byte == 0)?;
    if end == 0 {
        return None;
    }
    let text = &bytes[..end];
    let printable = text
        .iter()
        .all(|byte| matches!(*byte, b'\t' | b'\n' | b'\r') || (0x20..0x7f).contains(byte));
    printable.then_some(end + 1)
}

/// Bytes named by the inner text of a C string literal rendered by the decompiler.
///
/// `quote_c_string` escapes quotes, backslashes, control characters and `\xNN`; reversing
/// those escapes gives the bytes a short literal points at, even when the string table does
/// not cover the address.
pub fn c_literal_bytes(text: &str) -> Option<Vec<u8>> {
    let mut bytes = Vec::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            let mut buffer = [0u8; 4];
            bytes.extend_from_slice(ch.encode_utf8(&mut buffer).as_bytes());
            continue;
        }
        match chars.next()? {
            'n' => bytes.push(b'\n'),
            'r' => bytes.push(b'\r'),
            't' => bytes.push(b'\t'),
            '\\' => bytes.push(b'\\'),
            '"' => bytes.push(b'"'),
            'x' => {
                let high = chars.next()?.to_digit(16)?;
                let low = chars.next()?.to_digit(16)?;
                bytes.push((high * 16 + low) as u8);
            }
            other => {
                // The decompiler emits no other escapes; keep an unknown one verbatim.
                bytes.push(b'\\');
                let mut buffer = [0u8; 4];
                bytes.extend_from_slice(other.encode_utf8(&mut buffer).as_bytes());
            }
        }
    }
    Some(bytes)
}

/// `align` operand for a padding run: the number of bytes the run covers (`align 3`,
/// `align 5`), which says more than IDA's power-of-two form (`align 8`, `align 10h`).
pub fn align_operand(len: u64) -> String {
    len.to_string()
}

/// Two-digit hex with IDA's `h` suffix, plus the leading zero IDA adds when the first digit
/// is a letter (`0FFh`, `0Ah`).
fn hex_byte(byte: u8) -> String {
    if byte < 0xA0 {
        format!("{byte:02X}h")
    } else {
        format!("0{byte:02X}h")
    }
}

/// IDA's directive and auto-name prefix for a data item read as `size` bytes.
pub fn typed_item_kind(size: usize) -> (&'static str, &'static str) {
    match size {
        16 => ("xmmword", "xmmword_"),
        8 => ("dq", "qword_"),
        4 => ("dd", "dword_"),
        2 => ("dw", "word_"),
        _ => ("db", "byte_"),
    }
}

/// Address a 4/8-byte item holds (little-endian), when the value could name one.
pub fn data_offset_target(bytes: &[u8]) -> Option<Address> {
    let mut value = 0u64;
    for (index, byte) in bytes.iter().enumerate().take(8) {
        value |= u64::from(*byte) << (index * 8);
    }
    (value != 0).then(|| Address::new(value))
}

/// Big-endian hex text of a little-endian value, IDA style: uppercase, `h` suffix, and a
/// leading zero when the first digit is a letter (`0FFFFFFFFFFFFFFFFh`).
pub fn hex_value(bytes: &[u8]) -> String {
    if bytes.iter().all(|byte| *byte == 0) {
        return "0".to_owned();
    }
    let mut digits = String::with_capacity(bytes.len() * 2);
    for byte in bytes.iter().rev() {
        digits.push_str(&format!("{byte:02X}"));
    }
    let mut text = digits.trim_start_matches('0').to_owned();
    if text.is_empty() {
        text.push('0');
    }
    if text
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_alphabetic())
    {
        text.insert(0, '0');
    }
    text.push('h');
    text
}

/// One undefined data byte the way IDA prints it: `0` or `0FFh`, right-aligned in the
/// four-cell operand field (`db    0`, `db  54h`, `db 0FFh`).
pub fn db_byte_operand(byte: u8) -> String {
    let text = if byte == 0 {
        "0".to_owned()
    } else {
        hex_byte(byte)
    };
    format!("{text:>4}")
}

/// `db …` operand text for a literal's bytes: always one escaped C string plus its NUL —
/// `'Enter flag: ',0`, `'\r\n',0`, `'caf\xC3\xA9',0`.
///
/// IDA falls back to a `0Dh,0Ah,0` byte list when the text is not printable; the escaped
/// literal keeps the string readable and one line.
pub fn format_db_bytes(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len());
    for byte in bytes {
        match *byte {
            b'\r' => text.push_str("\\r"),
            b'\n' => text.push_str("\\n"),
            b'\t' => text.push_str("\\t"),
            b'\\' => text.push_str("\\\\"),
            b'\'' => text.push_str("\\'"),
            byte @ 0x20..=0x7e => text.push(byte as char),
            byte => text.push_str(&format!("\\x{byte:02X}")),
        }
    }
    format!("'{text}',0")
}
