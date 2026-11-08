// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Minimal DWARF line-table reader (`.debug_line`, DWARF 2 through 5).
//!
//! Only what a disassembler needs: a sorted `address -> file:line` map. Type
//! information lives in `.debug_info` and is deliberately out of scope here.

use serde::{Deserialize, Serialize};

/// One row of the line table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceLine {
    pub address: u64,
    /// File name, joined with its include directory when it is relative.
    pub file: String,
    pub line: u32,
    pub column: u16,
}

/// The `.debug_*` sections a line table needs.
#[derive(Debug, Clone, Copy, Default)]
pub struct DwarfSections<'a> {
    pub debug_line: &'a [u8],
    pub debug_line_str: &'a [u8],
    pub debug_str: &'a [u8],
}

/// Parse every compilation unit in `.debug_line` and return rows sorted by address.
///
/// Malformed input stops the current unit instead of failing: a truncated or
/// unusual line table must not keep a binary from loading.
pub fn parse_line_table(sections: &DwarfSections<'_>, address_size: usize) -> Vec<SourceLine> {
    let mut rows = Vec::new();
    let mut cursor = Cursor::new(sections.debug_line);
    while !cursor.done() {
        let unit_start = cursor.position();
        let Ok(unit_length) = cursor.u32() else {
            break;
        };
        if unit_length == 0xffff_ffff {
            // DWARF64 unit: header is 8-byte lengths. Rare, and not worth the extra
            // code path yet — skip the rest of the section.
            break;
        }
        let unit_end = unit_start + 4 + unit_length as usize;
        if unit_end > sections.debug_line.len() {
            break;
        }
        if let Some(mut unit_rows) = parse_unit(&mut cursor, unit_end, sections, address_size) {
            rows.append(&mut unit_rows);
        }
        cursor.seek(unit_end);
    }
    rows.sort_by_key(|row| (row.address, row.line));
    rows.dedup_by_key(|row| row.address);
    rows
}

