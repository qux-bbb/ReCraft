// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Naming-accuracy benchmark (DESIGN §9.3 "用 PDB 符号做命名准确率回归").
//!
//! The prompt benchmark in [`crate::eval`] only proves that the *input* carries evidence.
//! This module answers the question users actually ask — "are the proposed names any
//! good?" — by scoring the naming pipeline against functions whose real symbol is still
//! in the database (exports, entry points, a benchmark binary built from source).
//!
//! Two modes share one scorer:
//!
//! - `heuristic` (offline, deterministic): what the panel produces without a provider.
//!   It is the floor a model has to beat, and it runs in CI without a key.
//! - `model` (needs a provider): the same candidates, but the prompt is **blinded** —
//!   every candidate's current symbol is replaced with `sub_<addr>`, so the model cannot
//!   read the answer it is being scored on. Recursion is scrubbed the same way.
//!
//! Scoring is deliberately tolerant of word order and of the address suffix the
//! heuristic appends: `exact` compares normalised names, `partial` is token overlap ≥ 0.5,
//! and `overlap` is the Jaccard score of the identifier tokens. Renaming `field_probe` to
//! `probe_field_140001260` is not a failure; calling it `network_140001260` is.

use std::collections::BTreeMap;

use recraft_common::{Address, Result};
use recraft_core::Session;

/// Version of this benchmark (bump when the scorer or the candidate filter changes).
pub const NAME_EVAL_VERSION: &str = "name-eval-2026-11-v1";

/// How the benchmark produced names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameEvalMode {
    Heuristic,
    Model,
}

impl NameEvalMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            NameEvalMode::Heuristic => "heuristic",
            NameEvalMode::Model => "model",
        }
    }
}

/// Score of one proposed name against the real symbol.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NameScore {
    /// Normalised proposal equals the normalised truth.
    pub exact: bool,
    /// `exact`, or at least half of the identifier tokens are shared.
    pub partial: bool,
    /// Jaccard overlap of the identifier tokens (0..=1).
    pub overlap: f32,
}

impl NameScore {
    pub fn json_fields(self) -> (bool, bool, f32) {
        (self.exact, self.partial, self.overlap)
    }
}

/// One scored function.
#[derive(Debug, Clone)]
pub struct NameCase {
    pub addr: Address,
    /// The symbol the binary actually carries.
    pub truth: String,
    /// What the pipeline proposed (empty when the model skipped the function).
    pub proposal: String,
    /// `heuristic` / `model` / `missing`.
    pub source: &'static str,
    pub apis: usize,
    pub strings: usize,
    pub score: NameScore,
}

/// Aggregate result of one benchmark run.
#[derive(Debug, Clone)]
pub struct NameEvalReport {
    pub version: &'static str,
    pub mode: NameEvalMode,
    pub template: &'static str,
    pub cases: Vec<NameCase>,
    pub exact: usize,
    pub partial: usize,
    /// Functions the model did not name at all (empty proposal).
    pub missing: usize,
    /// Mean token overlap over all cases (0..=1).
    pub mean_overlap: f32,
}

impl NameEvalReport {
    pub fn total(&self) -> usize {
        self.cases.len()
    }

    /// Share of exact matches, in percent.
    pub fn exact_pct(&self) -> f32 {
        percent(self.exact, self.total())
    }

    /// Share of exact + partial matches, in percent.
    pub fn partial_pct(&self) -> f32 {
        percent(self.partial, self.total())
    }

    /// One-line summary for the CLI and tests.
    pub fn summary(&self) -> String {
        format!(
            "name eval {} ({}): {}/{} exact ({:.1}%), {}/{} partial ({:.1}%), mean overlap {:.2}, {} missing",
            self.version,
            self.mode.as_str(),
            self.exact,
            self.total(),
            self.exact_pct(),
            self.partial,
            self.total(),
            self.partial_pct(),
            self.mean_overlap,
            self.missing
        )
    }
}

fn percent(count: usize, total: usize) -> f32 {
    if total == 0 {
        0.0
    } else {
        count as f32 * 100.0 / total as f32
    }
}

/// A candidate with the evidence gathered for scoring.
struct Seed {
    addr: Address,
    truth: String,
    apis: usize,
    strings: usize,
}

