// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Shared primitives: addresses, ids, ranges, errors, and configuration.
//!
//! Part of ReCraft. See `docs/DESIGN.md` for the architecture.
//!
//! Everything in this crate is UI- and format-agnostic. Keep it dependency-light:
//! it is the bottom of the crate graph.

use std::fmt;
use std::ops::{Add, AddAssign, Sub};

/// A virtual address inside a program being analysed.
///
/// Addresses are intentionally a distinct type: mixing them with file offsets or
/// sizes is the most common class of bug in disassemblers.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Address(pub u64);

impl Address {
    pub const ZERO: Address = Address(0);

    #[inline]
    pub const fn new(value: u64) -> Self {
        Address(value)
    }

    #[inline]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Add a signed delta, returning `None` on underflow/overflow.
    #[inline]
    pub fn checked_add_signed(self, delta: i64) -> Option<Address> {
        self.0.checked_add_signed(delta).map(Address)
    }

    /// Forward distance to `other`, if `other` is not below `self`.
    #[inline]
    pub fn distance_to(self, other: Address) -> Option<u64> {
        other.0.checked_sub(self.0)
    }

    #[inline]
    pub const fn is_aligned(self, alignment: u64) -> bool {
        alignment != 0 && self.0 % alignment == 0
    }

    /// Align down to the previous multiple of `alignment` (must be a power of two).
    #[inline]
    pub const fn align_down(self, alignment: u64) -> Address {
        debug_assert!(alignment.is_power_of_two());
        Address(self.0 & !(alignment - 1))
    }

    /// Align up to the next multiple of `alignment` (must be a power of two).
    #[inline]
    pub const fn align_up(self, alignment: u64) -> Address {
        debug_assert!(alignment.is_power_of_two());
        Address((self.0 + alignment - 1) & !(alignment - 1))
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Address({:#x})", self.0)
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}

impl From<u64> for Address {
    #[inline]
    fn from(value: u64) -> Self {
        Address(value)
    }
}

impl From<Address> for u64 {
    #[inline]
    fn from(value: Address) -> Self {
        value.0
    }
}

impl Add<u64> for Address {
    type Output = Address;

    #[inline]
    fn add(self, rhs: u64) -> Self::Output {
        Address(self.0.wrapping_add(rhs))
    }
}

impl AddAssign<u64> for Address {
    #[inline]
    fn add_assign(&mut self, rhs: u64) {
        self.0 = self.0.wrapping_add(rhs);
    }
}

impl Sub<Address> for Address {
    type Output = u64;

    #[inline]
    fn sub(self, rhs: Address) -> Self::Output {
        self.0.wrapping_sub(rhs.0)
    }
}

/// A half-open address range: `[start, end)`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct AddressRange {
    pub start: Address,
    pub end: Address,
}

impl AddressRange {
    #[inline]
    pub const fn new(start: Address, end: Address) -> Self {
        AddressRange { start, end }
    }

    #[inline]
    pub const fn empty(at: Address) -> Self {
        AddressRange { start: at, end: at }
    }

    #[inline]
    pub fn len(self) -> u64 {
        self.end.0.saturating_sub(self.start.0)
    }

    #[inline]
    pub fn is_empty(self) -> bool {
        self.end.0 <= self.start.0
    }

    #[inline]
    pub fn contains(self, addr: Address) -> bool {
        self.start.0 <= addr.0 && addr.0 < self.end.0
    }

    #[inline]
    pub fn contains_range(self, other: AddressRange) -> bool {
        self.start.0 <= other.start.0 && other.end.0 <= self.end.0
    }

    #[inline]
    pub fn overlaps(self, other: AddressRange) -> bool {
        self.start.0 < other.end.0 && other.start.0 < self.end.0
    }

    /// Intersection of two ranges; empty if they do not overlap.
    #[inline]
    pub fn intersect(self, other: AddressRange) -> AddressRange {
        let start = Address(self.start.0.max(other.start.0));
        let end = Address(self.end.0.min(other.end.0));
        if end.0 <= start.0 {
            AddressRange::empty(start)
        } else {
            AddressRange::new(start, end)
        }
    }

    /// Truncate the range so that it is at most `max_len` bytes long.
    #[inline]
    pub fn clamp_end(mut self, max_len: u64) -> Self {
        if self.len() > max_len {
            self.end = Address(self.start.0 + max_len);
        }
        self
    }
}

impl fmt::Debug for AddressRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AddressRange({:#x}..{:#x})", self.start.0, self.end.0)
    }
}

impl fmt::Display for AddressRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#x}..{:#x}", self.start.0, self.end.0)
    }
}