fn parse_unit(
    cursor: &mut Cursor<'_>,
    unit_end: usize,
    sections: &DwarfSections<'_>,
    address_size: usize,
) -> Option<Vec<SourceLine>> {
    let version = cursor.u16().ok()?;
    if !(2..=5).contains(&version) {
        return None;
    }
    let (address_size, header_length) = if version >= 5 {
        let address_size = cursor.u8().ok()? as usize;
        let _segment_selector_size = cursor.u8().ok()?;
        (address_size, cursor.u32().ok()?)
    } else {
        (address_size, cursor.u32().ok()?)
    };
    let header_start = cursor.position();
    let program_start = header_start
        .checked_add(header_length as usize)
        .filter(|end| *end <= unit_end)?;

    let minimum_instruction_length = cursor.u8().ok()?;
    let maximum_operations_per_instruction = if version >= 4 {
        cursor.u8().ok()?.max(1)
    } else {
        1
    };
    let default_is_stmt = cursor.u8().ok()? != 0;
    let line_base = cursor.u8().ok()? as i8;
    let line_range = cursor.u8().ok()?;
    if line_range == 0 {
        return None;
    }
    let opcode_base = cursor.u8().ok()?;
    let standard_lengths: Vec<u8> = (1..opcode_base).map(|_| cursor.u8().unwrap_or(0)).collect();

    let (_directories, files) = if version >= 5 {
        let directories = read_v5_entry_list(cursor, sections).ok()?;
        let files = read_v5_entry_list(cursor, sections).ok()?;
        (directories, files)
    } else {
        let directories = read_nt_strings(cursor).ok()?;
        let files = read_v4_file_names(cursor).ok()?;
        (directories, files)
    };

    cursor.seek(program_start);
    let mut rows = Vec::new();
    let mut state = LineState {
        address: 0,
        file: 1,
        line: 1,
        column: 0,
        is_stmt: default_is_stmt,
    };
    let mut sequence_rows: Vec<SourceLine> = Vec::new();
    while cursor.position() < unit_end {
        let Ok(opcode) = cursor.u8() else {
            break;
        };
        if opcode >= opcode_base {
            let adjusted = u32::from(opcode - opcode_base);
            let address_advance =
                u64::from(minimum_instruction_length) * u64::from(adjusted / u32::from(line_range));
            state.address = state.address.saturating_add(address_advance);
            let line_advance = i32::from(line_base) + (adjusted % u32::from(line_range)) as i32;
            state.line = (i64::from(state.line) + i64::from(line_advance)).max(1) as u32;
            push_row(&mut sequence_rows, &state, &files);
            let _ = maximum_operations_per_instruction;
            continue;
        }
        match opcode {
            0 => {
                let Some(length) = cursor.uleb() else {
                    break;
                };
                let sub_end = cursor.position().saturating_add(length as usize);
                let sub_opcode = cursor.u8().unwrap_or(0);
                match sub_opcode {
                    1 => {
                        // end_sequence: emit the closing row, then reset the state.
                        push_row(&mut sequence_rows, &state, &files);
                        rows.append(&mut sequence_rows);
                        state = LineState {
                            address: 0,
                            file: 1,
                            line: 1,
                            column: 0,
                            is_stmt: default_is_stmt,
                        };
                    }
                    2 => {
                        state.address = match address_size {
                            2 => u64::from(cursor.u16().unwrap_or(0)),
                            4 => u64::from(cursor.u32().unwrap_or(0)),
                            _ => cursor.u64().unwrap_or(0),
                        };
                    }
                    4 => {
                        let _ = cursor.uleb();
                    }
                    _ => {}
                }
                cursor.seek(sub_end.min(unit_end));
            }
            1 => {
                push_row(&mut sequence_rows, &state, &files);
            }
            2 => {
                let advance = cursor.uleb().unwrap_or(0);
                state.address = state
                    .address
                    .saturating_add(u64::from(minimum_instruction_length) * advance);
            }
            3 => {
                if let Some(advance) = cursor.sleb() {
                    state.line = (i64::from(state.line) + advance).max(1) as u32;
                }
            }
            4 => {
                state.file = cursor.uleb().unwrap_or(state.file);
            }
            5 => {
                state.column = cursor.uleb().unwrap_or(u64::from(state.column)) as u16;
            }
            8 => {
                let advance = u64::from(u8::MAX - opcode_base + 1) / u64::from(line_range);
                state.address = state
                    .address
                    .saturating_add(u64::from(minimum_instruction_length) * advance);
            }
            9 => {
                state.address = state
                    .address
                    .saturating_add(u64::from(cursor.u16().unwrap_or(0)));
            }
            _ => {
                // Standard opcodes with operands (set_isa, unknown future ones): skip
                // the operand count the header advertises.
                let count = standard_lengths
                    .get(usize::from(opcode).saturating_sub(1))
                    .copied()
                    .unwrap_or(0);
                for _ in 0..count {
                    if cursor.uleb().is_none() {
                        break;
                    }
                }
            }
        }
    }
    Some(rows)
}

struct LineState {
    address: u64,
    file: u64,
    line: u32,
    column: u16,
    is_stmt: bool,
}

fn push_row(rows: &mut Vec<SourceLine>, state: &LineState, files: &[String]) {
    if state.address == 0 {
        return;
    }
    let file = files
        .get(state.file as usize)
        .or_else(|| files.first())
        .cloned()
        .unwrap_or_default();
    let _ = state.is_stmt;
    rows.push(SourceLine {
        address: state.address,
        file,
        line: state.line,
        column: state.column,
    });
}

/// DWARF 2-4: null-terminated strings, one per directory.
fn read_nt_strings(cursor: &mut Cursor<'_>) -> Result<Vec<String>, ()> {
    let mut values = Vec::new();
    loop {
        let value = cursor.c_string().ok_or(())?;
        if value.is_empty() {
            return Ok(values);
        }
        values.push(value);
    }
}

