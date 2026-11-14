// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! String extraction (M1): ASCII and UTF-16LE runs in readable segments.
//!
//! The default scan mirrors IDA's *Strings* window (Shift-F12) defaults, so a list built
//! here is comparable with what IDA shows for the same binary:
//!
//! - a literal is **NUL-terminated** (IDA's `STRTYPE_C` / `STRTYPE_C_16`); a printable run
//!   that ends at another non-printable byte, at a segment boundary or at EOF is not a
//!   string, which keeps x86 opcode bytes such as `D$pH` out of the list;
//! - at least `min_len` characters (5 by default, matching IDA 8.x's *minimal string
//!   length*; the terminator is not counted, see [`string_min_len`]);
//! - the control characters `ida.cfg`'s default `StrlitChars` accepts (`\a\b\t\n\v\f\r`,
//!   `ESC`) may appear *inside* a literal, so a CRLF-separated message stays one string
//!   instead of becoming two;
//! - bytes inside a decoded function are skipped: IDA does not look for strings inside
//!   instructions unless *Ignore instructions/data definitions* is enabled;
//! - ASCII and UTF-16LE are both extracted, and no string overlaps another.
//!
//! [`StringScanOptions`] relaxes any of those to reproduce the older "any non-printable
//! byte ends a run" behaviour. Scans run in chunks so a 1 GiB sample is never materialised
//! in memory; runs split across chunk boundaries are carried over.

use std::collections::BTreeSet;

use recraft_common::{Address, Result};
use recraft_core::Session;

/// `string.enc` for printable ASCII.
pub const STRING_ENC_ASCII: i64 = 1;
/// `string.enc` for UTF-16 little endian.
pub const STRING_ENC_UTF16LE: i64 = 2;
/// `string.kind` for unclassified strings.
pub const STRING_KIND_GENERIC: i64 = 0;

const CHUNK_BYTES: usize = 256 * 1024;
const MAX_STRING_CHARS: usize = 1024;

/// How literals are delimited and which candidates count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StringScanOptions {
    /// Minimum number of characters (the terminator is not counted).
    pub min_len: usize,
    /// Extract NUL-terminated ASCII literals (`STRTYPE_C`).
    pub ascii: bool,
    /// Extract NUL-terminated UTF-16LE literals (`STRTYPE_C_16`).
    pub utf16: bool,
    /// Require a terminator, as IDA does. When `false` any non-printable byte ends a
    /// run, so instruction bytes and count-prefixed text are reported too.
    pub nul_terminated: bool,
    /// Let `StrlitChars` control characters be part of a literal (IDA default).
    pub control_chars: bool,
    /// Skip candidates that start inside a decoded function (IDA's default).
    pub skip_code: bool,
}

impl Default for StringScanOptions {
    fn default() -> Self {
        Self {
            min_len: 4,
            ascii: true,
            utf16: true,
            nul_terminated: true,
            control_chars: true,
            skip_code: true,
        }
    }
}

/// One extracted string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedString {
    pub addr: Address,
    /// Length in bytes (ASCII: bytes, UTF-16: 2 * chars), terminator excluded.
    pub len: u32,
    pub enc: i64,
    pub text: String,
}

struct Scanner {
    opts: StringScanOptions,
    /// Merged, sorted function bodies; empty when `skip_code` is off.
    code: Vec<(u64, u64)>,
    max_strings: usize,
    out: Vec<ExtractedString>,
    ascii_start: Option<u64>,
    ascii: Vec<u8>,
    ascii_overflow: bool,
    wide_start: Option<u64>,
    wide: Vec<u16>,
    wide_overflow: bool,
    /// Byte waiting for its second half when a chunk ends on an odd offset.
    wide_pending: Option<(u64, u8)>,
    /// Last byte of the previous chunk (the "inside an ASCII run" check).
    prev_byte: Option<u8>,
}

impl Scanner {
    fn new(opts: StringScanOptions, code: Vec<(u64, u64)>, max_strings: usize) -> Self {
        Self {
            opts,
            code,
            max_strings,
            out: Vec::new(),
            ascii_start: None,
            ascii: Vec::new(),
            ascii_overflow: false,
            wide_start: None,
            wide: Vec::new(),
            wide_overflow: false,
            wide_pending: None,
            prev_byte: None,
        }
    }

    fn full(&self) -> bool {
        self.out.len() >= self.max_strings
    }

