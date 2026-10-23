// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Per-byte analysis flags, stored as 4 bits per image byte.
//!
//! Layout: one database page per 64 KiB of image (`PAGE_BYTES`), 32 KiB of packed
//! nibbles (`data` blob). Nibbles encode:
//!
//! | bits | meaning |
//! |------|---------|
//! | 0-1  | kind: 0 unknown, 1 code, 2 data |
//! | 2    | head: this byte starts an instruction |
//!
//! The store keeps an LRU page cache; writes go straight to the database and update
//! the cache in place, so pages never need to be flushed by callers.

use std::collections::{HashMap, VecDeque};

use recraft_common::{Address, AddressRange, Error, Result};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

/// Image bytes covered by one flags page.
pub const PAGE_BYTES: u64 = 0x1_0000;
const PAGE_FLAG_BYTES: usize = PAGE_BYTES as usize / 2;

/// Code/data classification of a byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[repr(u8)]
pub enum CodeData {
    #[default]
    Unknown = 0,
    Code = 1,
    Data = 2,
}

impl CodeData {
    #[inline]
    const fn from_bits(bits: u8) -> Self {
        match bits & 0b11 {
            1 => CodeData::Code,
            2 => CodeData::Data,
            _ => CodeData::Unknown,
        }
    }
}

/// Flags for one image byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Flags {
    pub kind: CodeData,
    pub head: bool,
}

impl Flags {
    pub const UNKNOWN: Flags = Flags {
        kind: CodeData::Unknown,
        head: false,
    };

    #[inline]
    pub const fn code() -> Self {
        Flags {
            kind: CodeData::Code,
            head: true,
        }
    }

    #[inline]
    pub const fn code_tail() -> Self {
        Flags {
            kind: CodeData::Code,
            head: false,
        }
    }

    #[inline]
    pub const fn data() -> Self {
        Flags {
            kind: CodeData::Data,
            head: false,
        }
    }

    #[inline]
    pub const fn is_code(self) -> bool {
        matches!(self.kind, CodeData::Code)
    }

    #[inline]
    pub const fn is_data(self) -> bool {
        matches!(self.kind, CodeData::Data)
    }

    #[inline]
    const fn to_nibble(self) -> u8 {
        (self.kind as u8) | if self.head { 0b0100 } else { 0 }
    }

    #[inline]
    const fn from_nibble(nibble: u8) -> Self {
        Flags {
            kind: CodeData::from_bits(nibble),
            head: nibble & 0b0100 != 0,
        }
    }
}

/// Page-cached access to the `flags_page` table.
pub struct FlagsStore {
    pages: HashMap<u64, Box<[u8]>>,
    order: VecDeque<u64>,
    capacity: usize,
}

impl FlagsStore {
    /// `capacity` is in pages (1 page = 64 KiB of image = 32 KiB of flags).
    pub fn new(capacity: usize) -> Self {
        Self {
            pages: HashMap::new(),
            order: VecDeque::new(),
            capacity: capacity.max(1),
        }
    }

    #[inline]
    fn split(addr: Address) -> (u64, usize) {
        let page_no = addr.get() / PAGE_BYTES;
        let index = (addr.get() % PAGE_BYTES) as usize;
        (page_no, index)
    }

    fn touch(&mut self, page_no: u64) {
        if let Some(position) = self.order.iter().position(|&key| key == page_no) {
            if position + 1 != self.order.len() {
                self.order.remove(position);
                self.order.push_back(page_no);
            }
        }
    }