impl Seed {
    fn into_case(self, proposal: String, source: &'static str) -> NameCase {
        let score = score_name(&self.truth, &proposal);
        NameCase {
            addr: self.addr,
            truth: self.truth,
            proposal,
            source,
            apis: self.apis,
            strings: self.strings,
            score,
        }
    }
}

/// Functions whose real symbol is known: exports, the entry point, and user names.
/// `sub_*`/`loc_*` are auto names, not ground truth, and decorated C++ symbols are not
/// C identifiers so they cannot be compared to a proposal.
fn collect(session: &mut Session, limit: usize) -> Result<Vec<Seed>> {
    let mut seeds = Vec::new();
    for function in session.functions()? {
        if seeds.len() >= limit {
            break;
        }
        let Some(name) = session.name_at(function.start)? else {
            continue;
        };
        if name.starts_with("sub_") || name.starts_with("loc_") || !is_c_identifier(&name) {
            continue;
        }
        let explanation = crate::explain(session, function.start)?;
        seeds.push(Seed {
            addr: function.start,
            truth: name,
            apis: explanation.apis.len(),
            strings: explanation.strings.len(),
        });
    }
    Ok(seeds)
}

fn is_c_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(first) if first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

/// Score one proposal against the real symbol (`proposal` may be empty).
pub fn score_name(truth: &str, proposal: &str) -> NameScore {
    let truth = normalize_symbol(truth);
    let proposal = normalize_symbol(proposal);
    if proposal.is_empty() || truth.is_empty() {
        return NameScore {
            exact: false,
            partial: false,
            overlap: 0.0,
        };
    }
    let truth_tokens = tokens(&truth);
    let proposal_tokens = tokens(&proposal);
    // Word order carries no meaning in a symbol (`probe_field` == `field_probe`).
    let exact = if truth_tokens.is_empty() || proposal_tokens.is_empty() {
        truth == proposal
    } else {
        truth_tokens == proposal_tokens
    };
    let overlap = if truth_tokens.is_empty() || proposal_tokens.is_empty() {
        0.0
    } else {
        let shared = truth_tokens.intersection(&proposal_tokens).count();
        let union = truth_tokens.union(&proposal_tokens).count();
        shared as f32 / union.max(1) as f32
    };
    NameScore {
        exact,
        partial: exact || overlap >= 0.5,
        overlap,
    }
}

/// Lowercase snake_case, with a trailing `_<address>` removed. The heuristic appends the
/// function address to keep names unique; that tail says nothing about the semantics.
pub fn normalize_symbol(name: &str) -> String {
    let mut text = String::with_capacity(name.len() + 8);
    let mut previous: Option<char> = None;
    for ch in name.trim().chars() {
        if ch == '_' || ch == '-' || ch == ' ' || ch == '.' || ch == ':' {
            text.push('_');
            previous = Some(ch);
            continue;
        }
        if ch.is_ascii_uppercase() {
            let split =
                previous.is_some_and(|prev| prev.is_ascii_lowercase() || prev.is_ascii_digit());
            if split && !text.ends_with('_') {
                text.push('_');
            }
            text.push(ch.to_ascii_lowercase());
        } else if ch.is_ascii_alphanumeric() {
            text.push(ch);
        } else {
            text.push('_');
        }
        previous = Some(ch);
    }
    let mut normalized: String = text
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_");
    normalized = strip_address_suffix(&normalized);
    normalized
}

fn strip_address_suffix(name: &str) -> String {
    if let Some((head, tail)) = name.rsplit_once('_')
        && tail.len() >= 5
        && tail.len() <= 16
        && tail.chars().all(|ch| ch.is_ascii_hexdigit())
        && head.chars().any(|ch| ch.is_ascii_alphabetic())
    {
        return head.to_owned();
    }
    name.to_owned()
}

fn tokens(symbol: &str) -> std::collections::BTreeSet<String> {
    symbol
        .split('_')
        .filter(|token| token.len() > 1)
        .map(str::to_owned)
        .collect()
}

