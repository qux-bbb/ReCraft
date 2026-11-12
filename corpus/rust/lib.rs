// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Rust evaluation corpus: the shapes a real Rust binary shows and that the C/Go
//! corpora cannot produce.
//!
//! Built as a `cdylib` by `corpus/build-rust.ps1` (PE, x64) and by
//! `corpus/build-linux.sh` (ELF, x86-64). The exported `rust_*` wrappers give the PE
//! image an export table to take names from; the ELF build additionally keeps
//! `.symtab`, so the mangled `corpus_rust::…` names exercise legacy Itanium
//! demangling end to end (`rustc_demangle`, hash suffix dropped).
//!
//! What each group is for:
//! - plain functions: the same control-flow shapes as `corpus/c/functions.c`, so the
//!   two languages can be compared at the same structure/coverage floor;
//! - `panic!` / `Option` / `Result`: the panic + unwinding machinery (`_Unwind_*`,
//!   `core::panicking::*`, and `.pdata`/`.eh_frame` function ranges);
//! - trait objects and closures: vtables, drop glue and indirect calls through
//!   function pointers stored in data;
//! - `Vec` / `String` / `format!`: heap allocation and `core::fmt` monomorphizations;
//! - generics: two instantiations of the same function (distinct mangled names at
//!   distinct addresses).
//!
//! Every non-exported function is reachable from `rust_entry`, otherwise the linker's
//! section GC drops it from the `cdylib`.

#![allow(clippy::missing_safety_doc)]

/// A string constant, so the strings view has something to find.
static GREETING: &str = "ReCraft rust corpus";

/// Data-side function table (indirect calls without a register load).
static HANDLERS: [fn(i64) -> i64; 4] = [handler_a, handler_b, handler_c, handler_d];

#[inline(never)]
fn handler_a(value: i64) -> i64 {
    value.wrapping_add(1)
}

#[inline(never)]
fn handler_b(value: i64) -> i64 {
    value.wrapping_mul(3)
}

#[inline(never)]
fn handler_c(value: i64) -> i64 {
    value ^ 0x55
}

#[inline(never)]
fn handler_d(value: i64) -> i64 {
    value.wrapping_sub(7)
}

#[inline(never)]
pub fn add_leaf(a: i64, b: i64) -> i64 {
    a.wrapping_add(b)
}

#[inline(never)]
pub fn branchy(x: i64) -> i64 {
    if x > 10 {
        x.wrapping_mul(2)
    } else if x < -5 {
        x.wrapping_neg()
    } else {
        0
    }
}

#[inline(never)]
pub fn loopy(n: i64) -> i64 {
    let mut sum = 0i64;
    let mut i = 0i64;
    while i < n {
        i += 1;
        if i & 1 == 0 {
            continue;
        }
        sum = sum.wrapping_add(i);
    }
    sum
}

#[inline(never)]
pub fn nested_loops(n: i64) -> i64 {
    let mut sum = 0i64;
    for i in 0..n {
        for j in 0..n {
            sum = sum.wrapping_add(i.wrapping_mul(j));
        }
    }
    sum
}

/// Large enough that rustc emits a jump table instead of a comparison chain.
#[inline(never)]
pub fn switcher_big(x: i64) -> i64 {
    match x {
        0 => 100,
        1 => 101,
        2 => 102,
        3 => 103,
        4 => 104,
        5 => 105,
        6 => 106,
        7 => 107,
        8 => 108,
        9 => 109,
        10 => 110,
        11 => 111,
        12 => 112,
        13 => 113,
        14 => 114,
        15 => 115,
        16 => 116,
        17 => 117,
        18 => 118,
        19 => 119,
        _ => -1,
    }
}

#[inline(never)]
pub fn recursive(n: u64) -> u64 {
    if n <= 1 {
        return 1;
    }
    n.wrapping_mul(recursive(n - 1))
}

#[inline(never)]
pub fn dispatch(index: usize, value: i64) -> i64 {
    HANDLERS[index & 3](value)
}

#[derive(Clone, Copy)]
pub struct Point {
    pub x: i64,
    pub y: i64,
    pub tag: u32,
}

#[inline(never)]
pub fn point_sum(point: &Point) -> i64 {
    point
        .x
        .wrapping_add(point.y)
        .wrapping_add(point.tag as i64)
}