    fn ensure_page(&mut self, conn: &Connection, page_no: u64) -> Result<()> {
        if self.pages.contains_key(&page_no) {
            self.touch(page_no);
            return Ok(());
        }
        let data: Option<Vec<u8>> = conn
            .query_row(
                "SELECT data FROM flags_page WHERE page_no = ?1",
                [page_no as i64],
                |row| row.get(0),
            )
            .optional()
            .map_err(db_err)?;
        let page: Box<[u8]> = match data {
            Some(data) if data.len() == PAGE_FLAG_BYTES => data.into_boxed_slice(),
            _ => vec![0u8; PAGE_FLAG_BYTES].into_boxed_slice(),
        };
        while self.order.len() >= self.capacity {
            if let Some(evicted) = self.order.pop_front() {
                self.pages.remove(&evicted);
            } else {
                break;
            }
        }
        self.pages.insert(page_no, page);
        self.order.push_back(page_no);
        Ok(())
    }

    /// Read the flags of one byte (unknown if never written).
    pub fn get(&mut self, conn: &Connection, addr: Address) -> Result<Flags> {
        let (page_no, index) = Self::split(addr);
        self.ensure_page(conn, page_no)?;
        let page = self
            .pages
            .get(&page_no)
            .ok_or_else(|| Error::internal("flags page vanished from cache"))?;
        let byte = page[index / 2];
        let nibble = if index % 2 == 0 {
            byte & 0x0f
        } else {
            byte >> 4
        };
        Ok(Flags::from_nibble(nibble))
    }

    /// Read `range.len()` flags (used to build the inverse of a code/data command).
    ///
    /// Callers must bound the length (commands cap it at one page).
    pub fn snapshot(&mut self, conn: &Connection, range: AddressRange) -> Result<Vec<Flags>> {
        let mut flags = Vec::with_capacity(range.len() as usize);
        let mut addr = range.start;
        while addr < range.end {
            flags.push(self.get(conn, addr)?);
            addr = Address::new(addr.get() + 1);
        }
        Ok(flags)
    }

    /// Fill `[start, start+flags.len())` with the given flags.
    pub fn write(&mut self, conn: &Connection, start: Address, flags: &[Flags]) -> Result<()> {
        if flags.is_empty() {
            return Ok(());
        }
        let end = start.get() + flags.len() as u64;
        let mut page_no = start.get() / PAGE_BYTES;
        while page_no * PAGE_BYTES < end {
            let page_start = page_no * PAGE_BYTES;
            let page_end = page_start + PAGE_BYTES;
            let write_start = start.get().max(page_start);
            let write_end = end.min(page_end);
            if write_end > write_start {
                self.ensure_page(conn, page_no)?;
                let offset = (write_start - page_start) as usize;
                {
                    let page = self
                        .pages
                        .get_mut(&page_no)
                        .ok_or_else(|| Error::internal("flags page vanished from cache"))?;
                    for local in 0..(write_end - write_start) as usize {
                        let index = offset + local;
                        let nibble =
                            flags[(write_start - start.get()) as usize + local].to_nibble();
                        let byte_index = index / 2;
                        if index % 2 == 0 {
                            page[byte_index] = (page[byte_index] & 0xf0) | nibble;
                        } else {
                            page[byte_index] = (page[byte_index] & 0x0f) | (nibble << 4);
                        }
                    }
                }
                let page = self
                    .pages
                    .get(&page_no)
                    .ok_or_else(|| Error::internal("flags page vanished from cache"))?;
                conn.execute(
                    "INSERT INTO flags_page(page_no, data) VALUES (?1, ?2)
                     ON CONFLICT(page_no) DO UPDATE SET data = excluded.data",
                    params![page_no as i64, &page[..]],
                )
                .map_err(db_err)?;
            }
            page_no += 1;
        }
        Ok(())
    }

    /// Fill a range with one flag value; the first byte gets `head = true` for code.
    pub fn fill(
        &mut self,
        conn: &Connection,
        start: Address,
        len: u64,
        kind: CodeData,
    ) -> Result<()> {
        if len == 0 {
            return Ok(());
        }
        let flags = match kind {
            CodeData::Code => {
                let mut flags = vec![Flags::code_tail(); len as usize];
                flags[0] = Flags::code();
                flags
            }
            CodeData::Data => vec![Flags::data(); len as usize],
            CodeData::Unknown => vec![Flags::UNKNOWN; len as usize],
        };
        self.write(conn, start, &flags)
    }

