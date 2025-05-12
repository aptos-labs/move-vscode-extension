use crate::init_tracing_for_test;
use expect_test::Expect;
use ide::Analysis;
use ide_db::Severity;
use ide_db::assists::{Assist, AssistResolveStrategy};
use ide_diagnostics::config::DiagnosticsConfig;
use ide_diagnostics::diagnostic::Diagnostic;
use syntax::TextRange;
use test_utils::{
    Marking, apply_markings, get_all_marked_positions, get_first_marked_position, remove_markings,
};
use vfs::FileId;

pub fn check_diagnostics(expect: Expect) {
    init_tracing_for_test();

    let source = stdx::trim_indent(expect.data());
    let trimmed_source = remove_markings(&source);

    let (_, _, diagnostics) = get_diagnostics(trimmed_source.as_str());

    let mut actual = apply_diagnostics_to_file(&trimmed_source, &diagnostics);
    actual.push_str("\n");

    expect.assert_eq(stdx::trim_indent(&actual).as_str());
}
//
// pub fn check_diagnostic_and_fix(before: &str, after: &str) {
//     let (_, file_id, mut diagnostic) = get_diagnostics(before);
//     let diag = diagnostic.pop().expect("diagnostics expected, but none returned");
//
//     let (exp_range, exp_severity, exp_message) = get_expected_diagnostics(before, file_id)
//         .pop()
//         .expect("missing diagnostic mark");
//     assert_eq!(diag.range.range, exp_range);
//     assert_eq!(diag.severity, exp_severity);
//     assert_eq!(diag.message, exp_message);
//
//     let fix = &diag
//         .fixes
//         .unwrap_or_else(|| panic!("{:?} diagnostic misses fixes", diag.code))[0];
//
//     let line_idx = get_first_marked_position(before, "//^").mark_line_col.line;
//     let mut lines = before.lines().collect::<Vec<_>>();
//     lines.remove(line_idx as usize);
//     let before_no_error_line = lines.join("\n");
//
//     let actual_after = apply_fix(fix, &before_no_error_line);
//     assert_eq_text!(&actual_after, after);
// }

pub fn check_diagnostics_and_fix(before: Expect, after: Expect) {
    init_tracing_for_test();

    let before_source = stdx::trim_indent(before.data());
    let trimmed_before_source = remove_markings(&before_source);

    let (_, _, mut diagnostics) = get_diagnostics(trimmed_before_source.as_str());

    let diagnostic = diagnostics.pop().expect("no diagnostics found");
    assert_no_extra_diagnostics(&trimmed_before_source, diagnostics);

    let mut actual = apply_diagnostics_to_file(&trimmed_before_source, &vec![diagnostic.clone()]);
    actual.push_str("\n");

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
    let (analysis, file_id) = Analysis::from_single_file(source.to_string());

    let config = DiagnosticsConfig::test_sample();
    let diagnostics = analysis
        .semantic_diagnostics(&config, AssistResolveStrategy::None, file_id)
        .unwrap();

    (analysis, file_id, diagnostics)
}

fn get_expected_diagnostics(source: &str) -> Vec<(TextRange, Severity, String)> {
    let marked_positions = get_all_marked_positions(source, "//^");

    let mut exps = vec![];
    for marked in marked_positions {
        let mut parts = marked.line.splitn(3, " ");
        let prefix = parts.next().unwrap();
        let severity = parts.next().unwrap();
        let len = prefix.trim_start_matches("//").len();
        let exp_range = TextRange::at(marked.item_offset, (len as u32).into());
        let expected_severity = Severity::from_test_ident(severity);
        let expected_message = parts.next().unwrap();
        exps.push((exp_range, expected_severity, expected_message.to_string()));
    }
    exps
}

fn apply_fix(fix: &Assist, before: &str) -> String {
    let source_change = fix.source_change.as_ref().unwrap();
    let mut after = before.to_string();

    for (edit, snippet_edit) in source_change.source_file_edits.values() {
        edit.apply(&mut after);
        if let Some(snippet_edit) = snippet_edit {
            snippet_edit.apply(&mut after);
        }
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
            Marking { text_range, message }
        })
        .collect();
    apply_markings(source, markings)
}