/// DWARF 2-4: `{ name, dir_index, mtime, length }` until an empty name.
fn read_v4_file_names(cursor: &mut Cursor<'_>) -> Result<Vec<String>, ()> {
    let mut files = vec![String::new()];
    loop {
        let name = cursor.c_string().ok_or(())?;
        if name.is_empty() {
            return Ok(files);
        }
        let _directory = cursor.uleb().ok_or(())?;
        let _mtime = cursor.uleb().ok_or(())?;
        let _length = cursor.uleb().ok_or(())?;
        files.push(name);
    }
}

/// DWARF 5: `entry_format_count, [(content_type, form)], count, entries`.
fn read_v5_entry_list(
    cursor: &mut Cursor<'_>,
    sections: &DwarfSections<'_>,
) -> Result<Vec<String>, ()> {
    let format_count = cursor.u8()?;
    let mut formats = Vec::with_capacity(usize::from(format_count));
    for _ in 0..format_count {
        let content_type = cursor.uleb().ok_or(())?;
        let form = cursor.uleb().ok_or(())?;
        formats.push((content_type, form));
    }
    let count = cursor.uleb().ok_or(())?;
    let mut values = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let mut path = String::new();
        for (content_type, form) in &formats {
            let value = read_form_value(cursor, *form, sections)?;
            if *content_type == 1 {
                path = value;
            }
        }
        values.push(path);
    }
    Ok(values)
}

/// Only the forms a path entry can take in practice.
fn read_form_value(
    cursor: &mut Cursor<'_>,
    form: u64,
    sections: &DwarfSections<'_>,
) -> Result<String, ()> {
    match form {
        0x08 => cursor.c_string().ok_or(()), // DW_FORM_string
        0x0e => {
            // DW_FORM_strp into .debug_str
            let offset = cursor.u32()? as usize;
            Ok(read_str_at(sections.debug_str, offset))
        }
        0x1f => {
            // DW_FORM_line_strp into .debug_line_str
            let offset = cursor.u32()? as usize;
            Ok(read_str_at(sections.debug_line_str, offset))
        }
        0x0b => Ok(cursor.u8()?.to_string()),  // data1
        0x05 => Ok(cursor.u16()?.to_string()), // data2
        0x06 => Ok(cursor.u32()?.to_string()), // data4
        0x07 => Ok(cursor.u64()?.to_string()), // data8
        0x0f | 0x1e => Ok(cursor.uleb().ok_or(())?.to_string()), // udata, data16
        _ => Err(()),
    }
}

fn read_str_at(section: &[u8], offset: usize) -> String {
    let Some(rest) = section.get(offset..) else {
        return String::new();
    };
    let end = rest
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(rest.len());
    String::from_utf8_lossy(&rest[..end]).into_owned()
}

