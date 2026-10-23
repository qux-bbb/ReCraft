// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Deterministic malformed-input regression for the PE loader (DESIGN §11).
//!
//! `cargo-fuzz` needs nightly and is run offline, so this test approximates it with a
//! fixed xorshift PRNG: random blobs, truncations of a real image and header mutations
//! must never panic. Load failures are fine; panics are not. When the mutations still
//! load, every public accessor is exercised to catch panics deeper in the pipeline.

use std::path::Path;

fn notepad_bytes() -> Option<Vec<u8>> {
    std::fs::read(Path::new("C:/Windows/System32/notepad.exe")).ok()
}

/// xorshift64* — deterministic and dependency-free.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, bound: usize) -> usize {
        if bound == 0 {
            0
        } else {
            (self.next() % bound as u64) as usize
        }
    }
}

fn exercise(image: &recraft_format::PeImage) {
    // Public accessors must not panic even for a partly malformed image.
    let _ = image.segment_containing(image.image_base);
    let _ = image.segment_containing(recraft_common::Address::new(image.size_of_image));
    let _ = image.rva_to_file_offset(0);
    let _ = image.rva_to_file_offset(u32::MAX);
    for section in &image.sections {
        let _ = section.is_readable();
        let _ = section.is_writable();
        let _ = section.is_executable();
        let _ = section.contains_rva(section.rva);
    }
    let _ = image.imports.len();
    let _ = image.exports.len();
    let _ = image.tls_callbacks.len();
    let _ = image.pdata_functions.len();
}

#[test]
fn random_blobs_never_panic() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    for _ in 0..500 {
        let len = rng.below(4096) + 1;
        let mut bytes = vec![0u8; len];
        for byte in &mut bytes {
            *byte = rng.next() as u8;
        }
        if let Ok(image) = recraft_format::load(&bytes) {
            exercise(&image);
        }
    }
}

#[test]
fn truncated_images_never_panic() {
    let Some(valid) = notepad_bytes() else {
        return;
    };
    let mut rng = Rng(0xDEAD_BEEF_CAFE_F00D);
    for _ in 0..400 {
        let len = rng.below(valid.len());
        let bytes = &valid[..len];
        if let Ok(image) = recraft_format::load(bytes) {
            exercise(&image);
        }
    }
}

#[test]
fn mutated_headers_never_panic() {
    let Some(valid) = notepad_bytes() else {
        return;
    };
    let mut rng = Rng(0x1234_5678_9ABC_DEF0);
    for _ in 0..400 {
        let mut bytes = valid.clone();
        let flips = rng.below(4) + 1;
        for _ in 0..flips {
            // Focus on the DOS/NT headers and the section table.
            let index = rng.below(0x400.min(bytes.len()));
            bytes[index] ^= 1u8 << (rng.below(8) as u32);
        }
        if let Ok(image) = recraft_format::load(&bytes) {
            exercise(&image);
        }
    }
    // Truncating at the DOS header/e_lfanew boundary must also be safe.
    for len in [0usize, 1, 2, 0x3c, 0x3f, 0x40, 0x100] {
        if len <= valid.len() {
            let _ = recraft_format::load(&valid[..len]);
        }
    }
}
