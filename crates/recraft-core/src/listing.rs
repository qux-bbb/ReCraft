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
use recraft_decode::{FlowKind, Formatter, Symbols};

use crate::segments::SegmentKind;
use crate::session::Session;
use crate::view::{DisasmLine, FunctionInfo, XrefEntry};

/// `xref.type` values. They mirror `recraft_analysis::XREF_*`; core does not depend on the
/// analysis crate, so the three values it needs are repeated here.
const XREF_CALL: i64 = 1;
const XREF_JUMP: i64 = 2;
const XREF_DATA: i64 = 3;

/// Names the listing substitutes for numeric addresses.
///
/// Built once per project revision and shared behind an [`Arc`]: every disassembly window
/// renders through it, and rebuilding the whole name/function table per window would put a
/// full-table read behind every scroll.
pub(crate) struct ListingSymbols {
    names: HashMap<u64, String>,
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
        Ok(Self {
            names,
            functions,
            mapped,
            data,
        })
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
        let prefix = match size {
            1 => "byte_",
            2 => "word_",
            4 => "dword_",
            8 => "qword_",
            16 => "xmmword_",
            32 => "ymmword_",
            _ => return None,
        };
        Some(format!("{prefix}{address:x}"))
    }
}

fn in_ranges(ranges: &[(u64, u64)], address: u64) -> bool {
    ranges
        .iter()
        .any(|(start, end)| address >= *start && address < *end)
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