    /// Is `addr` inside a decoded function body?
    fn inside_code(&self, addr: u64) -> bool {
        if self.code.is_empty() {
            return false;
        }
        let index = self.code.partition_point(|&(_, end)| end <= addr);
        self.code
            .get(index)
            .is_some_and(|&(start, end)| start <= addr && addr < end)
    }

    /// A candidate literal may not start inside instruction bytes (IDA's default).
    fn may_start(&self, addr: u64) -> bool {
        !(self.opts.skip_code && self.inside_code(addr))
    }

    fn push_ascii(&mut self, addr: u64, byte: u8) {
        if is_printable_ascii(byte) || (self.opts.control_chars && is_control(byte)) {
            if self.ascii_start.is_none() {
                if !self.may_start(addr) {
                    return;
                }
                self.ascii_start = Some(addr);
                self.ascii.clear();
                self.ascii_overflow = false;
            }
            if self.ascii.len() < MAX_STRING_CHARS {
                self.ascii.push(byte);
            } else {
                self.ascii_overflow = true;
            }
        } else if self.opts.nul_terminated {
            // Only the terminator completes a literal; anything else invalidates the run.
            self.flush_ascii(byte == 0);
        } else {
            self.flush_ascii(true);
        }
    }

    fn flush_ascii(&mut self, terminated: bool) {
        if let Some(start) = self.ascii_start.take()
            && terminated
            && !self.ascii_overflow
            && self.ascii.len() >= self.opts.min_len
        {
            let text = String::from_utf8_lossy(&self.ascii).into_owned();
            self.out.push(ExtractedString {
                addr: Address::new(start),
                len: self.ascii.len() as u32,
                enc: STRING_ENC_ASCII,
                text,
            });
        }
        self.ascii.clear();
        self.ascii_overflow = false;
    }

    fn is_wide_content(&self, value: u16) -> bool {
        (0x20..=0x7e).contains(&value)
            || (self.opts.control_chars && value < 0x80 && is_control(value as u8))
    }

    fn push_wide(&mut self, addr: u64, value: u16, previous: Option<u8>) {
        if self.is_wide_content(value) {
            if self.wide_start.is_none() {
                // Don't start in the middle of an ASCII run: a trailing ASCII byte plus
                // its NUL terminator is a printable UTF-16 code unit, so `flags\0Wide`
                // would otherwise also yield `sWide`. IDA steps over the ASCII literal
                // and starts at the next unit instead.
                if previous.is_some_and(is_printable_ascii) {
                    return;
                }
                if !self.may_start(addr) {
                    return;
                }
                self.wide_start = Some(addr);
                self.wide.clear();
                self.wide_overflow = false;
            }
            if self.wide.len() < MAX_STRING_CHARS {
                self.wide.push(value);
            } else {
                self.wide_overflow = true;
            }
        } else if self.opts.nul_terminated {
            self.flush_wide(value == 0);
        } else {
            self.flush_wide(true);
        }
    }

    fn flush_wide(&mut self, terminated: bool) {
        if let Some(start) = self.wide_start.take()
            && terminated
            && !self.wide_overflow
            && self.wide.len() >= self.opts.min_len
        {
            let text: String = self
                .wide
                .iter()
                .map(|&value| char::from_u32(u32::from(value)).unwrap_or('\u{fffd}'))
                .collect();
            self.out.push(ExtractedString {
                addr: Address::new(start),
                len: (self.wide.len() * 2) as u32,
                enc: STRING_ENC_UTF16LE,
                text,
            });
        }
        self.wide.clear();
        self.wide_overflow = false;
    }

    fn feed(&mut self, base: u64, bytes: &[u8]) {
        for (offset, &byte) in bytes.iter().enumerate() {
            self.push_ascii(base + offset as u64, byte);
        }

        // UTF-16LE, keeping absolute alignment so pairs split between chunks work.
        let mut index = 0usize;
        if let Some((addr, low)) = self.wide_pending.take()
            && base <= addr + 1
            && !bytes.is_empty()
        {
            let value = u16::from_le_bytes([low, bytes[0]]);
            self.push_wide(addr, value, None);
            index = 1;
        }
        while index + 1 < bytes.len() {
            let addr = base + index as u64;
            if addr % 2 != 0 {
                index += 1;
                if index + 1 >= bytes.len() {
                    // Odd byte at the end of a segment: drop it.
                    break;
                }
                continue;
            }
            let value = u16::from_le_bytes([bytes[index], bytes[index + 1]]);
            let previous = if index > 0 {
                Some(bytes[index - 1])
            } else {
                self.prev_byte
            };
            self.push_wide(addr, value, previous);
            index += 2;
        }
        if index + 1 == bytes.len() {
            let addr = base + index as u64;
            if addr % 2 == 0 {
                self.wide_pending = Some((addr, bytes[index]));
            }
        }
        self.prev_byte = bytes.last().copied();
    }

