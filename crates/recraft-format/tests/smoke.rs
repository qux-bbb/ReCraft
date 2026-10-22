// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Smoke test against real Windows binaries when they are present. On other
//! platforms the test is a no-op, so `cargo test` stays portable.

use std::path::Path;

use recraft_common::Address;
use recraft_format::load;

fn check(path: &str) {
    if !Path::new(path).exists() {
        eprintln!("skipping {path}: not present");
        return;
    }
    let bytes = std::fs::read(path).unwrap_or_else(|error| panic!("read {path}: {error}"));
    let image = load(&bytes).unwrap_or_else(|error| panic!("parse {path}: {error}"));

    let imported_functions: usize = image
        .imports
        .iter()
        .map(|module| module.functions.len())
        .sum();
    eprintln!(
        "{path}: file_size={} is_64={} image_base={:#x} entry={:#x} sections={} imports={} ({} modules, {} functions) exports={} pdata={} tls={} overlay={:?}",
        image.file_size,
        image.is_64,
        image.image_base.get(),
        image.entry_point.get(),
        image.sections.len(),
        image.imports.len(),
        image
            .imports
            .iter()
            .filter(|module| !module.delayed)
            .count(),
        imported_functions,
        image.exports.len(),
        image.pdata_functions.len(),
        image.tls_callbacks.len(),
        image.overlay_file_offset,
    );

    assert!(image.is_64, "{path} should be a 64-bit PE");
    assert!(!image.sections.is_empty(), "{path} has no sections");
    assert!(!image.imports.is_empty(), "{path} has no imports");
    assert_ne!(
        image.entry_point,
        Address::ZERO,
        "{path} has no entry point"
    );
}

#[test]
fn parses_windows_system_binaries() {
    check(r"C:\Windows\System32\notepad.exe");
    check(r"C:\Windows\System32\kernel32.dll");
}