    pub fn invalidate(&mut self, range: AddressRange) {
        if range.is_empty() {
            return;
        }
        let first = range.start.get() / PAGE_BYTES;
        let last = (range.end.get().saturating_sub(1)) / PAGE_BYTES;
        for page_no in first..=last {
            if self.pages.remove(&page_no).is_some()
                && let Some(position) = self.order.iter().position(|&key| key == page_no)
            {
                self.order.remove(position);
            }
        }
    }

    pub fn clear(&mut self) {
        self.pages.clear();
        self.order.clear();
    }

    pub fn cached_pages(&self) -> usize {
        self.pages.len()
    }
}

fn db_err(error: rusqlite::Error) -> Error {
    Error::storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use recraft_storage::schema;

    fn conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        schema::initialize(&conn).unwrap();
        conn
    }

    #[test]
    fn defaults_to_unknown_and_roundtrips() {
        let conn = conn();
        let mut store = FlagsStore::new(4);
        let addr = Address::new(0x1000);
        assert_eq!(store.get(&conn, addr).unwrap(), Flags::UNKNOWN);

        store.fill(&conn, addr, 3, CodeData::Code).unwrap();
        assert_eq!(store.get(&conn, addr).unwrap(), Flags::code());
        assert_eq!(
            store.get(&conn, Address::new(0x1001)).unwrap(),
            Flags::code_tail()
        );
        assert_eq!(
            store.get(&conn, Address::new(0x1002)).unwrap(),
            Flags::code_tail()
        );
        assert_eq!(
            store.get(&conn, Address::new(0x1003)).unwrap(),
            Flags::UNKNOWN
        );

        // A fresh store must see the persisted pages.
        let mut store2 = FlagsStore::new(4);
        assert_eq!(store2.get(&conn, addr).unwrap(), Flags::code());
    }

    #[test]
    fn writes_across_page_boundaries() {
        let conn = conn();
        let mut store = FlagsStore::new(4);
        let start = Address::new(PAGE_BYTES - 2);
        store.fill(&conn, start, 6, CodeData::Code).unwrap();
        for offset in 0..6u64 {
            let flags = store
                .get(&conn, Address::new(start.get() + offset))
                .unwrap();
            assert!(flags.is_code(), "offset {offset} should be code");
            assert_eq!(flags.head, offset == 0);
        }
        assert_eq!(store.cached_pages(), 2);
    }

    #[test]
    fn snapshot_restore_roundtrip() {
        let conn = conn();
        let mut store = FlagsStore::new(4);
        let range = AddressRange::new(Address::new(0x2000), Address::new(0x2008));
        store
            .fill(&conn, range.start, range.len(), CodeData::Code)
            .unwrap();
        let before = store.snapshot(&conn, range).unwrap();
        assert_eq!(before.len(), 8);

        store
            .fill(&conn, range.start, range.len(), CodeData::Data)
            .unwrap();
        assert!(store.get(&conn, range.start).unwrap().is_data());

        store.write(&conn, range.start, &before).unwrap();
        assert!(store.get(&conn, range.start).unwrap().is_code());
        assert!(store.get(&conn, Address::new(0x2001)).unwrap().is_code());
    }

    #[test]
    fn cache_eviction_keeps_working() {
        let conn = conn();
        let mut store = FlagsStore::new(2);
        for page in 0..5u64 {
            store
                .fill(&conn, Address::new(page * PAGE_BYTES), 1, CodeData::Data)
                .unwrap();
        }
        assert!(store.cached_pages() <= 2);
        // Evicted pages are reloaded from the database.
        assert!(store.get(&conn, Address::new(0)).unwrap().is_data());
        assert!(
            store
                .get(&conn, Address::new(4 * PAGE_BYTES))
                .unwrap()
                .is_data()
        );
    }
}
