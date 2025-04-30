use crate::{assert_eq_text, init_tracing_for_test};
use expect_test::Expect;
use ide::Analysis;
use ide::test_utils::{get_all_marked_positions, get_first_marked_position};
use ide_db::Severity;
use ide_db::assists::{Assist, AssistResolveStrategy};
use ide_diagnostics::config::DiagnosticsConfig;
use ide_diagnostics::diagnostic::Diagnostic;
use lang::nameres::scope::VecExt;
use line_index::{LineCol, LineIndex};
use std::iter;
use syntax::TextRange;
use vfs::FileId;

pub fn check_diagnostics(source: &str) {
    let (_, file_id, mut diagnostics) = get_diagnostics(source);
    let mut exps = get_expected_diagnostics(source, file_id);
    let mut missing_exps = vec![];
    loop {
        if let Some((range, severity, message)) = exps.last().cloned() {
            if let Some(indx) = diagnostics.iter().position(|x| x.range.range == range) {
                let diag = diagnostics.remove(indx);
                assert_eq!(diag.severity, severity);
                assert_eq!(diag.message, message);
            } else {
                missing_exps.push((range, severity, message));
            }
            exps.pop();
        } else {
            break;
        }
    }
    assert!(
        missing_exps.is_empty(),
        "Missing diagnostics {:#?}, \n actual {:#?}",
        missing_exps,
        diagnostics
    );
    assert_no_extra_diagnostics(source, diagnostics);
}

pub fn check_diagnostic_expect(expect: Expect) {
    init_tracing_for_test();

    let source = stdx::trim_indent(expect.data());
    let trimmed_source = remove_expected_diagnostics(&source);

    let (_, _, diagnostics) = get_diagnostics(trimmed_source.as_str());

    let mut actual = apply_diagnostics_to_file(&trimmed_source, &diagnostics);
    actual.push_str("\n");

    expect.assert_eq(stdx::trim_indent(&actual).as_str());
}

pub fn check_diagnostic_and_fix(before: &str, after: &str) {
    let (_, file_id, mut diagnostic) = get_diagnostics(before);
    let diag = diagnostic.pop().expect("diagnostics expected, but none returned");

    let (exp_range, exp_severity, exp_message) = get_expected_diagnostics(before, file_id)
        .pop()
        .expect("missing diagnostic mark");
    assert_eq!(diag.range.range, exp_range);
    assert_eq!(diag.severity, exp_severity);
    assert_eq!(diag.message, exp_message);

    let fix = &diag
        .fixes
        .unwrap_or_else(|| panic!("{:?} diagnostic misses fixes", diag.code))[0];

    let line_idx = get_first_marked_position(before, "//^").mark_line_col.line;
    let mut lines = before.lines().collect::<Vec<_>>();
    lines.remove(line_idx as usize);
    let before_no_error_line = lines.join("\n");

    let actual_after = apply_fix(fix, &before_no_error_line);
    assert_eq_text!(&actual_after, after);
}

pub fn check_diagnostic_and_fix_expect(before: Expect, after: Expect) {
    init_tracing_for_test();

    let before_source = stdx::trim_indent(before.data());
    let trimmed_before_source = remove_expected_diagnostics(&before_source);

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

pub fn check_fix(before: &str, after: &str) {
    let (_, _, diag) = get_diagnostics(before);

    let diag = diag.single_or_none().expect("no diagnostics");
    let fix = &diag
        .fixes
        .unwrap_or_else(|| panic!("{:?} diagnostic misses fixes", diag.code))[0];

    let actual_after = apply_fix(fix, &before);
    assert_eq_text!(&actual_after, after);
}

fn get_diagnostics(source: &str) -> (Analysis, FileId, Vec<Diagnostic>) {
    let (analysis, file_id) = Analysis::from_single_file(source.to_string());

    let config = DiagnosticsConfig::test_sample();
    let diagnostics = analysis
        .semantic_diagnostics(&config, AssistResolveStrategy::None, file_id)
        .unwrap();

    (analysis, file_id, diagnostics)
}

fn get_expected_diagnostics(source: &str, file_id: FileId) -> Vec<(TextRange, Severity, String)> {
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

fn remove_expected_diagnostics(source: &str) -> String {
    let marked_positions = get_all_marked_positions(source, "//^");

    let mut lines_to_remove = vec![];
    for marked in marked_positions {
        lines_to_remove.push(marked.mark_line_col.line as usize);
    }

    let trimmed_source = source
        .lines()
        .enumerate()
        .filter(|(i, line)| !lines_to_remove.contains(i))
        .map(|it| it.1)
        .collect::<Vec<_>>()
        .join("\n");

    trimmed_source
}

fn apply_fix(fix: &Assist, before: &str) -> String {
    let source_change = fix.source_change.as_ref().unwrap();
    // let file_id = *source_change.source_file_edits.keys().next().unwrap();
    // let db = analysis.db();
    // let mut actual = db.file_text(file_id).to_string();
    let mut after = before.to_string();

    for (edit, snippet_edit) in source_change.source_file_edits.values() {
        edit.apply(&mut after);
        if let Some(snippet_edit) = snippet_edit {
            snippet_edit.apply(&mut after);
        }
    }

    after
}

pub(crate) fn assert_no_extra_diagnostics(source: &str, diags: Vec<Diagnostic>) {
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
    let line_index = LineIndex::new(source);

    let mut lines = vec![];
    for diagnostic in diagnostics {
        let text_range = diagnostic.range.range;
        let lc_start = line_index.line_col(text_range.start());
        let lc_end = line_index.line_col(text_range.end());
        let line = diagnostic_line(diagnostic, lc_start, lc_end);
        lines.push((lc_start.line, line));
    }

    let mut source_lines = source.lines().map(|it| it.to_string()).collect::<Vec<_>>();
    let mut added = 0;
    for (line, line_text) in lines {
        let line = line + 1 + added;
        source_lines.insert(line as usize, line_text.clone());
        added += 1;
    }
    source_lines.join("\n")
}

fn diagnostic_line(diagnostic: &Diagnostic, start: LineCol, end: LineCol) -> String {
    let prefix = iter::repeat_n(" ", (start.col - 2) as usize)
        .collect::<Vec<_>>()
        .join("");
    let range = iter::repeat_n("^", (end.col - start.col) as usize)
        .collect::<Vec<_>>()
        .join("");
    let message = diagnostic.message.clone();
    let severity = diagnostic.severity.to_test_ident();
    let line = format!("{prefix}//{range} {severity} {message}");
    line
}
