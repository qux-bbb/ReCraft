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

use std::collections::BTreeSet;
use std::path::Path;

use recraft_common::{Address, Error, Result};
use recraft_core::Session;
use recraft_ir::{Lifter, Stmt};

use crate::{pseudocode_l1, pseudocode_l2};

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
    /// `switch (...)` statements in the pseudo-C (jump tables and recovered dispatch ladders).
    pub switches: usize,
    /// Deepest statement indentation, in columns (multiples of four).
    pub max_indent: usize,
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

    /// How many functions carry 0, 1, 2 … fallback jumps, so the shape of the remaining work is
    /// visible without listing every function.
    pub fn fallback_histogram(&self) -> std::collections::BTreeMap<usize, usize> {
        let mut histogram = std::collections::BTreeMap::new();
        for function in &self.functions {
            *histogram.entry(function.fallbacks).or_insert(0usize) += 1;
        }
        histogram
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

    pub fn switches(&self) -> usize {
        self.functions.iter().map(|f| f.switches).sum()
    }

    /// Deepest nesting of any measured function, in levels (four columns each).
    pub fn max_nesting(&self) -> usize {
        self.functions
            .iter()
            .map(|f| f.max_indent / 4)
            .max()
            .unwrap_or(0)
    }

    /// Functions nested eight levels or deeper: the ones whose control flow is still hard to
    /// read, without the single worst outlier hiding the rest.
    pub fn deeply_nested(&self) -> usize {
        self.functions.iter().filter(|f| f.max_indent >= 32).count()
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
    // A limit of zero means "measure everything": a sample with more functions than the default
    // cap (the Go binary has 3261 against a 2000 default) otherwise needs a magic large number.
    let limit = if limit == 0 {
        functions.len()
    } else {
        limit.min(functions.len())
    };
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
        let switches = code
            .lines
            .iter()
            .filter(|line| line.text.trim_start().starts_with("switch ("))
            .count();
        // Indentation is the shape of the control flow: a `switch` keeps a dispatch at one
        // level while a nested `if` ladder adds one level per case.
        let max_indent = code
            .lines
            .iter()
            .map(|line| line.text.len() - line.text.trim_start().len())
            .max()
            .unwrap_or(0);
        measured.push(FunctionQuality {
            start: function.start,
            name,
            instructions,
            intrinsic_instructions: intrinsics,
            fallbacks,
            labels,
            inline_asm,
            switches,
            max_indent,
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

/// One way the emitted pseudo-C can be malformed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LintKind {
    /// A `goto loc_…` without a matching label definition.
    GotoWithoutLabel,
    /// The same label defined twice in one function.
    DuplicateLabel,
    /// Unbalanced braces outside string literals.
    UnbalancedBraces,
    /// `if ()` or `while ()` with no condition.
    EmptyCondition,
    /// The body uses an argument the signature does not declare.
    UndeclaredArgument,
    /// A `loc_` label printed above the signature line.
    LabelAboveSignature,
    /// A `case`/`default` label that is not one level inside its `switch`.
    MisplacedCaseLabel,
}

impl LintKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            LintKind::GotoWithoutLabel => "goto without label",
            LintKind::DuplicateLabel => "duplicate label",
            LintKind::UnbalancedBraces => "unbalanced braces",
            LintKind::EmptyCondition => "empty condition",
            LintKind::UndeclaredArgument => "undeclared argument",
            LintKind::LabelAboveSignature => "label above signature",
            LintKind::MisplacedCaseLabel => "misplaced case label",
        }
    }
}

/// A lint finding: which rule, in which function.
#[derive(Debug, Clone)]
pub struct LintFinding {
    pub start: Address,
    pub name: Option<String>,
    pub kind: LintKind,
    pub detail: String,
}

/// Aggregate of [`lint`].
#[derive(Debug, Default)]
pub struct LintReport {
    pub functions: usize,
    /// Functions whose L1 and L2 text were both checked.
    pub checked: usize,
    pub findings: Vec<LintFinding>,
}

impl LintReport {
    pub fn ok(&self) -> bool {
        self.findings.is_empty()
    }

    /// Findings of one rule.
    pub fn count(&self, kind: LintKind) -> usize {
        self.findings.iter().filter(|f| f.kind == kind).count()
    }

