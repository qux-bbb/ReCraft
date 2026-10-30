// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Quality metrics and golden snapshots (DESIGN §7.4).
//!
//! Two independent measurements over an analysed project:
//!
//! - **Instruction semantic coverage**: instructions whose effects were lifted into IR
//!   without falling back to an `intrinsic` (`__asm("...")`). Target: >= 95%.
//! - **Structured success rate**: functions whose L1 pseudo-C contains no `goto loc_`
//!   fallback. Target: >= 70% at M3 (M2 exit gate: >= 60%).
//!
//! Golden snapshots are normalised L1 text: every `0x...` address and `loc_...` label is
//! replaced by a placeholder so snapshots survive layout shifts, while statement and
//! control-flow shapes still diff.

use std::path::Path;

use recraft_common::{Address, Error, Result};
use recraft_core::Session;
use recraft_ir::{Lifter, Stmt};

use crate::pseudocode_l1;

/// Per-function measurements.
#[derive(Debug, Clone)]
pub struct FunctionQuality {
    pub start: Address,
    pub name: Option<String>,
    pub instructions: u64,
    pub intrinsic_instructions: u64,
    /// `goto loc_...` lines emitted by the structurer.
    pub fallbacks: usize,
    /// `loc_...` labels emitted by the structurer.
    pub labels: usize,
    /// `__asm("...")` statements left in the pseudo-C.
    pub inline_asm: usize,
    pub structured: bool,
    pub lines: usize,
}

/// Project-wide aggregate.
#[derive(Debug, Clone)]
pub struct QualityReport {
    pub functions: Vec<FunctionQuality>,
    /// Functions beyond the requested limit (not measured).
    pub skipped: usize,
    /// How often each intrinsic mnemonic was hit (diagnostic for coverage work).
    pub intrinsics: std::collections::BTreeMap<String, u64>,
}

impl QualityReport {
    pub fn total(&self) -> usize {
        self.functions.len()
    }

    pub fn structured(&self) -> usize {
        self.functions.iter().filter(|f| f.structured).count()
    }

    pub fn instructions(&self) -> u64 {
        self.functions.iter().map(|f| f.instructions).sum()
    }

    pub fn labels(&self) -> usize {
        self.functions.iter().map(|f| f.labels).sum()
    }

    pub fn inline_asm(&self) -> usize {
        self.functions.iter().map(|f| f.inline_asm).sum()
    }

    pub fn intrinsic_instructions(&self) -> u64 {
        self.functions
            .iter()
            .map(|f| f.intrinsic_instructions)
            .sum()
    }

    pub fn structured_rate(&self) -> f64 {
        if self.functions.is_empty() {
            0.0
        } else {
            self.structured() as f64 / self.functions.len() as f64
        }
    }

    pub fn coverage(&self) -> f64 {
        let instructions = self.instructions();
        if instructions == 0 {
            0.0
        } else {
            1.0 - self.intrinsic_instructions() as f64 / instructions as f64
        }
    }

    /// The `count` most frequent intrinsic mnemonics, most frequent first.
    pub fn top_intrinsics(&self, count: usize) -> Vec<(String, u64)> {
        let mut entries: Vec<(String, u64)> = self
            .intrinsics
            .iter()
            .map(|(key, value)| (key.clone(), *value))
            .collect();
        entries.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
        entries.truncate(count);
        entries
    }
}

