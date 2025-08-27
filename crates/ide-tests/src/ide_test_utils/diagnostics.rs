// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::init_tracing_for_test;
use expect_test::Expect;
use ide::Analysis;
use ide_db::assists::{Assist, AssistId, AssistResolveStrategy};
use ide_diagnostics::config::DiagnosticsConfig;
use ide_diagnostics::diagnostic::Diagnostic;
use stdx::itertools::Itertools;
use test_utils::fixtures::TestState;
use test_utils::{SourceMark, apply_source_marks, fixtures, remove_marks};
use vfs::FileId;

pub fn check_diagnostics(expect: Expect) {
    init_tracing_for_test();
    check_diagnostics_inner(expect, DiagnosticsConfig::test_sample());
}

pub fn check_diagnostics_with_config(config: DiagnosticsConfig, expect: Expect) {
    init_tracing_for_test();
    check_diagnostics_inner(expect, config);
}

pub fn check_diagnostics_on_tmpfs(test_state: TestState, expect: Expect) {
    // init_tracing_for_test();

    let (file_id, file_source) = test_state.file_with_caret("/*caret*/");

    let config = DiagnosticsConfig::test_sample();
    let frange = test_state.analysis().full_file_range(file_id).unwrap();
    let diagnostics = test_state
        .analysis()
        .semantic_diagnostics(&config, AssistResolveStrategy::All, frange)
        .unwrap();

    let actual = apply_diagnostics_to_file(&file_source, &diagnostics);
    expect.assert_eq(stdx::trim_indent(&actual).as_str());
}

pub fn check_diagnostics_and_fix(before: Expect, after_fix: Expect) {
    init_tracing_for_test();

    let source = clean_source(&before);
    let diagnostics = check_diagnostics_inner(before, DiagnosticsConfig::test_sample());

    let mut fixes = get_fixes_with_id(diagnostics, None);
    let fix = match fixes.len() {
        1 => fixes.pop().unwrap(),
        0 => panic!("No fixes available"),
        _ => panic!("Multiple fixes available"),
    };

    assert_apply_fix(fix, source, after_fix);
}

pub fn check_diagnostics_on_tmpfs_and_fix(test_state: TestState, before_fix: Expect, after_fix: Expect) {
    init_tracing_for_test();

    let (file_id, file_source) = test_state.file_with_caret("/*caret*/");
    let trimmed_before_source = stdx::trim_indent(&file_source);

    let config = DiagnosticsConfig::test_sample();
    let frange = test_state.analysis().full_file_range(file_id).unwrap();
    let diagnostics = test_state
        .analysis()
        .semantic_diagnostics(&config, AssistResolveStrategy::All, frange)
        .unwrap();
    let actual = apply_diagnostics_to_file(&trimmed_before_source, &diagnostics);
    before_fix.assert_eq(stdx::trim_indent(&actual).as_str());

    let fix = get_fixes_with_id(diagnostics, None)
        .into_iter()
        .exactly_one()
        .ok()
        .unwrap_or_else(|| panic!("no fixes found"));

    assert_apply_fix(fix, clean_source(&before_fix), after_fix);
}

pub fn check_diagnostics_and_fix_with_id(fix_id: AssistId, before: Expect, after_fix: Expect) {
    init_tracing_for_test();

    let source = clean_source(&before);
    let diagnostics = check_diagnostics_inner(before, DiagnosticsConfig::test_sample());

    let mut fixes = get_fixes_with_id(diagnostics, Some(fix_id));
    let fix = match fixes.len() {
        1 => fixes.pop().unwrap(),
        0 => panic!("No fixes with id `{}` available", fix_id.0),
        _ => panic!("Multiple fixes with id `{}` available", fix_id.0),
    };

    assert_apply_fix(fix, source, after_fix);
}

pub fn check_diagnostics_no_fix(fix_id: AssistId, before: Expect) {
    init_tracing_for_test();

    let diagnostics = check_diagnostics_inner(before, DiagnosticsConfig::test_sample());

    let fixes = get_fixes_with_id(diagnostics, Some(fix_id));
    assert!(fixes.is_empty(), "extra fixes found");
}

fn check_diagnostics_inner(before: Expect, config: DiagnosticsConfig) -> Vec<Diagnostic> {
    let source = clean_source(&before);

    let (_, _, diagnostics) = get_diagnostics(source.as_str(), config);

    let actual = apply_diagnostics_to_file(&source, &diagnostics);
    before.assert_eq(stdx::trim_indent(&actual).as_str());

    diagnostics
}

fn get_fixes_with_id(diagnostics: Vec<Diagnostic>, fix_id: Option<AssistId>) -> Vec<Assist> {
    diagnostics
        .into_iter()
        .filter_map(|it| it.fixes)
        .flatten()
        .filter(|it| fix_id.is_none_or(|fix_id| it.id == fix_id))
        .collect()
}

fn clean_source(before: &Expect) -> String {
    let source = before.data().to_string();
    remove_marks(&stdx::trim_indent(&source), "//^")
}

fn get_diagnostics(source: &str, config: DiagnosticsConfig) -> (Analysis, FileId, Vec<Diagnostic>) {
    let (analysis, file_id) = fixtures::from_single_file(source.to_string());

    let frange = analysis.full_file_range(file_id).unwrap();
    let diagnostics = analysis
        .semantic_diagnostics(&config, AssistResolveStrategy::All, frange)
        .unwrap();

    (analysis, file_id, diagnostics)
}

fn assert_apply_fix(fix: Assist, source: impl Into<String>, after_fix: Expect) {
    let mut actual_after = apply_fix(&fix, &source.into());
    actual_after.push_str("\n");
    after_fix.assert_eq(&actual_after);
}

pub fn apply_fix(fix: &Assist, before: &str) -> String {
    let source_change = fix.source_change.as_ref().unwrap();
    let mut after = before.to_string();

    for edit in source_change.source_file_edits.values() {
        edit.apply(&mut after);
    }

    after
}

fn apply_diagnostics_to_file(source: &str, diagnostics: &Vec<Diagnostic>) -> String {
    let markings = diagnostics
        .into_iter()
        .map(|it| {
            let text_range = it.range.range;
            let message = format!("{} {}", it.severity.to_test_ident(), it.message.clone());
            SourceMark {
                text_range,
                message,
                custom_symbol: None,
            }
        })
        .collect();
    apply_source_marks(source, markings)
}
