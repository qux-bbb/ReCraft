// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Offline prompt-quality regression baseline (DESIGN §9.3).
//!
//! LLM replies cannot be scored deterministically without a model, but the **prompt**
//! fed to a provider can: it must contain the function's real evidence (API tags,
//! referenced strings, pseudo-code) within the token budget, and must stay redacted.
//! [`evaluate_explain`] checks that contract against fixed cases, so prompt/template
//! regressions fail CI without any network access.

use recraft_common::{Error, Result};
use recraft_core::Session;

/// Version of the evaluation baseline (bump when cases or expectations change).
pub const EVAL_VERSION: &str = "prompt-eval-2026-10-v1";

/// One expected prompt for a function: the prompt must contain every `expected` needle.
#[derive(Debug, Clone, Copy)]
pub struct EvalCase {
    pub name: &'static str,
    /// Substrings that must appear in the assembled prompt (evidence/derived text).
    pub expected: &'static [&'static str],
}

/// Built-in cases for the x64 corpus fixture (`corpus/build/x64/corpus.exe`).
pub const CORPUS_CASES: &[EvalCase] = &[
    EvalCase {
        name: "field_probe",
        expected: &["field_probe", "arg1->field_10", "pseudo-code:"],
    },
    EvalCase {
        name: "array_probe",
        expected: &["array_probe", "arg1[", "pseudo-code:"],
    },
    EvalCase {
        name: "str_probe",
        expected: &["str_probe", "char* arg1", "arg1"],
    },
    EvalCase {
        name: "float_math",
        expected: &["float_math", "f32"],
    },
    EvalCase {
        name: "branchy",
        expected: &["branchy", "if ("],
    },
];

/// Result of one evaluation run.
#[derive(Debug, Clone, Default)]
pub struct EvalReport {
    pub cases: usize,
    pub passed: usize,
    pub failures: Vec<String>,
}

impl EvalReport {
    pub fn is_ok(&self) -> bool {
        self.failures.is_empty()
    }
}

/// Assemble the explanation prompt for each case and check its evidence + budget.
pub fn evaluate_explain(session: &mut Session, cases: &[EvalCase]) -> Result<EvalReport> {
    let settings = crate::ProviderSettings {
        max_tokens: 2048,
        ..crate::ProviderSettings::default()
    };
    let budget = crate::TokenBudget::new(settings.max_tokens);
    let mut report = EvalReport {
        cases: cases.len(),
        ..EvalReport::default()
    };
    for case in cases {
        let Some(addr) = session.addr_of_name(case.name)? else {
            report
                .failures
                .push(format!("{}: symbol not found", case.name));
            continue;
        };
        let prompt = crate::build_explain_prompt(session, addr, &settings)?;
        let combined = format!("{}\n{}", prompt.system, prompt.user);
        let mut case_failures = Vec::new();
        for needle in case.expected {
            if !combined.contains(needle) {
                case_failures.push(format!("missing {needle:?}"));
            }
        }
        if prompt.estimated_tokens > budget.max_tokens + 8 {
            case_failures.push(format!(
                "prompt over budget: {} > {}",
                prompt.estimated_tokens, budget.max_tokens
            ));
        }
        if combined.contains("http://") || combined.contains("https://") {
            case_failures.push("prompt still contains a raw URL".to_owned());
        }
        if case_failures.is_empty() {
            report.passed += 1;
        } else {
            report
                .failures
                .push(format!("{}: {}", case.name, case_failures.join(", ")));
        }
    }
    Ok(report)
}

/// Fail with the first failures, for CLI/test use.
pub fn ensure_eval(report: &EvalReport) -> Result<()> {
    if report.is_ok() {
        return Ok(());
    }
    Err(Error::internal(format!(
        "prompt eval {EVAL_VERSION}: {}/{} passed; failures: {}",
        report.passed,
        report.cases,
        report.failures.join(" | ")
    )))
}