    /// Flush at the end of a segment: a literal that runs into the boundary has no
    /// terminator, so IDA would not report it.
    fn flush_segment(&mut self) {
        let terminated = !self.opts.nul_terminated;
        self.flush_ascii(terminated);
        self.flush_wide(terminated);
        self.wide_pending = None;
        self.prev_byte = None;
    }
}

fn is_printable_ascii(byte: u8) -> bool {
    (0x20..=0x7e).contains(&byte)
}

/// `ida.cfg`'s default `StrlitChars` adds these to printable ASCII: `\a \b \t \n \v \f
/// \r ESC`.
fn is_control(byte: u8) -> bool {
    matches!(byte, 0x07 | 0x08 | 0x09 | 0x0a | 0x0b | 0x0c | 0x0d | 0x1b)
}

/// Extract ASCII and UTF-16LE strings with IDA's defaults.
///
/// `min_len` is in characters (4, so IDA's minimal length of 5 including the terminator).
/// `max_strings` bounds memory on pathological inputs.
pub fn extract_strings(
    session: &mut Session,
    min_len: usize,
    max_strings: usize,
) -> Result<Vec<ExtractedString>> {
    let options = StringScanOptions {
        min_len,
        ..StringScanOptions::default()
    };
    extract_strings_with(session, options, max_strings)
}

/// Extract strings with explicit [`StringScanOptions`].
pub fn extract_strings_with(
    session: &mut Session,
    options: StringScanOptions,
    max_strings: usize,
) -> Result<Vec<ExtractedString>> {
    let segments = session.segments().to_vec();
    let options = StringScanOptions {
        min_len: options.min_len.max(2),
        ..options
    };
    let code = if options.skip_code {
        function_ranges(session)?
    } else {
        Vec::new()
    };
    let mut scanner = Scanner::new(options, code, max_strings.max(1));
    let mut buffer = vec![0u8; CHUNK_BYTES];

    for segment in segments {
        if !segment.perm.is_readable() || segment.file_size == 0 {
            continue;
        }
        let size = segment.file_size.min(segment.len());
        let mut offset = 0u64;
        while offset < size {
            if scanner.full() {
                break;
            }
            let want = ((size - offset) as usize).min(buffer.len());
            let read = session.read_at(
                Address::new(segment.range.start.get() + offset),
                &mut buffer[..want],
            )?;
            if read == 0 {
                break;
            }
            scanner.feed(segment.range.start.get() + offset, &buffer[..read]);
            offset += read as u64;
        }
        scanner.flush_segment();
        if scanner.full() {
            break;
        }
    }

    let strings = drop_overlaps(scanner.out);
    snap_to_references(session, strings, options.min_len)
}

/// Start a literal where the code references it, the way IDA lists strings.
///
/// A data array can end with a printable byte immediately before a literal — the crackme's
/// shuffled table ends in `O`, and the banner follows with no NUL in between — so a linear
/// scan reports one character too many (`O====…`). The code references the real first
/// character, so the candidate is split there; the prefix only survives when it is long
/// enough to be a literal on its own.
fn snap_to_references(
    session: &mut Session,
    strings: Vec<ExtractedString>,
    min_len: usize,
) -> Result<Vec<ExtractedString>> {
    let referenced: BTreeSet<u64> = session
        .all_xrefs(Some(crate::XREF_DATA))?
        .into_iter()
        .map(|xref| xref.to.get())
        .collect();
    if referenced.is_empty() {
        return Ok(strings);
    }
    let mut out = Vec::with_capacity(strings.len());
    for string in strings {
        let start = string.addr.get();
        let end = start + u64::from(string.len);
        let Some(target) = referenced.range(start + 1..end).next().copied() else {
            out.push(string);
            continue;
        };
        let prefix_len = (target - start) as usize;
        if prefix_len >= min_len {
            if let Some(prefix) = decode_range(session, start, prefix_len, string.enc)? {
                out.push(prefix);
            }
        }
        let rest_len = (end - target) as usize;
        if let Some(rest) = decode_range(session, target, rest_len, string.enc)? {
            out.push(rest);
        }
    }
    Ok(out)
}

