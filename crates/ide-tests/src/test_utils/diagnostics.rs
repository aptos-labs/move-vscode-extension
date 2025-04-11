use crate::assert_eq_text;
use base_db::SourceDatabase;
use ide::Analysis;
use ide::test_utils::{get_marked_position_line_index, get_marked_position_offset_with_line};
use ide_db::Severity;
use ide_db::assists::{Assist, AssistResolveStrategy};
use ide_diagnostics::config::DiagnosticsConfig;
use ide_diagnostics::diagnostic::Diagnostic;
use syntax::TextRange;
use syntax::files::FileRange;
use vfs::FileId;

pub fn check_diagnostic(source: &str) {
    let (_, file_id, diagnostic) = get_diagnostic_at_mark(source);
    let diag = diagnostic.expect("no diagnostics");

    let (expected_range, expected_severity, expected_message) =
        get_expected_diagnostic_at_mark(source, file_id);
    assert_eq!(diag.range, expected_range);
    assert_eq!(diag.severity, expected_severity);
    assert_eq!(diag.message, expected_message);
}

pub fn check_no_diagnostics(source: &str) {
    let (_, _, diagnostic) = get_diagnostic_at_mark(source);
    assert!(
        diagnostic.is_none(),
        "No diagnostics expected, actually {:?}",
        diagnostic
    );
}

pub fn check_diagnostic_and_fix(before: &str, after: &str) {
    let (_, file_id, diagnostic) = get_diagnostic_at_mark(before);
    let diag = diagnostic.expect("no diagnostics");

    let (expected_range, expected_severity, expected_message) =
        get_expected_diagnostic_at_mark(before, file_id);
    assert_eq!(diag.range, expected_range);
    assert_eq!(diag.severity, expected_severity);
    assert_eq!(diag.message, expected_message);

    let fix = &diag
        .fixes
        .unwrap_or_else(|| panic!("{:?} diagnostic misses fixes", diag.code))[0];

    let line_idx = get_marked_position_line_index(before, "//^");
    let mut lines = before.lines().collect::<Vec<_>>();
    lines.remove(line_idx);
    let before_no_error_line = lines.join("\n");

    let actual_after = apply_fix(fix, &before_no_error_line);
    assert_eq_text!(&actual_after, after);
}

pub fn check_fix(before: &str, after: &str) {
    let (_, _, diag) = get_diagnostic_at_mark(before);

    let diag = diag.expect("no diagnostics");
    let fix = &diag
        .fixes
        .unwrap_or_else(|| panic!("{:?} diagnostic misses fixes", diag.code))[0];

    let actual_after = apply_fix(fix, &before);
    assert_eq_text!(&actual_after, after);
}

fn get_diagnostic_at_mark(source: &str) -> (Analysis, FileId, Option<Diagnostic>) {
    let (analysis, file_id) = Analysis::from_single_file(source.to_string());

    let config = DiagnosticsConfig::test_sample();
    let mut diagnostics = analysis
        .semantic_diagnostics(&config, AssistResolveStrategy::None, file_id)
        .unwrap();

    let diagnostic = diagnostics.pop();

    (analysis, file_id, diagnostic)
}

fn get_expected_diagnostic_at_mark(source: &str, file_id: FileId) -> (FileRange, Severity, String) {
    let (offset, line) = get_marked_position_offset_with_line(source, "//^");
    let mut parts = line.splitn(3, " ");

    let prefix = parts.next().unwrap();
    let severity = parts.next().unwrap();

    let len = prefix.trim_start_matches("//").len();
    let expected_range = FileRange {
        file_id,
        range: TextRange::at(offset, (len as u32).into()),
    };
    let expected_severity = match severity {
        "err:" => Severity::Error,
        "warn:" => Severity::Warning,
        "weak:" => Severity::WeakWarning,
        _ => unreachable!(),
    };
    let expected_message = parts.next().unwrap();
    (expected_range, expected_severity, expected_message.to_string())
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
