// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::init_tracing_for_test;
use expect_test::{Expect, ExpectFile};
use ide::Analysis;
use ide_db::assists::{Assist, AssistResolveStrategy};
use ide_diagnostics::config::DiagnosticsConfig;
use ide_diagnostics::diagnostic::Diagnostic;
use std::collections::HashSet;
use test_utils::fixtures::TestState;
use test_utils::{SourceMark, apply_source_marks, fixtures, get_first_marked_position, remove_marks};
use vfs::FileId;

pub fn check_diagnostics(expect: Expect) {
    init_tracing_for_test();

    let source = stdx::trim_indent(expect.data());
    let trimmed_source = remove_marks(&source, "//^");

    let (_, _, diagnostics) = get_diagnostics(trimmed_source.as_str());

    let actual = apply_diagnostics_to_file(&trimmed_source, &diagnostics);
    expect.assert_eq(stdx::trim_indent(&actual).as_str());
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

pub fn check_diagnostics_in_file(expect: ExpectFile, disabled_codes: HashSet<String>) {
    init_tracing_for_test();

    let source = stdx::trim_indent(&expect.data());
    let trimmed_source = remove_marks(&source, "//^");

    let (_, _, diagnostics) = get_diagnostics(trimmed_source.as_str());
    let diagnostics = diagnostics
        .into_iter()
        .filter(|diag| !disabled_codes.contains(&diag.code.as_str().to_string()))
        .collect::<Vec<_>>();

    let actual = apply_diagnostics_to_file(&trimmed_source, &diagnostics);
    expect.assert_eq(stdx::trim_indent(&actual).as_str());
}

pub fn check_diagnostics_and_fix(before: Expect, after: Expect) {
    init_tracing_for_test();

    let before_source = stdx::trim_indent(before.data());
    let trimmed_before_source = remove_marks(&before_source, "//^");

    let (_, _, mut diagnostics) = get_diagnostics(trimmed_before_source.as_str());

    let diagnostic = diagnostics.pop().expect("no diagnostics found");
    assert_no_extra_diagnostics(&trimmed_before_source, diagnostics);

    let actual = apply_diagnostics_to_file(&trimmed_before_source, &vec![diagnostic.clone()]);
    before.assert_eq(stdx::trim_indent(&actual).as_str());

    let fix = &diagnostic
        .fixes
        .unwrap_or_else(|| panic!("{:?} diagnostic misses fixes", diagnostic.code))[0];

    let line_idx = get_first_marked_position(&before_source, "//^")
        .mark_line_col
        .line;
    let mut lines = before_source.lines().collect::<Vec<_>>();
    lines.remove(line_idx as usize);
    let before_no_error_line = lines.join("\n");

    let mut actual_after = apply_fix(fix, &before_no_error_line);
    actual_after.push_str("\n");
    after.assert_eq(&stdx::trim_indent(&actual_after).as_str());
}

fn get_diagnostics(source: &str) -> (Analysis, FileId, Vec<Diagnostic>) {
    let (analysis, file_id) = fixtures::from_single_file(source.to_string());

    let config = DiagnosticsConfig::test_sample();
    let frange = analysis.full_file_range(file_id).unwrap();
    let diagnostics = analysis
        .semantic_diagnostics(&config, AssistResolveStrategy::All, frange)
        .unwrap();

    (analysis, file_id, diagnostics)
}

fn apply_fix(fix: &Assist, before: &str) -> String {
    let source_change = fix.source_change.as_ref().unwrap();
    let mut after = before.to_string();

    for edit in source_change.source_file_edits.values() {
        edit.apply(&mut after);
    }

    after
}

fn assert_no_extra_diagnostics(source: &str, diags: Vec<Diagnostic>) {
    if diags.is_empty() {
        return;
    }

    println!("Extra diagnostics:");
    for d in diags {
        let s = apply_diagnostics_to_file(source, &vec![d]);
        println!("{}", s);
    }
    println!("======================================");

    panic!("Extra diagnostics available");
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