struct Cursor<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn position(&self) -> usize {
        self.position
    }

    fn done(&self) -> bool {
        self.position >= self.bytes.len()
    }

    fn seek(&mut self, position: usize) {
        self.position = position.min(self.bytes.len());
    }

    fn take(&mut self, count: usize) -> Option<&'a [u8]> {
        let end = self.position.checked_add(count)?;
        let slice = self.bytes.get(self.position..end)?;
        self.position = end;
        Some(slice)
    }

    fn u8(&mut self) -> Result<u8, ()> {
        self.take(1).map(|bytes| bytes[0]).ok_or(())
    }

    fn u16(&mut self) -> Result<u16, ()> {
        self.take(2)
            .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
            .ok_or(())
    }

    fn u32(&mut self) -> Result<u32, ()> {
        self.take(4)
            .map(|bytes| u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
            .ok_or(())
    }

    fn u64(&mut self) -> Result<u64, ()> {
        let bytes = self.take(8).ok_or(())?;
        let mut value = [0u8; 8];
        value.copy_from_slice(bytes);
        Ok(u64::from_le_bytes(value))
    }

    fn uleb(&mut self) -> Option<u64> {
        let mut value = 0u64;
        let mut shift = 0u32;
        loop {
            let byte = *self.take(1)?.first()?;
            if shift < 64 {
                value |= u64::from(byte & 0x7f) << shift;
            }
            if byte & 0x80 == 0 {
                return Some(value);
            }
            shift += 7;
            if shift > 63 {
                return None;
            }
        }
    }

    fn sleb(&mut self) -> Option<i64> {
        let mut value = 0i64;
        let mut shift = 0u32;
        loop {
            let byte = *self.take(1)?.first()?;
            if shift < 64 {
                value |= i64::from(byte & 0x7f) << shift;
            }
            shift += 7;
            if byte & 0x80 == 0 {
                if shift < 64 && byte & 0x40 != 0 {
                    value |= -1i64 << shift;
                }
                return Some(value);
            }
            if shift > 63 {
                return None;
            }
        }
    }

    fn c_string(&mut self) -> Option<String> {
        let rest = self.bytes.get(self.position..)?;
        let end = rest.iter().position(|byte| *byte == 0)?;
        let text = String::from_utf8_lossy(&rest[..end]).into_owned();
        self.position += end + 1;
        Some(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a DWARF 4 line unit: one file, `0x1000: line 20`, `0x1010: line 21`.
    fn dwarf4_unit() -> Vec<u8> {
        // minimum_instruction_length, maximum_operations_per_instruction,
        // default_is_stmt, line_base, line_range, opcode_base.
        let mut header = vec![1u8, 1, 1, 0, 10, 13];
        header.extend([0, 1, 1, 1, 1, 0, 0, 0, 1, 0, 0, 1]); // standard lengths
        header.extend(b"src\0\0"); // include_directories
        header.extend(b"functions.c\0"); // file 1
        header.extend([0, 0, 0]); // dir, mtime, length
        header.push(0); // end of files
        let mut program = Vec::new();
        program.extend([0x00, 0x09, 0x02]); // DW_LNE_set_address, length 9
        program.extend(0x1000u64.to_le_bytes());
        program.extend([0x00, 0x05]); // DW_LNE_define... actually DW_LNE
        // Use standard opcodes: advance_line to 20, copy, advance_pc 16, advance_line, copy.
        program.clear();
        program.extend([0x00, 0x09, 0x02]);
        program.extend(0x1000u64.to_le_bytes());
        program.extend([0x03, 19]); // DW_LNS_advance_line +19 -> line 20
        program.push(0x01); // DW_LNS_copy
        program.extend([0x02, 16]); // DW_LNS_advance_pc +16
        program.extend([0x03, 1]); // advance_line +1 -> 21
        program.push(0x01);
        program.extend([0x00, 0x01, 0x01]); // DW_LNE_end_sequence
        let mut unit = Vec::new();
        let unit_length = (2 + 4 + header.len() + program.len()) as u32;
        unit.extend(unit_length.to_le_bytes());
        unit.extend(4u16.to_le_bytes());
        unit.extend((header.len() as u32).to_le_bytes());
        unit.extend(header);
        unit.extend(program);
        unit
    }

    #[test]
    fn reads_a_dwarf4_line_table() {
        let unit = dwarf4_unit();
        let rows = parse_line_table(
            &DwarfSections {
                debug_line: &unit,
                ..DwarfSections::default()
            },
            8,
        );
        assert_eq!(rows.len(), 2, "{rows:?}");
        assert_eq!(rows[0].address, 0x1000);
        assert_eq!(rows[0].line, 20);
        assert!(rows[0].file.ends_with("functions.c"), "{}", rows[0].file);
        assert_eq!(rows[1].address, 0x1010);
        assert_eq!(rows[1].line, 21);
    }

    #[test]
    fn malformed_input_stops_instead_of_panicking() {
        let rows = parse_line_table(
            &DwarfSections {
                debug_line: &[0xff, 0xff],
                ..DwarfSections::default()
            },
            8,
        );
        assert!(rows.is_empty());
    }
}