/// Re-decode `len` bytes at `start` as one string in `enc`.
fn decode_range(
    session: &mut Session,
    start: u64,
    len: usize,
    enc: i64,
) -> Result<Option<ExtractedString>> {
    if len == 0 {
        return Ok(None);
    }
    let mut bytes = vec![0u8; len];
    session.read_at(Address::new(start), &mut bytes)?;
    let (text, size) = if enc == STRING_ENC_UTF16LE {
        let wide: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        let text: String = wide
            .iter()
            .map(|&value| char::from_u32(u32::from(value)).unwrap_or('\u{fffd}'))
            .collect();
        (text, wide.len() * 2)
    } else {
        (String::from_utf8_lossy(&bytes).into_owned(), bytes.len())
    };
    if text.is_empty() {
        return Ok(None);
    }
    Ok(Some(ExtractedString {
        addr: Address::new(start),
        len: size as u32,
        enc,
        text,
    }))
}

/// IDA walks the database once and steps over each literal it reports, so a later
/// candidate that overlaps an earlier one is never listed. The ASCII and UTF-16 scans
/// here are independent passes, so the same rule is applied to the merged list: lowest
/// address first, ties to `C` (IDA checks `STRTYPE_C` first).
fn drop_overlaps(mut strings: Vec<ExtractedString>) -> Vec<ExtractedString> {
    strings.sort_by_key(|string| {
        (
            string.addr.get(),
            if string.enc == STRING_ENC_ASCII { 0 } else { 1 },
        )
    });
    let mut kept: Vec<ExtractedString> = Vec::with_capacity(strings.len());
    let mut end = 0u64;
    for string in strings {
        if string.addr.get() < end {
            continue;
        }
        end = string.addr.get() + u64::from(string.len) + terminator_len(string.enc);
        kept.push(string);
    }
    kept
}

/// Bytes a literal occupies beyond its content.
fn terminator_len(enc: i64) -> u64 {
    if enc == STRING_ENC_UTF16LE { 2 } else { 1 }
}

/// Merged function bodies: candidates must not start inside them.
fn function_ranges(session: &Session) -> Result<Vec<(u64, u64)>> {
    let mut ranges: Vec<(u64, u64)> = session
        .functions()?
        .into_iter()
        .map(|function| (function.start.get(), function.end.get()))
        .filter(|(start, end)| end > start)
        .collect();
    ranges.sort_unstable();
    let mut merged: Vec<(u64, u64)> = Vec::with_capacity(ranges.len());
    for (start, end) in ranges {
        match merged.last_mut() {
            Some(last) if start <= last.1 => last.1 = last.1.max(end),
            _ => merged.push((start, end)),
        }
    }
    Ok(merged)
}

/// Persist strings into the session's derived string table.
pub fn persist_strings(session: &mut Session, strings: &[ExtractedString]) -> Result<()> {
    let rows: Vec<(Address, u32, i64, i64)> = strings
        .iter()
        .map(|string| (string.addr, string.len, string.enc, STRING_KIND_GENERIC))
        .collect();
    session.replace_auto_strings(&rows)?;
    session
        .project_mut()
        .set_meta_string(STRINGS_REV_META_KEY, &STRINGS_REV.to_string())
}

/// Revision of the extraction rules. Bump it whenever the scanner output changes so a
/// project analyzed by an older build refreshes its stored list instead of showing stale
/// text (the `O` glued to a banner was such a case).
pub const STRINGS_REV: i64 = 2;

const STRINGS_REV_META_KEY: &str = "strings.rev";

/// Whether the stored list was produced by the current extraction rules.
pub fn strings_are_stale(session: &Session) -> bool {
    session
        .project()
        .meta_string(STRINGS_REV_META_KEY)
        .ok()
        .flatten()
        .and_then(|value| value.trim().parse::<i64>().ok())
        != Some(STRINGS_REV)
}

/// Re-extract the list with the current rules and store it.
pub fn refresh_strings(session: &mut Session) -> Result<()> {
    let min_len = string_min_len(session);
    let strings = extract_strings(session, min_len, 50_000)?;
    persist_strings(session, &strings)
}