/// Measure structural quality and semantic coverage for up to `limit` functions.
pub fn measure(session: &mut Session, limit: usize) -> Result<QualityReport> {
    let mut functions = session.functions()?;
    functions.sort_by_key(|function| function.start);
    let skipped = functions.len().saturating_sub(limit);
    functions.truncate(limit);

    let mut measured = Vec::with_capacity(functions.len());
    let mut intrinsic_histogram: std::collections::BTreeMap<String, u64> =
        std::collections::BTreeMap::new();
    for function in functions {
        let name = session.name_at(function.start)?;
        let mut lifter = Lifter::new();
        let mut instructions = 0u64;
        let mut intrinsics = 0u64;
        // Measure only instructions that analysis actually reached (CFG blocks). A linear
        // sweep of the function range would wrongly count alignment padding (`int3`),
        // jump tables and unvisited gaps as code.
        let cfg = recraft_analysis::build_cfg(session, function.start, function.end)?;
        for block in &cfg.blocks {
            for cfg_insn in &block.insns {
                let Some(insn) = session.insn_at(cfg_insn.addr)? else {
                    continue;
                };
                if insn.is_invalid() {
                    continue;
                }
                let item = lifter.lift(&insn);
                instructions += 1;
                let mut hit_intrinsic = false;
                for stmt in &item.stmts {
                    if let Stmt::Intrinsic { text } = stmt {
                        let mnemonic = text
                            .split_whitespace()
                            .next()
                            .unwrap_or(text.as_str())
                            .to_owned();
                        *intrinsic_histogram.entry(mnemonic).or_default() += 1;
                        hit_intrinsic = true;
                    }
                }
                if hit_intrinsic {
                    intrinsics += 1;
                }
            }
        }

        let code = pseudocode_l1(session, function.start)?;
        let fallbacks = code
            .lines
            .iter()
            .filter(|line| line.text.contains("goto loc_"))
            .count();
        let labels = code
            .lines
            .iter()
            .filter(|line| line.text.trim_start().starts_with("loc_"))
            .count();
        let inline_asm = code
            .lines
            .iter()
            .filter(|line| line.text.contains("__asm("))
            .count();
        measured.push(FunctionQuality {
            start: function.start,
            name,
            instructions,
            intrinsic_instructions: intrinsics,
            fallbacks,
            labels,
            inline_asm,
            structured: fallbacks == 0,
            lines: code.lines.len(),
        });
    }
    Ok(QualityReport {
        functions: measured,
        skipped,
        intrinsics: intrinsic_histogram,
    })
}

/// Result of comparing the project with a golden directory.
#[derive(Debug, Clone, Default)]
pub struct GoldenReport {
    pub checked: usize,
    /// Files present in the project but missing from the golden directory.
    pub missing: Vec<String>,
    /// `(file, expected, actual)` for different snapshots.
    pub mismatched: Vec<(String, String, String)>,
}

impl GoldenReport {
    pub fn ok(&self) -> bool {
        self.checked > 0 && self.missing.is_empty() && self.mismatched.is_empty()
    }
}

/// Write one normalised L1 snapshot per function into `dir`.
pub fn write_golden(session: &mut Session, dir: &Path) -> Result<usize> {
    std::fs::create_dir_all(dir)?;
    let functions = sorted_functions(session)?;
    let names = golden_file_names(session, &functions)?;
    let mut written = 0usize;
    for (index, function) in functions.iter().enumerate() {
        let file = dir.join(&names[index]);
        let text = golden_text(session, function.start)?;
        std::fs::write(file, text)?;
        written += 1;
    }
    Ok(written)
}

/// Compare the project against snapshots in `dir`.
pub fn check_golden(session: &mut Session, dir: &Path) -> Result<GoldenReport> {
    let functions = sorted_functions(session)?;
    let mut report = GoldenReport::default();
    if functions.is_empty() {
        return Err(Error::internal(
            "no functions: run `analyze` before checking golden snapshots",
        ));
    }
    let names = golden_file_names(session, &functions)?;
    for (index, function) in functions.iter().enumerate() {
        let file_name = names[index].clone();
        let path = dir.join(&file_name);
        let actual = golden_text(session, function.start)?;
        report.checked += 1;
        match std::fs::read_to_string(&path) {
            Ok(expected) => {
                if expected != actual {
                    report.mismatched.push((file_name, expected, actual));
                }
            }
            Err(_) => report.missing.push(file_name),
        }
    }
    Ok(report)
}

fn sorted_functions(session: &Session) -> Result<Vec<recraft_core::FunctionInfo>> {
    let mut functions = session.functions()?;
    functions.sort_by_key(|function| function.start);
    Ok(functions)
}

/// Snapshot file names keyed by symbol name so inserting functions does not renumber
/// existing files; duplicate names get an address suffix.
fn golden_file_names(
    session: &Session,
    functions: &[recraft_core::FunctionInfo],
) -> Result<Vec<String>> {
    let mut stems = Vec::with_capacity(functions.len());
    let mut counts = std::collections::BTreeMap::new();
    for function in functions {
        let name = session.name_at(function.start)?;
        let stem = sanitise_stem(name.as_deref(), function.start);
        *counts.entry(stem.clone()).or_insert(0usize) += 1;
        stems.push(stem);
    }
    Ok(stems
        .into_iter()
        .zip(functions)
        .map(|(stem, function)| {
            if counts.get(&stem).copied().unwrap_or(0) > 1 {
                format!("{stem}_{:x}.c", function.start.get())
            } else {
                format!("{stem}.c")
            }
        })
        .collect())
}

fn sanitise_stem(name: Option<&str>, start: Address) -> String {
    let stem: String = name
        .unwrap_or("")
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect();
    if stem.is_empty() {
        format!("sub_{:x}", start.get())
    } else {
        stem
    }
}