/// Score the offline heuristic naming for every function that still has a real symbol.
pub fn evaluate_names_heuristic(session: &mut Session, limit: usize) -> Result<NameEvalReport> {
    let seeds = collect(session, limit)?;
    let mut cases = Vec::with_capacity(seeds.len());
    for seed in seeds {
        let explanation = crate::explain(session, seed.addr)?;
        let proposal =
            crate::heuristic_proposal(seed.addr, &explanation.apis, &explanation.strings);
        // The API/string counts come from the same explanation the heuristic used.
        let mut seed = seed;
        seed.apis = explanation.apis.len();
        seed.strings = explanation.strings.len();
        cases.push(seed.into_case(proposal.value, "heuristic"));
    }
    Ok(report(NameEvalMode::Heuristic, cases))
}

/// Score the model naming for every function that still has a real symbol.
///
/// Candidates are sent in blinded batches of [`MAX_BATCH`]; the reply is parsed with the
/// production parser, so a contract regression shows up as `missing`, not as a crash.
pub fn evaluate_names_with_provider(
    session: &mut Session,
    settings: &crate::ProviderSettings,
    limit: usize,
) -> Result<NameEvalReport> {
    const MAX_BATCH: usize = 16;
    let seeds = collect(session, limit)?;
    if seeds.is_empty() {
        return Ok(report(NameEvalMode::Model, Vec::new()));
    }
    let mut provider = crate::provider_from_settings(settings);
    let mut named: BTreeMap<u64, String> = BTreeMap::new();
    for batch in seeds.chunks(MAX_BATCH) {
        let candidates: Vec<Address> = batch.iter().map(|seed| seed.addr).collect();
        let prompt = crate::build_rename_prompt_blinded(session, &candidates, settings)?;
        let reply = provider.complete(&prompt)?;
        for suggestion in crate::parse_rename_reply(&reply) {
            named.insert(suggestion.target.get(), suggestion.name);
        }
    }
    let cases = seeds
        .into_iter()
        .map(|seed| {
            let proposal = named.remove(&seed.addr.get()).unwrap_or_default();
            let source = if proposal.is_empty() {
                "missing"
            } else {
                "model"
            };
            seed.into_case(proposal, source)
        })
        .collect();
    Ok(report(NameEvalMode::Model, cases))
}

fn report(mode: NameEvalMode, cases: Vec<NameCase>) -> NameEvalReport {
    let exact = cases.iter().filter(|case| case.score.exact).count();
    let partial = cases.iter().filter(|case| case.score.partial).count();
    let missing = cases
        .iter()
        .filter(|case| case.proposal.trim().is_empty())
        .count();
    let mean_overlap = if cases.is_empty() {
        0.0
    } else {
        cases.iter().map(|case| case.score.overlap).sum::<f32>() / cases.len() as f32
    };
    NameEvalReport {
        version: NAME_EVAL_VERSION,
        mode,
        template: crate::PROMPT_TEMPLATE_VERSION,
        cases,
        exact,
        partial,
        missing,
        mean_overlap,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_ignores_case_order_and_address_suffix() {
        assert!(score_name("field_probe", "field_probe").exact);
        assert!(score_name("field_probe", "probe_field").exact);
        assert!(score_name("field_probe", "field_probe_140001260").exact);
        assert!(score_name("RegOpenKeyExW", "reg_open_key_ex_w").exact);
        assert!(!score_name("field_probe", "array_probe").exact);
    }

    #[test]
    fn partial_counts_shared_tokens_and_vague_names_do_not_pass() {
        let score = score_name("open_config_file", "open_file");
        assert!(!score.exact);
        assert!(score.partial, "{score:?}");
        let vague = score_name("write_registry_value", "interesting_1400012a8");
        assert!(!vague.exact && !vague.partial, "{vague:?}");
        let empty = score_name("field_probe", "");
        assert!(!empty.exact && !empty.partial && empty.overlap == 0.0);
    }

    #[test]
    fn address_suffix_is_only_stripped_when_it_is_hex() {
        assert_eq!(normalize_symbol("sub_140001000"), "sub");
        assert_eq!(normalize_symbol("crc32"), "crc32");
        assert_eq!(normalize_symbol("base64_encode"), "base64_encode");
        assert_eq!(normalize_symbol("dispatch_401050"), "dispatch");
        // A short hex tail is part of the word, not an address.
        assert_eq!(normalize_symbol("sha256"), "sha256");
    }
}
