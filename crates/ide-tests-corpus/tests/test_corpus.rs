use codespan_reporting::diagnostic::{Label, LabelStyle};
use codespan_reporting::term;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use expect_test::{Expect, expect, expect_file};
use ide_db::Severity;
use ide_db::assists::AssistResolveStrategy;
use ide_diagnostics::config::DiagnosticsConfig;
use ide_diagnostics::diagnostic::Diagnostic;
use ide_tests::ide_test_utils::diagnostics::{check_diagnostics, check_diagnostics_in_file};
use ide_tests::init_tracing_for_test;
use paths::{AbsPath, AbsPathBuf};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use test_utils::fixtures;
use test_utils::fixtures::test_state::{TestPackageFiles, named_with_deps};

fn test_diagnostics(fpath: &Path) -> datatest_stable::Result<()> {
    let source = fs::read_to_string(fpath).unwrap();
    let source = format!(
        r#"
/*caret*/
{source}
    "#
    );

    let test_state = fixtures::from_multiple_files_on_tmpfs(vec![named_with_deps(
        "Fuzzing",
        // language=TOML
        r#"
[dependencies.AptosFramework]
git = "https://github.com/aptos-labs/aptos-framework.git"
rev = "mainnet"
subdir = "aptos-framework"
"#,
        // language=Move
        &format!(
            r#"
//- main.move
{source}
"#
        ),
    )]);

    let mut disabled_codes = HashSet::new();
    disabled_codes.insert("unused-variable".to_string());
    disabled_codes.insert("replace-with-method-call".to_string());
    disabled_codes.insert("replace-with-compound-expr".to_string());

    let (file_id, _) = test_state.file_with_caret("/*caret*/");

    let diagnostics = test_state
        .analysis()
        .full_diagnostics(
            &DiagnosticsConfig::test_sample(),
            AssistResolveStrategy::None,
            file_id,
        )
        .unwrap();

    // let skipped_messages = vec![];
    let skipped_messages = vec!["Assigned expr of type '()'", "Unresolved reference `field"];

    let diagnostics = diagnostics
        .into_iter()
        .filter(|diag| {
            if disabled_codes.contains(&diag.code.as_str().to_string()) {
                return false;
            }
            // if skipped_messages.iter().any(|it| diag.message.contains(it)) {
            //     return false;
            // }
            true
        })
        .collect::<Vec<_>>();
    for diagnostic in diagnostics.clone() {
        print_diagnostic(
            &source,
            AbsPathBuf::assert_utf8(fpath.to_path_buf()).as_path(),
            diagnostic,
        );
    }

    if !diagnostics.is_empty() {
        panic!("{} diagnostics found", diagnostics.len());
    }

    Ok(())
}

datatest_stable::harness! {
    { test = test_diagnostics, root = "/home/mkurnikov/code/move-fuzzing-llm", pattern = r"^.*\.move$" },
}

fn print_diagnostic(file_text: &str, file_path: &AbsPath, diagnostic: Diagnostic) {
    let Diagnostic {
        code,
        message,
        range,
        severity,
        ..
    } = diagnostic;

    let severity = match severity {
        Severity::Error => codespan_reporting::diagnostic::Severity::Error,
        Severity::Warning => codespan_reporting::diagnostic::Severity::Warning,
        Severity::WeakWarning => codespan_reporting::diagnostic::Severity::Note,
        _ => {
            return;
        }
    };

    let mut files = codespan_reporting::files::SimpleFiles::new();
    let file_id = files.add(file_path.to_string(), file_text.to_string());

    let codespan_diagnostic = codespan_reporting::diagnostic::Diagnostic::new(severity)
        .with_label(Label::new(LabelStyle::Primary, file_id, range.range))
        .with_code(code.as_str())
        .with_message(message);

    let term_config = term::Config::default();
    let mut stderr = StandardStream::stderr(ColorChoice::Auto);
    term::emit(&mut stderr, &term_config, &files, &codespan_diagnostic).unwrap();
}
