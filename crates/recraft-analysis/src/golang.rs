// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Go runtime symbol recovery (M1).
//!
//! Go binaries keep the `pclntab` (function table with names and PC ranges) because
//! the runtime needs it for stack traces, even in stripped builds. We locate the
//! `pcHeader` magic and parse the function table, which gives exact function starts
//! and real names (`main.main`, `runtime.mallocgc`, ...).

use recraft_common::{Address, Result};
use recraft_core::Session;

const GO12_MAGIC: u32 = 0xffff_fffb;
const GO116_MAGIC: u32 = 0xffff_fffa;
const GO118_MAGIC: u32 = 0xffff_fff0;
const GO120_MAGIC: u32 = 0xffff_fff1;

const CHUNK: usize = 1 << 20;
const OVERLAP: usize = 4;
const MAX_GO_FUNCTIONS: u64 = 1_000_000;

/// One function recovered from `pclntab`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoFunction {
    pub start: Address,
    pub name: String,
}

/// Locate the first *valid* `pclntab` header in readable segments.
fn find_header(session: &mut Session, image_base: u64, image_size: u64) -> Result<Option<Address>> {
    let segments = session.segments().to_vec();
    let mut buffer = vec![0u8; CHUNK];
    for segment in segments {
        if !segment.perm.is_readable() || segment.file_size == 0 {
            continue;
        }
        let size = segment.file_size.min(segment.len());
        let mut offset = 0u64;
        while offset < size {
            let want = ((size - offset) as usize).min(CHUNK);
            let read = session.read_at(
                Address::new(segment.range.start.get() + offset),
                &mut buffer[..want],
            )?;
            if read == 0 {
                break;
            }
            let mut scan_from = 0usize;
            while let Some(relative) = find_magic(&buffer[scan_from..read]) {
                let index = scan_from + relative;
                let candidate = Address::new(segment.range.start.get() + offset + index as u64);
                if valid_header(session, candidate, image_base, image_size)? {
                    return Ok(Some(candidate));
                }
                scan_from = index + 4;
                if scan_from + 4 > read {
                    break;
                }
            }
            if read <= OVERLAP {
                break;
            }
            // Overlap so a magic split between chunks is not missed.
            offset += (read - OVERLAP) as u64;
        }
    }
    Ok(None)
}

/// Cheap sanity check that rejects lookalike constants inside code/data.
fn valid_header(
    session: &mut Session,
    base: Address,
    _image_base: u64,
    image_size: u64,
) -> Result<bool> {
    let pointer_size = read_u8(session, Address::new(base.get() + 7))?;
    if pointer_size != 4 && pointer_size != 8 {
        return Ok(false);
    }
    let pointer = u64::from(pointer_size);
    let nfunc = read_ptr(session, Address::new(base.get() + 8), pointer_size)?;
    if nfunc == 0 || nfunc > MAX_GO_FUNCTIONS {
        return Ok(false);
    }
    let funcname_offset = read_ptr(
        session,
        Address::new(base.get() + 8 + 3 * pointer),
        pointer_size,
    )?;
    let pcln_offset = read_ptr(
        session,
        Address::new(base.get() + 8 + 7 * pointer),
        pointer_size,
    )?;
    Ok(funcname_offset > 0
        && funcname_offset < image_size
        && pcln_offset > 0
        && pcln_offset < image_size)
}

fn find_magic(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| {
        let value = u32::from_le_bytes([window[0], window[1], window[2], window[3]]);
        matches!(value, GO12_MAGIC | GO116_MAGIC | GO118_MAGIC | GO120_MAGIC)
    })
}

fn read_bytes<const N: usize>(session: &mut Session, addr: Address) -> Result<[u8; N]> {
    let mut buffer = [0u8; N];
    let read = session.read_at(addr, &mut buffer)?;
    if read != N {
        return Err(recraft_common::Error::internal(format!(
            "short read at {:#x}: {read}/{N}",
            addr.get()
        )));
    }
    Ok(buffer)
}

fn read_u8(session: &mut Session, addr: Address) -> Result<u8> {
    Ok(read_bytes::<1>(session, addr)?[0])
}

fn read_u32(session: &mut Session, addr: Address) -> Result<u32> {
    Ok(u32::from_le_bytes(read_bytes(session, addr)?))
}

fn read_ptr(session: &mut Session, addr: Address, pointer_size: u8) -> Result<u64> {
    match pointer_size {
        8 => Ok(u64::from_le_bytes(read_bytes(session, addr)?)),
        4 => Ok(u64::from(u32::from_le_bytes(read_bytes(session, addr)?))),
        other => Err(recraft_common::Error::internal(format!(
            "unsupported pointer size {other}"
        ))),
    }
}