    /// How many distinct functions have at least one finding.
    pub fn affected(&self) -> usize {
        let mut starts: Vec<u64> = self.findings.iter().map(|f| f.start.get()).collect();
        starts.sort_unstable();
        starts.dedup();
        starts.len()
    }
}

/// Which pseudo-code levels [`lint`] checks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LintLevel {
    /// Both levels (the default).
    #[default]
    Both,
    L1,
    L2,
}

/// Check the structural invariants every emitted function must hold, on both L1 and L2 text.
///
/// These are the properties the text passes promise: a label for every `goto`, one definition per
/// label, balanced braces outside string literals, no conditionless branch, a signature that
/// declares every argument the body uses, and no label above the signature. A violation is a
/// decompiler bug, not a style question, so `recraft lint` exits non-zero when one is found.
pub fn lint(session: &mut Session, limit: usize, level: LintLevel) -> Result<LintReport> {
    let mut functions = session.functions()?;
    functions.sort_by_key(|function| function.start);
    let total = functions.len();
    functions.truncate(limit);
    if total == 0 {
        return Err(Error::invalid_argument(
            "no functions: run `analyze` before linting the output",
        ));
    }
    let mut report = LintReport {
        functions: total,
        checked: 0,
        findings: Vec::new(),
    };
    for function in functions {
        let name = session.name_at(function.start)?;
        let mut texts = Vec::with_capacity(2);
        if level != LintLevel::L2 {
            texts.push(pseudocode_l1(session, function.start)?.text());
        }
        if level != LintLevel::L1 {
            texts.push(pseudocode_l2(session, function.start)?.text());
        }
        for text in texts {
            report.checked += 1;
            for (kind, detail) in lint_text(&text) {
                report.findings.push(LintFinding {
                    start: function.start,
                    name: name.clone(),
                    kind,
                    detail,
                });
            }
        }
    }
    // Stable order so a truncated report always shows the lowest addresses.
    report
        .findings
        .sort_by_key(|finding| (finding.start.get(), finding.kind, finding.detail.clone()));
    if report.checked == 0 {
        // `--limit 0` would otherwise look like a clean run that inspected nothing.
        return Err(Error::invalid_argument(
            "no texts checked: raise --limit above 0",
        ));
    }
    Ok(report)
}

/// The invariant violations in one rendered function.
fn lint_text(text: &str) -> Vec<(LintKind, String)> {
    let mut findings = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    let labels: Vec<String> = lines
        .iter()
        .filter_map(|line| {
            let trimmed = line.trim();
            let name = trimmed.strip_suffix(':')?;
            name.starts_with("loc_").then(|| name.to_owned())
        })
        .collect();
    for (index, line) in lines.iter().enumerate() {
        for target in goto_targets(line) {
            if !labels.iter().any(|label| label == &target) {
                findings.push((LintKind::GotoWithoutLabel, target));
            }
        }
        if line.contains("if ()") || line.contains("while ()") {
            findings.push((LintKind::EmptyCondition, (*line).trim().to_owned()));
        }
        let _ = index;
    }
    let mut reported: BTreeSet<String> = BTreeSet::new();
    for label in &labels {
        if reported.contains(label) {
            continue;
        }
        if labels
            .iter()
            .filter(|candidate| *candidate == label)
            .count()
            > 1
        {
            findings.push((LintKind::DuplicateLabel, label.clone()));
            reported.insert(label.clone());
        }
    }
    // Braces outside string literals.
    let mut depth = 0isize;
    let mut quoted = false;
    let mut escaped = false;
    for ch in text.chars() {
        match ch {
            '"' if !escaped => quoted = !quoted,
            '\\' if quoted => escaped = !escaped,
            '{' if !quoted => depth += 1,
            '}' if !quoted => depth -= 1,
            _ => escaped = false,
        }
    }
    if depth != 0 {
        findings.push((LintKind::UnbalancedBraces, format!("depth {depth}")));
    }
    findings.extend(misplaced_case_labels(&lines));
    // The signature must declare every argument the body uses.
    let signature = lines
        .iter()
        .position(|line| line.trim_end().ends_with('{') && !line.trim_start().starts_with("//"));
    if let Some(signature) = signature {
        for line in lines.iter().take(signature) {
            if line.trim().starts_with("loc_") {
                findings.push((LintKind::LabelAboveSignature, (*line).trim().to_owned()));
            }
        }
        // The parameter list is the last `(...)` of the line: a Go or C++ symbol may carry
        // parentheses in its own name (`internal/abi.(*RegArgs).IntRegArgAddr`).
        let header = lines[signature];
        let open = header.rfind('(').unwrap_or(0);
        let close = header.rfind(')').unwrap_or(header.len());
        let parameters = if close > open {
            &header[open + 1..close]
        } else {
            ""
        };
        let declared = parameters
            .split(',')
            .filter_map(|parameter| {
                let name = parameter.trim().rsplit(' ').next()?;
                name.strip_prefix("arg")?.parse::<usize>().ok()
            })
            .max()
            .unwrap_or(0);
        let used = text
            .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
            .filter_map(|token| {
                token
                    .strip_prefix("arg")
                    .and_then(|digits| digits.parse::<usize>().ok())
            })
            .max()
            .unwrap_or(0);
        if used > declared {
            findings.push((
                LintKind::UndeclaredArgument,
                format!("arg{used} > {declared}"),
            ));
        }
    }
    findings
}