/// Stable identifier of a function record in the project database.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct FuncId(pub u32);

/// Stable identifier of a type record in the project database.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct TypeId(pub u32);

/// Error type shared by all ReCraft library crates.
///
/// Library crates return this (or a domain-specific error that wraps it);
/// binaries may convert to `anyhow` at their boundary.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid executable format: {0}")]
    Format(String),

    #[error("unsupported feature: {0}")]
    Unsupported(String),

    #[error("project storage error: {0}")]
    Storage(String),

    #[error("operation cancelled")]
    Cancelled,

    #[error("internal error: {0}")]
    Internal(String),
}

impl Error {
    pub fn format(message: impl Into<String>) -> Self {
        Error::Format(message.into())
    }

    pub fn storage(message: impl Into<String>) -> Self {
        Error::Storage(message.into())
    }

    pub fn unsupported(message: impl Into<String>) -> Self {
        Error::Unsupported(message.into())
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Error::Internal(message.into())
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_display_is_hex() {
        assert_eq!(Address::new(0x401000).to_string(), "0x401000");
        assert_eq!(format!("{:?}", Address::new(0x401000)), "Address(0x401000)");
    }

    #[test]
    fn address_checked_signed_math() {
        let a = Address::new(0x1000);
        assert_eq!(a.checked_add_signed(-0x10), Some(Address::new(0xff0)));
        assert_eq!(a.checked_add_signed(0x10), Some(Address::new(0x1010)));
        assert_eq!(Address::new(8).checked_add_signed(-9), None);
    }

    #[test]
    fn address_alignment() {
        let a = Address::new(0x1234);
        assert_eq!(a.align_down(0x1000), Address::new(0x1000));
        assert_eq!(a.align_up(0x1000), Address::new(0x2000));
        assert!(Address::new(0x2000).is_aligned(0x1000));
    }

    #[test]
    fn range_basics() {
        let r = AddressRange::new(Address::new(0x1000), Address::new(0x2000));
        assert_eq!(r.len(), 0x1000);
        assert!(!r.is_empty());
        assert!(r.contains(Address::new(0x1000)));
        assert!(r.contains(Address::new(0x1fff)));
        assert!(!r.contains(Address::new(0x2000)));
        assert!(r.contains_range(AddressRange::new(
            Address::new(0x1100),
            Address::new(0x1200)
        )));
        assert!(!r.contains_range(AddressRange::new(
            Address::new(0x0f00),
            Address::new(0x1200)
        )));
    }

    #[test]
    fn range_intersection_and_overlap() {
        let a = AddressRange::new(Address::new(0x1000), Address::new(0x2000));
        let b = AddressRange::new(Address::new(0x1800), Address::new(0x3000));
        assert!(a.overlaps(b));
        assert_eq!(
            a.intersect(b),
            AddressRange::new(Address::new(0x1800), Address::new(0x2000))
        );

        let disjoint = AddressRange::new(Address::new(0x3000), Address::new(0x4000));
        assert!(!a.overlaps(disjoint));
        assert!(a.intersect(disjoint).is_empty());
    }

    #[test]
    fn range_clamp() {
        let r = AddressRange::new(Address::new(0x1000), Address::new(0x5000));
        assert_eq!(r.clamp_end(0x100).len(), 0x100);
        assert_eq!(r.clamp_end(0x1_0000), r);
    }
}