/// Parse `pclntab` and return every executable function it describes.
pub fn parse_pclntab(session: &mut Session) -> Result<Option<Vec<GoFunction>>> {
    let image_base = session.image().image_base.get();
    let image_size = session.image().size_of_image;
    let Some(base) = find_header(session, image_base, image_size)? else {
        return Ok(None);
    };

    let magic = read_u32(session, base)?;
    // magic, pad1, pad2, minLC, ptrSize
    let pointer_size = read_u8(session, Address::new(base.get() + 7))?;
    if pointer_size != 4 && pointer_size != 8 {
        return Ok(None);
    }
    let pointer = u64::from(pointer_size);
    let mut cursor = base.get() + 8;

    let nfunc = read_ptr(session, Address::new(cursor), pointer_size)?;
    cursor += pointer; // nfiles
    cursor += pointer; // nfiles value
    let stored_text_start = read_ptr(session, Address::new(cursor), pointer_size)?;
    cursor += pointer;
    let funcname_offset = read_ptr(session, Address::new(cursor), pointer_size)?;
    cursor += 4 * pointer; // cu, filetab, pctab, then pcln
    let pcln_offset = read_ptr(session, Address::new(cursor), pointer_size)?;
    if nfunc == 0 || nfunc > MAX_GO_FUNCTIONS {
        return Ok(None);
    }
    // Go 1.26 removed textStart from pcHeader (the field is now unused/zero and
    // callers must use moduledata.text). For PE images that is the .text start.
    let text_start = if stored_text_start != 0 {
        stored_text_start
    } else {
        let segments = session.segments().to_vec();
        match segments
            .iter()
            .filter(|segment| segment.perm.is_executable())
            .map(|segment| segment.range.start.get())
            .min()
        {
            Some(start) => start,
            None => return Ok(None),
        }
    };

    // functab entry layout: uint32 pairs since Go 1.18, uintptr pairs before.
    let entry_size = if magic == GO120_MAGIC || magic == GO118_MAGIC {
        8u64
    } else {
        2 * pointer
    };
    let funcname_base = Address::new(base.get() + funcname_offset);
    let table_base = base.get() + pcln_offset;
    let executable = session.segments().to_vec();

    let mut functions = Vec::new();
    for index in 0..nfunc {
        let entry = Address::new(table_base + index * entry_size);
        let (entry_off, func_off) = if entry_size == 8 {
            (
                u64::from(read_u32(session, entry)?),
                u64::from(read_u32(session, Address::new(entry.get() + 4))?),
            )
        } else {
            (
                read_ptr(session, entry, pointer_size)?,
                read_ptr(session, Address::new(entry.get() + pointer), pointer_size)?,
            )
        };
        let start = Address::new(text_start.wrapping_add(entry_off));
        if !executable
            .iter()
            .any(|segment| segment.perm.is_executable() && segment.contains(start))
        {
            continue;
        }

        let func = Address::new(base.get() + pcln_offset + func_off);
        let name_off = read_u32(session, Address::new(func.get() + 4))? as i32;
        let Some(name) = read_go_name(session, funcname_base, name_off) else {
            continue;
        };
        functions.push(GoFunction { start, name });
    }

    functions.sort_by_key(|function| function.start);
    functions.dedup_by_key(|function| function.start);
    Ok(Some(functions))
}

fn read_go_name(session: &mut Session, funcname_base: Address, name_off: i32) -> Option<String> {
    if name_off < 0 {
        return None;
    }
    let addr = Address::new(funcname_base.get().wrapping_add(name_off as u64));
    // funcnametab stores NUL-terminated strings (linker `AddCStringAt`).
    let mut bytes = Vec::with_capacity(64);
    for chunk in 0..8 {
        let mut buffer = [0u8; 64];
        let read = session
            .read_at(Address::new(addr.get() + chunk * 64), &mut buffer)
            .ok()?;
        if read == 0 {
            break;
        }
        for &byte in &buffer[..read] {
            if byte == 0 {
                let name = String::from_utf8(bytes).ok()?;
                if name.is_empty() || name.len() > 512 {
                    return None;
                }
                return Some(name);
            }
            bytes.push(byte);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn go_corpus() -> Option<PathBuf> {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../corpus/build/go/corpus-go.exe");
        path.exists().then_some(path)
    }

    fn open(bytes: &[u8]) -> (Session, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("go.rcraft");
        let session = Session::create(&project, bytes, &mut |_, _| {}, &|| false).unwrap();
        (session, dir)
    }

    #[test]
    fn go_corpus_functions_have_names() {
        let Some(path) = go_corpus() else {
            return;
        };
        let bytes = std::fs::read(path).unwrap();
        let (mut session, _dir) = open(&bytes);
        let functions = parse_pclntab(&mut session)
            .unwrap()
            .expect("pclntab not found");
        assert!(functions.len() > 20, "only {} functions", functions.len());
        let names: Vec<&str> = functions
            .iter()
            .map(|function| function.name.as_str())
            .collect();
        for expected in ["main.main", "main.helper", "main.add", "main.classify"] {
            assert!(
                names.contains(&expected),
                "missing {expected} (sample: {:?})",
                &names[..names.len().min(10)]
            );
        }
        assert!(
            names.iter().any(|name| name.starts_with("runtime.")),
            "runtime functions missing"
        );
    }

    #[test]
    fn non_go_binary_returns_none() {
        let Ok(bytes) = std::fs::read("C:/Windows/System32/notepad.exe") else {
            return;
        };
        let (mut session, _dir) = open(&bytes);
        assert!(parse_pclntab(&mut session).unwrap().is_none());
    }
}