/// Every `case`/`default` label must sit exactly one level inside the `switch` it belongs to.
/// A text pass that moves a ladder or an arm can leave a label at the wrong column, which reads
/// as if the arm were outside the switch (and is how one unbalanced-brace bug started).
fn misplaced_case_labels(lines: &[&str]) -> Vec<(LintKind, String)> {
    let mut findings = Vec::new();
    let mut quoted = false;
    let mut escaped = false;
    let mut depth = 0isize;
    // (brace depth right after the `switch` opened, its indentation)
    let mut switches: Vec<(isize, usize)> = Vec::new();
    for line in lines {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        let label =
            trimmed.starts_with("case ") || trimmed == "default:" || trimmed == "default: {";
        if let Some(&(_, switch_indent)) = switches.last()
            && label
            && indent != switch_indent + 4
        {
            findings.push((
                LintKind::MisplacedCaseLabel,
                format!(
                    "{} at column {indent}, switch at {switch_indent}",
                    trimmed.trim_end_matches(" {")
                ),
            ));
        }
        for ch in line.chars() {
            match ch {
                '"' if !escaped => quoted = !quoted,
                '\\' if quoted => escaped = !escaped,
                '{' if !quoted => depth += 1,
                '}' if !quoted => depth -= 1,
                _ => escaped = false,
            }
        }
        if trimmed.starts_with("switch (") && trimmed.ends_with('{') {
            switches.push((depth, indent));
        }
        while switches
            .last()
            .is_some_and(|&(open_depth, _)| depth < open_depth)
        {
            switches.pop();
        }
    }
    findings
}

/// Every `loc_…` target named by a `goto` on this line.
fn goto_targets(line: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let mut rest = line;
    while let Some(position) = rest.find("goto loc_") {
        let after = &rest[position + "goto loc_".len()..];
        let length = after
            .chars()
            .take_while(|ch| ch.is_ascii_hexdigit())
            .count();
        if length > 0 {
            targets.push(format!("loc_{}", &after[..length]));
        }
        rest = &after[length..];
    }
    targets
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
    write_golden_limited(session, dir, usize::MAX)
}

