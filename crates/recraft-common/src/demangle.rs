// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Symbol demangling for loader names (MSVC C++, Rust legacy and v0).

use msvc_demangler::DemangleFlags;

/// Demangle a decorated symbol name.
///
/// Returns `None` when the name is not recognized as a mangled name (plain C names,
/// ordinals, Go names, ...), so callers can keep the original text.
pub fn demangle(name: &str) -> Option<String> {
    let name = name.trim();
    if name.is_empty() {
        return None;
    }

    // MSVC C++: `?name@scope@@...`. Constructors/operators start with `??`.
    if name.starts_with('?') {
        if let Ok(demangled) = msvc_demangler::demangle(name, DemangleFlags::NO_FUNCTION_RETURNS) {
            let demangled = demangled.trim().to_owned();
            if !demangled.is_empty() && demangled != name {
                return Some(demangled);
            }
        }
        return None;
    }

    // Rust: legacy `_ZN...` and v0 `_R...` (Mach-O adds a leading underscore, and
    // Windows objects may use `__ZN`/`__R` prefixes).
    let rust_name = name
        .strip_prefix("__")
        .or_else(|| name.strip_prefix('_'))
        .unwrap_or(name);
    if rust_name.starts_with("ZN") || rust_name.starts_with('R') {
        if let Ok(demangled) = rustc_demangle::try_demangle(name) {
            // `{:#}` drops the legacy hash suffix (`::h1234...`).
            let text = format!("{demangled:#}");
            if text != name {
                return Some(text);
            }
        }
        // Some producers strip the leading underscore before storing the symbol.
        if name != rust_name
            && let Ok(demangled) = rustc_demangle::try_demangle(rust_name)
        {
            let text = format!("{demangled:#}");
            if text != name {
                return Some(text);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demangles_msvc_cpp() {
        let name = "?get@DemoTarget@recraft@@QEBAHXZ";
        let demangled = demangle(name).expect("MSVC name not demangled");
        assert!(
            demangled.contains("DemoTarget") && demangled.contains("get"),
            "{demangled}"
        );
        assert!(!demangled.starts_with('?'), "{demangled}");

        // Constructors use the `??0` form and must not be confused with `??_`
        // RTTI metadata.
        let constructor = "??0DemoTarget@recraft@@QEAA@XZ";
        let demangled = demangle(constructor).expect("constructor not demangled");
        assert!(demangled.contains("DemoTarget"), "{demangled}");

        // Overloads must stay distinguishable (parameter types are included).
        let int_version = demangle("?overloaded@recraft@@YAHH@Z").unwrap();
        let double_version = demangle("?overloaded@recraft@@YAHN@Z").unwrap();
        assert_ne!(int_version, double_version);
    }

    #[test]
    fn demangles_rust_legacy_and_v0() {
        let legacy = "_ZN4core3fmt3num3imp7fmt_u6417h7c55e9f9ba8b2e3aE";
        let demangled = demangle(legacy).expect("legacy Rust name not demangled");
        assert!(demangled.contains("core::fmt"), "{demangled}");

        let v0 = "_RNvCs15kBYyAo9fc_7mycrate4main";
        let demangled = demangle(v0).expect("v0 Rust name not demangled");
        assert!(demangled.contains("mycrate::main"), "{demangled}");
    }

    #[test]
    fn leaves_plain_names_alone() {
        for name in ["CreateFileW", "main.main", "start", "#413"] {
            assert_eq!(demangle(name), None, "{name} should not be demangled");
        }
    }
}