#[inline(never)]
pub fn vec_sum(values: &[i64]) -> i64 {
    let mut sum = 0i64;
    for value in values {
        sum = sum.wrapping_add(*value);
    }
    sum
}

pub trait Shape {
    fn area(&self) -> i64;
}

pub struct Rect {
    width: i64,
    height: i64,
}

pub struct Square {
    side: i64,
}

impl Shape for Rect {
    #[inline(never)]
    fn area(&self) -> i64 {
        self.width.wrapping_mul(self.height)
    }
}

impl Shape for Square {
    #[inline(never)]
    fn area(&self) -> i64 {
        self.side.wrapping_mul(self.side)
    }
}

/// Dynamic dispatch through two vtables.
#[inline(never)]
pub fn total_area(shapes: &[&dyn Shape]) -> i64 {
    let mut sum = 0i64;
    for shape in shapes {
        sum = sum.wrapping_add(shape.area());
    }
    sum
}

/// A capturing closure (the environment is passed as a hidden struct pointer).
#[inline(never)]
pub fn closure_fold(seed: i64, salt: i64) -> i64 {
    let add = |value: i64| value.wrapping_mul(salt).wrapping_add(seed);
    (0..8).map(add).fold(0i64, |acc, value| acc ^ value)
}

/// Panic path: pulls in `core::panicking::panic*` and the unwinder.
#[inline(never)]
pub fn checked_divide(a: i64, b: i64) -> i64 {
    if b == 0 {
        panic!("divide by zero in the Rust corpus");
    }
    a / b
}

/// `Option` + `Result` propagation, i.e. the `?`-shaped control flow.
#[inline(never)]
pub fn parse_number(text: &str) -> Option<i64> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    trimmed.parse::<i64>().ok()
}

/// `core::fmt` monomorphization behind `format!`.
#[inline(never)]
pub fn describe(value: i64) -> String {
    format!("{} = {}", GREETING, value)
}

/// Two monomorphizations of the same generic (`max_of::<i64>`, `max_of::<u32>`).
#[inline(never)]
pub fn max_of<T: PartialOrd + Copy>(a: T, b: T) -> T {
    if a > b { a } else { b }
}

// ---------------------------------------------------------------------------
// Exported C ABI wrappers: names in the PE export table, and the roots that keep
// every function above alive in the image.
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn rust_add_leaf(a: i64, b: i64) -> i64 {
    add_leaf(a, b)
}

#[no_mangle]
pub extern "C" fn rust_shapes(count: i64) -> i64 {
    let rect = Rect {
        width: 3,
        height: 7,
    };
    let square = Square { side: 5 };
    let shapes: [&dyn Shape; 2] = [&rect, &square];
    total_area(&shapes)
        .wrapping_mul(count.max(1))
        .wrapping_add(total_area(&shapes))
}

#[no_mangle]
pub extern "C" fn rust_describe(value: i64) -> i64 {
    describe(value).len() as i64
}

#[no_mangle]
pub extern "C" fn rust_entry(mode: i64, value: i64) -> i64 {
    let point = Point {
        x: value,
        y: value.wrapping_mul(2),
        tag: 7,
    };
    let numbers = [value, value.wrapping_add(1), value.wrapping_sub(1)];
    let mut acc = add_leaf(mode, value)
        .wrapping_add(branchy(value))
        .wrapping_add(loopy(value & 0xff))
        .wrapping_add(nested_loops(value & 0x0f))
        .wrapping_add(switcher_big(value))
        .wrapping_add(recursive((value & 0x0f) as u64) as i64)
        .wrapping_add(dispatch(value as usize, mode))
        .wrapping_add(point_sum(&point))
        .wrapping_add(vec_sum(&numbers))
        .wrapping_add(closure_fold(value, mode))
        .wrapping_add(max_of(value, mode))
        .wrapping_add(max_of(value as u32, mode as u32) as i64);
    acc = acc.wrapping_add(match parse_number("42") {
        Some(number) => number,
        None => 0,
    });
    if mode != 0 {
        acc = acc.wrapping_add(checked_divide(value, mode));
    }
    acc.wrapping_add(describe(value).len() as i64)
}

/// Kept for parity with the C corpus: an exported banner string.
#[no_mangle]
pub extern "C" fn rust_banner() -> *const u8 {
    GREETING.as_ptr()
}