/// Write snapshots for at most `limit` functions (a sample of a very large project).
pub fn write_golden_limited(session: &mut Session, dir: &Path, limit: usize) -> Result<usize> {
    std::fs::create_dir_all(dir)?;
    let mut functions = sorted_functions(session)?;
    functions.truncate(limit);
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

/// Write one L2 pseudo-C file per function into `dir`. This is the export counterpart of the
/// snapshots: same file names, but the text a reader wants rather than the normalised L1 form.
pub fn export_pseudocode(
    session: &mut Session,
    dir: &Path,
    limit: usize,
    only_unstructured: bool,
) -> Result<usize> {
    std::fs::create_dir_all(dir)?;
    let mut functions = sorted_functions(session)?;
    let limit = if limit == 0 {
        functions.len()
    } else {
        limit.min(functions.len())
    };
    functions.truncate(limit);
    let names = golden_file_names(session, &functions)?;
    // The index makes a directory of a few hundred files navigable: file, address, jump count and
    // size, which is exactly what "which function should I read first" needs.
    let mut index = String::from("file\taddress\tgotos\tlines\n");
    let mut written = 0usize;
    for (position, function) in functions.iter().enumerate() {
        let code = pseudocode_l2(session, function.start)?;
        let gotos = code
            .lines
            .iter()
            .filter(|line| line.text.contains("goto loc_"))
            .count();
        // Reviewing the hard cases only: a function with no fallback jump is already structured, so
        // its file is noise in that workflow.
        if only_unstructured && gotos == 0 {
            continue;
        }
        index.push_str(&format!(
            "{}\t{:#x}\t{}\t{}\n",
            names[position],
            function.start.get(),
            gotos,
            code.lines.len()
        ));
        std::fs::write(dir.join(&names[position]), code.text())?;
        written += 1;
    }
    std::fs::write(dir.join("index.tsv"), index)?;
    Ok(written)
}

/// Compare the project against snapshots in `dir`.
pub fn check_golden(session: &mut Session, dir: &Path) -> Result<GoldenReport> {
    check_golden_limited(session, dir, usize::MAX)
}

/// Check snapshots for at most `limit` functions.
pub fn check_golden_limited(
    session: &mut Session,
    dir: &Path,
    limit: usize,
) -> Result<GoldenReport> {
    let mut functions = sorted_functions(session)?;
    functions.truncate(limit);
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

/// Longest snapshot file stem kept before the dedupe suffix is added.
const MAX_STEM: usize = 80;

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
    // Go (and C++) names can exceed the 255-byte file-name limit after sanitising, so long
    // stems are shortened; the dedupe pass appends the address when two stems collide.
    let stem: String = stem.chars().take(MAX_STEM).collect();
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
    use recraft_analysis::{analyze, extract_strings, persist, persist_strings};
    use std::path::PathBuf;

    fn corpus_for(bits: &str) -> Option<(Session, tempfile::TempDir)> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../corpus/build")
            .join(bits)
            .join("corpus.exe");
        let bytes = std::fs::read(path).ok()?;
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("quality.rcraft");
        let mut session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let result = analyze(&mut session).unwrap();
        persist(&mut session, &result).unwrap();
        Some((session, dir))
    }

    fn corpus() -> Option<(Session, tempfile::TempDir)> {
        corpus_for("x64")
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

    /// Number of snapshots committed for the real-world crackme fixture (`corpus/real/crackme`),
    /// the same `--limit 160` selection the case set uses.
    const REAL_CRACKME_SNAPSHOTS: usize = 160;

    /// The real-world sample, rebuilt from `corpus/real/crackme/src/crackme.c`. Like the corpus,
    /// the binary is a build product: without `corpus/real/crackme/build.ps1` having run, these
    /// tests skip instead of failing a clean checkout.
    fn real_crackme() -> Option<(Session, tempfile::TempDir)> {
        let exe = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../corpus/build/real/crackme/crackme.exe");
        let bytes = std::fs::read(exe).ok()?;
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("real-crackme.rcraft");
        let mut session = Session::create(&project, &bytes, &mut |_, _| {}, &|| false).unwrap();
        let result = analyze(&mut session).unwrap();
        persist(&mut session, &result).unwrap();
        // Mirror `recraft-cli analyze`: the strings feed comments in the emitted text, and the
        // built-in CRT signatures name `main`, `strlen`, ...; both are visible in snapshots.
        let strings = extract_strings(&mut session, 4, 200_000).unwrap();
        persist_strings(&mut session, &strings).unwrap();
        crate::signature::apply_builtin(&mut session, false).unwrap();
        Some((session, dir))
    }

    /// The fixture carries a full static CRT and a messier CFG than any corpus probe; its output
    /// must still satisfy every invariant. Only the snapshot selection is linted, because the
    /// whole 495-function sample costs ~20 s in a debug build.
    #[test]
    fn the_real_crackme_fixture_passes_every_output_invariant() {
        let Some((mut session, _dir)) = real_crackme() else {
            return;
        };
        let report = lint(&mut session, REAL_CRACKME_SNAPSHOTS, LintLevel::Both).unwrap();
        assert!(report.findings.is_empty(), "{:?}", report.findings);
        assert_eq!(report.affected(), 0);
    }

    /// Committed snapshots for the fixture: `tests/golden/crackme-real` once a rebuild has
    /// diverged from the pinned case binary, `tests/golden/crackme` while it is byte-identical
    /// (see `corpus/real/crackme/README.md`).
    #[test]
    fn the_real_crackme_fixture_matches_its_snapshots() {
        let Some((mut session, _dir)) = real_crackme() else {
            return;
        };
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let real = root.join("tests/golden/crackme-real");
        let has_real_snapshots = std::fs::read_dir(&real)
            .map(|entries| {
                entries.flatten().any(|entry| {
                    entry.path().extension().and_then(|value| value.to_str()) == Some("c")
                })
            })
            .unwrap_or(false);
        let dir = if has_real_snapshots {
            real
        } else {
            root.join("tests/golden/crackme")
        };
        let report = check_golden_limited(&mut session, &dir, REAL_CRACKME_SNAPSHOTS).unwrap();
        assert!(
            report.ok(),
            "{} checked, {} missing, {} mismatched in {}",
            report.checked,
            report.missing.len(),
            report.mismatched.len(),
            dir.display()
        );
    }

    #[test]
    fn the_32bit_corpus_passes_every_output_invariant() {
        let Some((mut session, _dir)) = corpus_for("x86") else {
            return;
        };
        assert!(!session.image().is_64);
        let report = lint(&mut session, usize::MAX, LintLevel::Both).unwrap();
        assert!(report.findings.is_empty(), "{:?}", report.findings);
        assert_eq!(report.affected(), 0);
    }
    #[test]
    fn exporting_only_unstructured_skips_a_fully_structured_sample() {
        let Some((mut session, _dir)) = corpus_for("x64") else {
            return;
        };
        let out = std::env::temp_dir().join(format!("recraft-hard-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&out);
        let written = export_pseudocode(&mut session, &out, 0, true).expect("export");
        assert_eq!(written, 0, "the x64 corpus is fully structured");
        let files: Vec<_> = std::fs::read_dir(&out)
            .expect("read dir")
            .map(|entry| entry.expect("entry").file_name().into_string().unwrap())
            .collect();
        assert_eq!(files, vec!["index.tsv".to_owned()]);
        let index = std::fs::read_to_string(out.join("index.tsv")).expect("index");
        assert_eq!(index.lines().count(), 1, "only the header");
        std::fs::remove_dir_all(&out).expect("clean up");
    }

    #[test]
    fn the_histogram_counts_functions_by_fallback_jumps() {
        let Some((mut session, _dir)) = corpus_for("x64") else {
            return;
        };
        let report = measure(&mut session, 0).expect("measure");
        let histogram = report.fallback_histogram();
        let counted: usize = histogram.values().sum();
        assert_eq!(counted, report.total());
        // A structured function contributes to the "0 jumps" bucket.
        assert_eq!(histogram.get(&0).copied().unwrap_or(0), report.structured());
    }

    #[test]
    fn a_zero_limit_measures_every_function() {
        let Some((mut session, _dir)) = corpus_for("x64") else {
            return;
        };
        let all = measure(&mut session, 0).expect("measure all");
        let capped = measure(&mut session, 5).expect("measure five");
        assert!(all.total() > capped.total());
        assert_eq!(capped.total(), 5);
        assert_eq!(capped.skipped, all.total() - 5);
    }

    #[test]
    fn checking_a_directory_without_snapshots_reports_them_all_missing() {
        let Some((mut session, _dir)) = corpus_for("x64") else {
            return;
        };
        let empty = std::env::temp_dir().join(format!("recraft-empty-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&empty);
        std::fs::create_dir_all(&empty).expect("create dir");
        let report = check_golden_limited(&mut session, &empty, usize::MAX).expect("check");
        assert!(report.mismatched.is_empty());
        assert_eq!(report.missing.len(), report.checked);
        std::fs::remove_dir_all(&empty).expect("clean up");
    }

    #[test]
    fn exporting_pseudocode_writes_one_l2_file_per_function() {
        let Some((mut session, _dir)) = corpus_for("x64") else {
            return;
        };
        let out = std::env::temp_dir().join(format!("recraft-export-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&out);
        let written = export_pseudocode(&mut session, &out, 0, false).expect("export");
        assert!(written >= 30, "only {written} functions exported");
        let files: Vec<_> = std::fs::read_dir(&out)
            .expect("read dir")
            .map(|entry| entry.expect("entry").path())
            .collect();
        // One file per function plus the index.
        assert_eq!(files.len(), written + 1);
        let index = std::fs::read_to_string(out.join("index.tsv")).expect("index");
        assert_eq!(index.lines().count(), written + 1, "index rows");
        let l2 = files
            .iter()
            .filter(|path| {
                std::fs::read_to_string(path)
                    .map(|text| text.contains("// L2 structure recovery"))
                    .unwrap_or(false)
            })
            .count();
        assert_eq!(l2, written, "exported text is not the L2 rendering");
        std::fs::remove_dir_all(&out).expect("clean up");
    }

    #[test]
    fn the_corpus_keeps_its_structured_rate_and_coverage() {
        // A structured-rate regression would not show up in the text snapshots (they store the L1
        // shape), so the measurement itself is fenced here.
        for bits in ["x64", "x86"] {
            let Some((mut session, _dir)) = corpus_for(bits) else {
                continue;
            };
            let report = measure(&mut session, usize::MAX).unwrap();
            assert_eq!(
                report.structured(),
                report.total(),
                "{bits}: {} of {} functions structured",
                report.structured(),
                report.total()
            );
            assert!(
                report.coverage() >= 0.97,
                "{bits}: coverage {:.3}",
                report.coverage()
            );
            assert!(
                report.total() >= 30,
                "{bits}: only {} functions",
                report.total()
            );
        }
    }
    #[test]
    fn the_corpus_passes_every_output_invariant() {
        let Some((mut session, _dir)) = corpus() else {
            return;
        };
        let report = lint(&mut session, usize::MAX, LintLevel::Both).unwrap();
        assert!(report.findings.is_empty(), "{:?}", report.findings);
        assert_eq!(report.affected(), 0);
        assert_eq!(report.checked, report.functions * 2);
    }
    #[test]
    fn lint_catches_each_invariant_violation() {
        let kinds = |text: &str| -> Vec<LintKind> {
            lint_text(text).into_iter().map(|(kind, _)| kind).collect()
        };
        // A clean function reports nothing.
        assert!(kinds("int32 sub_1(int64 arg1) {\n    return arg1;\n}\n").is_empty());
        // A jump without a label.
        assert!(
            kinds("int32 sub_1(int64 arg1) {\n    goto loc_10;\n}\n")
                .contains(&LintKind::GotoWithoutLabel)
        );
        // The same label twice.
        assert!(
            kinds("int32 sub_1(int64 arg1) {\n    loc_10:\n    loc_10:\n}\n")
                .contains(&LintKind::DuplicateLabel)
        );
        // Unbalanced braces.
        assert!(
            kinds("int32 sub_1(int64 arg1) {\n    if (arg1) {\n}\n")
                .contains(&LintKind::UnbalancedBraces)
        );
        // A conditionless branch.
        assert!(
            kinds("int32 sub_1(int64 arg1) {\n    if () {\n    }\n}\n")
                .contains(&LintKind::EmptyCondition)
        );
        // An argument the signature does not declare.
        assert!(
            kinds("int32 sub_1(int64 arg1) {\n    return arg2;\n}\n")
                .contains(&LintKind::UndeclaredArgument)
        );
        // A label above the signature.
        assert!(
            kinds("loc_10:\nint32 sub_1(int64 arg1) {\n}\n")
                .contains(&LintKind::LabelAboveSignature)
        );
        // A `case` label that escaped its switch body.
        assert!(
            kinds(
                "int32 sub_1(int32 arg1) {\n    switch (arg1) {\n        case 0x1: {\n        break;\n        }\n    case 0x2: {\n        break;\n    }\n    }\n}\n"
            )
            .contains(&LintKind::MisplacedCaseLabel)
        );
        // The same switch with both labels one level inside is clean.
        assert!(
            kinds(
                "int32 sub_1(int32 arg1) {\n    switch (arg1) {\n        case 0x1: {\n            break;\n        }\n        default: {\n            break;\n        }\n    }\n}\n"
            )
            .is_empty()
        );
        // A nested switch judges its labels against its own column.
        assert!(
            kinds(
                "int32 sub_1(int32 arg1) {\n    switch (arg1) {\n        case 0x1: {\n            switch (arg1) {\n                case 0x2: {\n                    break;\n                }\n            }\n            break;\n        }\n    }\n}\n"
            )
            .is_empty()
        );
    }
    #[test]
    fn long_names_are_shortened_for_snapshots() {
        let stem = sanitise_stem(
            Some(
                "internal/abi.(*RegArgs).SomeVeryLongMethodNameThatKeepsGoingAndGoingAndGoingAndGoingPastTheLimit",
            ),
            Address::new(0x140001000),
        );
        assert!(stem.len() <= MAX_STEM, "{stem}");
        assert!(!stem.contains('/'), "{stem}");
        assert!(!stem.contains('('), "{stem}");
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
