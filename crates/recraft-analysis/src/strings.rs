// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! String extraction (M1): ASCII and UTF-16LE runs in readable segments.
//!
//! Scans segment bytes in chunks so a 1 GiB sample does not have to be materialised in
//! memory. Runs split across chunk boundaries are carried over.

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

/// One extracted string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedString {
    pub addr: Address,
    /// Length in bytes (ASCII: bytes, UTF-16: 2 * chars).
    pub len: u32,
    pub enc: i64,
    pub text: String,
}

#[derive(Default)]
struct Scanner {
    min_len: usize,
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
    /// Last byte of the previous chunk (for the "inside an ASCII run" check).
    prev_byte: Option<u8>,
}

impl Scanner {
    fn full(&self) -> bool {
        self.out.len() >= self.max_strings
    }

    fn push_ascii(&mut self, addr: u64, byte: u8) {
        if is_printable_ascii(byte) {
            if self.ascii_start.is_none() {
                self.ascii_start = Some(addr);
                self.ascii.clear();
                self.ascii_overflow = false;
            }
            if self.ascii.len() < MAX_STRING_CHARS {
                self.ascii.push(byte);
            } else {
                self.ascii_overflow = true;
            }
        } else {
            self.flush_ascii();
        }
    }

    fn flush_ascii(&mut self) {
        if let Some(start) = self.ascii_start.take()
            && !self.ascii_overflow
            && self.ascii.len() >= self.min_len
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

    fn push_wide(&mut self, addr: u64, value: u16, prev_byte: Option<u8>) {
        let printable = value == 0x20 || value == 0x09 || (0x21..=0x7e).contains(&value);
        if printable {
            if self.wide_start.is_none() {
                // Don't start in the middle of an ASCII run: the trailing ASCII byte
                // plus its NUL terminator looks like a printable UTF-16 code unit.
                if prev_byte.is_some_and(is_printable_ascii) {
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
        } else {
            self.flush_wide();
        }
    }

    fn flush_wide(&mut self) {
        if let Some(start) = self.wide_start.take()
            && !self.wide_overflow
            && self.wide.len() >= self.min_len
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
}

fn is_printable_ascii(byte: u8) -> bool {
    (0x20..=0x7e).contains(&byte)
}

/// Heuristic filter for "interesting" strings: drop pure whitespace and control junk.
fn worth_keeping(text: &str) -> bool {
    text.chars().any(|ch| ch.is_ascii_alphanumeric())
}

/// Extract ASCII and UTF-16LE strings from readable segments.
///
/// `min_len` is in characters (4 is a good default). `max_strings` bounds memory on
/// pathological inputs.
pub fn extract_strings(
    session: &mut Session,
    min_len: usize,
    max_strings: usize,
) -> Result<Vec<ExtractedString>> {
    let segments = session.segments().to_vec();
    let mut scanner = Scanner {
        min_len: min_len.max(2),
        max_strings: max_strings.max(1),
        ..Scanner::default()
    };
    let mut buffer = vec![0u8; CHUNK_BYTES];

    for segment in segments {
        if !segment.perm.is_readable() || segment.file_size == 0 {
            continue;
        }
        scanner.prev_byte = None;
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
        scanner.flush_ascii();
        scanner.flush_wide();
        scanner.wide_pending = None;
        if scanner.full() {
            break;
        }
    }

    scanner.out.retain(|string| worth_keeping(&string.text));
    Ok(scanner.out)
}

/// Persist strings into the session's derived string table.
pub fn persist_strings(session: &mut Session, strings: &[ExtractedString]) -> Result<()> {
    let rows: Vec<(Address, u32, i64, i64)> = strings
        .iter()
        .map(|string| (string.addr, string.len, string.enc, STRING_KIND_GENERIC))
        .collect();
    session.replace_auto_strings(&rows)
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