/// Project `meta` key holding the minimum string length (IDA's *Minimal string length*,
/// which lives in the database and is applied on **Rebuild**).
pub const MIN_STRING_LEN_META_KEY: &str = "strings.min_len";

/// Default minimum length: five characters, as IDA 8.x ships it. IDA's Length column
/// counts the terminator, its minimum does not (`".bss"` is listed at 4, dropped at 5).
pub const DEFAULT_MIN_STRING_LEN: usize = 5;

/// Smallest accepted value: two characters keeps `".o"`-style text but stops single
/// printable bytes from turning opcode noise into "strings".
pub const MIN_STRING_LEN_FLOOR: usize = 2;

/// Largest accepted value: longer literals are rare, and the ceiling keeps a typo from
/// silently emptying the list.
pub const MIN_STRING_LEN_CEILING: usize = 64;

/// Clamp a user-supplied minimum length into the supported range.
pub fn clamp_min_string_len(value: usize) -> usize {
    value.clamp(MIN_STRING_LEN_FLOOR, MIN_STRING_LEN_CEILING)
}

/// The project's configured minimum string length, or [`DEFAULT_MIN_STRING_LEN`].
pub fn string_min_len(session: &Session) -> usize {
    session
        .project()
        .meta_string(MIN_STRING_LEN_META_KEY)
        .ok()
        .flatten()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .map(clamp_min_string_len)
        .unwrap_or(DEFAULT_MIN_STRING_LEN)
}

