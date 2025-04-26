use crate::assert_eq_text;
use ide::test_utils::{get_all_marked_positions, get_first_marked_position};
use ide::Analysis;
use ide_db::assists::{Assist, AssistResolveStrategy};
use ide_db::Severity;
use ide_diagnostics::config::DiagnosticsConfig;
use ide_diagnostics::diagnostic::Diagnostic;
use lang::nameres::scope::VecExt;
use line_index::LineIndex;
use std::fmt::Debug;
use std::iter;
use syntax::files::FileRange;
use syntax::TextRange;
use vfs::FileId;

pub fn check_diagnostics(source: &str) {
    let (_, file_id, mut diagnostics) = get_diagnostics(source);
    let mut exps = get_expected_diagnostics(source, file_id);
    let mut missing_exps = vec![];
    loop {
        if let Some((range, severity, message)) = exps.last().cloned() {
            if let Some(indx) = diagnostics.iter().position(|x| x.range == range) {
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
    // assert!(diagnostics.is_empty(), "Extra diagnostics: {:#?}", diagnostics);
}

pub fn check_diagnostic_and_fix(before: &str, after: &str) {
    let (_, file_id, mut diagnostic) = get_diagnostics(before);
    let diag = diagnostic.pop().expect("diagnostics expected, but none returned");

    let (exp_range, exp_severity, exp_message) = get_expected_diagnostics(before, file_id)
        .pop()
        .expect("missing diagnostic mark");
    assert_eq!(diag.range, exp_range);
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

fn get_expected_diagnostics(source: &str, file_id: FileId) -> Vec<(FileRange, Severity, String)> {
    let marked_positions = get_all_marked_positions(source, "//^");

    let mut exps = vec![];
    for marked in marked_positions {
        let mut parts = marked.line.splitn(3, " ");
        let prefix = parts.next().unwrap();
        let severity = parts.next().unwrap();
        let len = prefix.trim_start_matches("//").len();
        let expected_range = FileRange {
            file_id,
            range: TextRange::at(marked.item_offset, (len as u32).into()),
        };
        let expected_severity = match severity {
            "err:" => Severity::Error,
            "warn:" => Severity::Warning,
            "weak:" => Severity::WeakWarning,
            _ => unreachable!("unknown severity {:?}", severity),
        };
        let expected_message = parts.next().unwrap();
        exps.push((expected_range, expected_severity, expected_message.to_string()));
    }
    exps
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
        let s = diagnostic_in_file(source, &d);
        println!("{}", s);
    }
    println!("======================================");

    panic!("Extra diagnostics available");
}

fn diagnostic_in_file(source: &str, diagnostic: &Diagnostic) -> String {
    let line_index = LineIndex::new(source);

    let text_range = diagnostic.range.range;
    let lc_start = line_index.line_col(text_range.start());
    let lc_end = line_index.line_col(text_range.end());

    let prefix = iter::repeat_n(" ", (lc_start.col - 2) as usize)
        .collect::<Vec<_>>()
        .join("");
    let range = iter::repeat_n("^", (lc_end.col - lc_start.col) as usize)
        .collect::<Vec<_>>()
        .join("");
    let message = diagnostic.message.clone();
    let line = format!("{prefix}//{range} {message}");

    let mut lines = source.lines().collect::<Vec<_>>();
    lines.insert((lc_start.line + 1) as usize, &line);
    lines.join("\n")
}
