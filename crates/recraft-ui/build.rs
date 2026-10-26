// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Embed the generated application icon into the Windows executable.
//!
//! A Cargo build dependency such as `winres` would also work, but this keeps the build
//! chain dependency-free: locate the Windows SDK `rc.exe`, compile a one-line resource
//! script, and pass the `.res` to the linker.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=../../assets/recraft.ico");
    println!("cargo:rerun-if-changed=build.rs");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let icon = manifest.join("../../assets/recraft.ico");
    if !icon.is_file() {
        println!(
            "cargo:warning=application icon is missing at {}",
            icon.display()
        );
        return;
    }

    let Some(rc) = find_rc() else {
        println!("cargo:warning=Windows SDK rc.exe not found; building without an embedded icon");
        return;
    };

    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let script = out.join("recraft.rc");
    let resource = out.join("recraft.res");
    let icon = icon
        .canonicalize()
        .unwrap_or(icon)
        .to_string_lossy()
        .replace('\\', "/");
    if let Err(error) = fs::write(&script, format!("1 ICON \"{icon}\"\n")) {
        println!("cargo:warning=cannot write resource script: {error}");
        return;
    }

    let output = Command::new(&rc)
        .arg("/nologo")
        .arg("/fo")
        .arg(&resource)
        .arg(&script)
        .output();
    match output {
        Ok(output) if output.status.success() => {
            println!("cargo:rustc-link-arg-bins={}", resource.display());
        }
        Ok(output) => {
            println!(
                "cargo:warning=rc.exe failed ({}) while embedding the application icon",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        Err(error) => {
            println!("cargo:warning=cannot run rc.exe: {error}");
        }
    }
}

fn find_rc() -> Option<PathBuf> {
    if let Ok(path) = env::var("RC") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }
    if Command::new("rc.exe").arg("/?").output().is_ok() {
        return Some(PathBuf::from("rc.exe"));
    }

    let arch = match env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
        Ok("x86") => "x86",
        Ok("aarch64") => "arm64",
        _ => "x64",
    };
    let kits = env::var_os("ProgramFiles(x86)")?;
    let root = Path::new(&kits).join("Windows Kits/10/bin");
    let mut versions: Vec<PathBuf> = fs::read_dir(&root)
        .ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    versions.sort();
    versions
        .into_iter()
        .rev()
        .map(|version| version.join(arch).join("rc.exe"))
        .find(|candidate| candidate.is_file())
}