/// Store the minimum string length in the project (`strings.min_len`).
pub fn set_string_min_len(session: &mut Session, value: usize) -> Result<()> {
    session.project_mut().set_meta_string(
        MIN_STRING_LEN_META_KEY,
        &clamp_min_string_len(value).to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn corpus_path() -> Option<PathBuf> {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../corpus/build/x64/corpus.exe");
        path.exists().then_some(path)
    }

    /// Feed one buffer through a fresh scanner at `base`.
    fn scan(base: u64, bytes: &[u8], opts: StringScanOptions) -> Vec<ExtractedString> {
        let mut scanner = Scanner::new(opts, Vec::new(), 1_000);
        scanner.feed(base, bytes);
        scanner.flush_segment();
        scanner.out
    }

    #[test]
    fn nul_terminated_ascii_is_extracted() {
        let strings = scan(0x1000, b"hello\0", StringScanOptions::default());
        assert_eq!(strings.len(), 1);
        assert_eq!(strings[0].text, "hello");
        assert_eq!(strings[0].len, 5);
        assert_eq!(strings[0].enc, STRING_ENC_ASCII);
    }

    #[test]
    fn unterminated_printable_run_is_not_a_string() {
        // x86 bytes (`D$pH`) have no terminator, so IDA never lists them.
        let opts = StringScanOptions::default();
        assert!(scan(0x1000, b"D$pH", opts).is_empty());
        assert!(scan(0x1000, b"D$pH\0", opts).len() == 1);
        // Same bytes at a segment boundary: still unterminated.
        assert!(scan(0x1000, b"abcdefgh", opts).is_empty());
    }

    #[test]
    fn short_literals_need_a_terminator_too() {
        let opts = StringScanOptions::default();
        assert!(scan(0x1000, b"abc\0", opts).is_empty());
        assert!(scan(0x1000, b"abcd\0", opts).len() == 1);
    }

    #[test]
    fn legacy_mode_ends_runs_at_any_non_printable_byte() {
        let opts = StringScanOptions {
            nul_terminated: false,
            ..StringScanOptions::default()
        };
        let strings = scan(0x1000, b"hello\x01world", opts);
        assert_eq!(strings.len(), 2);
        assert_eq!(strings[0].text, "hello");
        assert_eq!(strings[1].text, "world");
    }

    #[test]
    fn control_characters_stay_inside_a_literal() {
        let strings = scan(0x1000, b"line1\r\nline2\0", StringScanOptions::default());
        assert_eq!(strings.len(), 1);
        assert_eq!(strings[0].text, "line1\r\nline2");
        assert_eq!(strings[0].len, 12);
    }

    #[test]
    fn nul_inside_a_literal_invalidates_the_run() {
        // `abc` then NUL then `def\0`: the NUL completes "abc" (too short) and starts a
        // new candidate at `def`, which is also too short. Nothing is reported.
        let strings = scan(0x1000, b"abc\0def\0", StringScanOptions::default());
        assert!(strings.is_empty());
    }

    #[test]
    fn candidates_inside_decoded_code_are_skipped() {
        let opts = StringScanOptions::default();
        let mut scanner = Scanner::new(opts, vec![(0x1000, 0x1004)], 100);
        scanner.feed(0x1000, b"abcd\0efgh\0");
        scanner.flush_segment();
        let strings = scanner.out;
        assert_eq!(strings.len(), 1);
        assert_eq!(strings[0].addr, Address::new(0x1005));
        assert_eq!(strings[0].text, "efgh");
    }

    #[test]
    fn utf16_literals_are_extracted() {
        let bytes: Vec<u8> = "wide\0\0"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        let strings = scan(0x1000, &bytes, StringScanOptions::default());
        assert_eq!(strings.len(), 1);
        assert_eq!(strings[0].text, "wide");
        assert_eq!(strings[0].enc, STRING_ENC_UTF16LE);
        assert_eq!(strings[0].len, 8);
    }

    #[test]
    fn utf16_literal_needs_its_terminator() {
        let bytes: Vec<u8> = "wide".encode_utf16().flat_map(u16::to_le_bytes).collect();
        assert!(scan(0x1000, &bytes, StringScanOptions::default()).is_empty());
    }

    #[test]
    fn overlapping_wide_candidate_is_dropped() {
        // `flags\0` is an ASCII literal; the UTF-16 pass would otherwise also read the
        // trailing `s` plus its NUL as the start of `sWide`.
        let mut bytes = b"flags\0".to_vec();
        for unit in "Wide\0".encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        let mut scanner = Scanner::new(StringScanOptions::default(), Vec::new(), 100);
        scanner.feed(0x1000, &bytes);
        scanner.flush_segment();
        let strings = drop_overlaps(scanner.out);
        let texts: Vec<&str> = strings.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(texts, vec!["flags", "Wide"]);
    }

    #[test]
    fn minimum_length_matches_ida_and_is_stored_in_the_project() {
        let Some(path) = corpus_path() else {
            return;
        };
        let bytes = std::fs::read(path).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("min-len.rcraft");
        let mut session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();

        // IDA 8.x defaults to five characters: a four-character string (".bss") is listed at
        // four and dropped at five.
        assert_eq!(string_min_len(&session), DEFAULT_MIN_STRING_LEN);
        let four = extract_strings(&mut session, 4, 10_000).unwrap();
        assert!(
            four.iter().any(|string| string.text == ".bss"),
            "four-character strings must be listed at length 4"
        );
        let five = extract_strings(&mut session, 5, 10_000).unwrap();
        assert!(
            !five.iter().any(|string| string.text == ".bss"),
            "four-character strings must be dropped at length 5"
        );

        // The setting is project state (IDA keeps its Strings setup in the database too).
        set_string_min_len(&mut session, 8).unwrap();
        assert_eq!(string_min_len(&session), 8);
        drop(session);
        let session = Session::open(&project).unwrap();
        assert_eq!(string_min_len(&session), 8);

        assert_eq!(clamp_min_string_len(0), MIN_STRING_LEN_FLOOR);
        assert_eq!(clamp_min_string_len(usize::MAX), MIN_STRING_LEN_CEILING);
    }

    #[test]
    fn corpus_strings_are_extracted() {
        let Some(path) = corpus_path() else {
            return;
        };
        let bytes = std::fs::read(path).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("strings.rcraft");
        let mut session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let strings = extract_strings(&mut session, 4, 10_000).unwrap();

        let ascii = strings
            .iter()
            .find(|string| string.text == "ReCraft analysis corpus");
        assert!(ascii.is_some(), "missing ASCII banner: {strings:?}");
        assert_eq!(ascii.unwrap().enc, STRING_ENC_ASCII);

        let wide = strings
            .iter()
            .find(|string| string.text == "ReCraft wide corpus");
        assert!(wide.is_some(), "missing UTF-16 banner: {strings:?}");
        assert_eq!(wide.unwrap().enc, STRING_ENC_UTF16LE);
    }

    #[test]
    fn notepad_has_many_strings() {
        let Some(bytes) = std::fs::read("C:/Windows/System32/notepad.exe").ok() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("strings.rcraft");
        let mut session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let strings = extract_strings(&mut session, 5, 100_000).unwrap();
        assert!(strings.len() > 200, "only {} strings", strings.len());
        assert!(strings.iter().any(|string| string.text.contains("notepad")));
    }
}