/// Normalised L1 text for one function.
pub fn golden_text(session: &mut Session, start: Address) -> Result<String> {
    let code = pseudocode_l1(session, start)?;
    let mut text = String::new();
    for line in &code.lines {
        text.push_str(&normalise_line(&line.text));
        text.push('\n');
    }
    Ok(text)
}

/// Replace concrete addresses (`0x140001234`, `loc_140001234`) with placeholders so the
/// snapshot keeps its shape across rebuilds.
fn normalise_line(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len());
    let mut index = 0usize;
    while index < chars.len() {
        if chars[index] == '0'
            && index + 1 < chars.len()
            && (chars[index + 1] == 'x' || chars[index + 1] == 'X')
        {
            let mut end = index + 2;
            while end < chars.len() && chars[end].is_ascii_hexdigit() {
                end += 1;
            }
            if end > index + 2 {
                out.push_str("0xADDR");
                index = end;
                continue;
            }
        }
        // A named global (`qword_140036648`) embeds the address in the identifier, so the
        // width has to stay while the digits are masked.
        if let Some(prefix) = DATA_PREFIXES
            .iter()
            .find(|prefix| chars[index..].starts_with(&prefix.chars().collect::<Vec<_>>()))
        {
            let mut end = index + prefix.len();
            while end < chars.len() && chars[end].is_ascii_hexdigit() {
                end += 1;
            }
            if end > index + prefix.len() {
                out.push_str(prefix);
                out.push_str("ADDR");
                index = end;
                continue;
            }
        }
        if chars[index..].starts_with(&['l', 'o', 'c', '_']) {
            let mut end = index + 4;
            while end < chars.len() && chars[end].is_ascii_hexdigit() {
                end += 1;
            }
            if end > index + 4 {
                out.push_str("loc_ADDR");
                index = end;
                continue;
            }
        }
        out.push(chars[index]);
        index += 1;
    }
    out.trim_end().to_owned()
}

/// Identifier prefixes of named global data, in masking order (longest first).
const DATA_PREFIXES: [&str; 7] = [
    "xmmword_", "dword_", "qword_", "byte_", "word_", "flt_", "dbl_",
];
/// Default directory for committed corpus snapshots.
pub fn default_golden_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/corpus")
}
#[cfg(test)]
mod tests {
    use super::*;
    use recraft_analysis::{analyze, persist};
    use std::path::PathBuf;

    fn corpus() -> Option<(Session, tempfile::TempDir)> {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../corpus/build/x64/corpus.exe");
        let bytes = std::fs::read(path).ok()?;
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("quality.rcraft");
        let mut session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let result = analyze(&mut session).unwrap();
        persist(&mut session, &result).unwrap();
        Some((session, dir))
    }

    #[test]
    fn corpus_quality_meets_m2_baseline() {
        let Some((mut session, _dir)) = corpus() else {
            return;
        };
        let report = measure(&mut session, 500).unwrap();
        assert!(report.total() >= 10, "too few functions");
        // v1 target is 95% semantics coverage (GPR, stack, cmov/imul, SSE subset).
        assert!(report.coverage() > 0.9, "coverage {:.3}", report.coverage());
        // M2 exit gate: structured success rate >= 60%.
        assert!(
            report.structured_rate() > 0.6,
            "structured {:.3}",
            report.structured_rate()
        );
    }

    #[test]
    fn normalisation_replaces_addresses() {
        assert_eq!(
            normalise_line("if (x) goto loc_140001234;"),
            "if (x) goto loc_ADDR;"
        );
        assert_eq!(
            normalise_line("*(u64*)((rsp + 0x28)) = 0x140001000;"),
            "*(u64*)((rsp + 0xADDR)) = 0xADDR;"
        );
    }

    #[test]
    fn golden_roundtrip_and_regression_detection() {
        let Some((mut session, _dir)) = corpus() else {
            return;
        };
        let golden = tempfile::tempdir().unwrap();
        let written = write_golden(&mut session, golden.path()).unwrap();
        assert_eq!(written, session.functions().unwrap().len());
        let report = check_golden(&mut session, golden.path()).unwrap();
        assert!(report.ok(), "roundtrip mismatch: {:?}", report.mismatched);

        let first = std::fs::read_dir(golden.path())
            .unwrap()
            .next()
            .expect("snapshot file")
            .unwrap()
            .path();
        let mut text = std::fs::read_to_string(&first).unwrap();
        text.push_str("// tampered\n");
        std::fs::write(&first, text).unwrap();
        let report = check_golden(&mut session, golden.path()).unwrap();
        assert!(!report.ok(), "tampering was not detected");
    }
}
