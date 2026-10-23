// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! Prompt-quality regression baseline (offline; no model or network needed).

use recraft_ai::{
    CORPUS_CASES, EVAL_VERSION, PROMPT_TEMPLATE_VERSION, ensure_eval, evaluate_explain,
};
use recraft_analysis::{analyze, persist};
use recraft_core::Session;

#[test]
fn corpus_prompt_eval_passes() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../corpus/build/x64/corpus.exe");
    let Ok(bytes) = std::fs::read(path) else {
        eprintln!("corpus fixture missing; prompt eval skipped");
        return;
    };
    let dir = tempfile::tempdir().expect("tempdir");
    let project = dir.path().join("eval.rcraft");
    let mut session =
        Session::create(&project, &bytes, &mut |_, _| {}, &|| false).expect("session");
    let result = analyze(&mut session).expect("analyze");
    persist(&mut session, &result).expect("persist");

    let report = evaluate_explain(&mut session, CORPUS_CASES).expect("eval run");
    assert!(
        report.is_ok(),
        "{EVAL_VERSION}: {}/{} passed: {}",
        report.passed,
        report.cases,
        report.failures.join(" | ")
    );
    assert_eq!(report.passed, CORPUS_CASES.len());
    ensure_eval(&report).expect("no failures");
    assert!(!PROMPT_TEMPLATE_VERSION.is_empty());
}
