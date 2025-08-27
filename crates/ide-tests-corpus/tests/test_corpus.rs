// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

#![allow(unused)]

use codespan_reporting::diagnostic::{Label, LabelStyle};
use codespan_reporting::term;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use expect_test::{Expect, expect, expect_file};
use ide_db::Severity;
use ide_db::assists::AssistResolveStrategy;
use ide_diagnostics::config::DiagnosticsConfig;
use ide_diagnostics::diagnostic::Diagnostic;
use ide_tests::ide_test_utils::diagnostics::check_diagnostics;
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

    let (file_id, _) = test_state.file_with_caret("/*caret*/");

    let mut diagnostics_config = DiagnosticsConfig::test_sample();
    diagnostics_config.disabled = vec![
        "unused-variable",
        "replace-with-method-call",
        "replace-with-compound-expr",
        "replace-with-index-expr",
        "redundant-cast",
    ]
    .into_iter()
    .map(|it| it.to_string())
    .collect();

    let diagnostics = test_state
        .analysis()
        .full_diagnostics(&diagnostics_config, AssistResolveStrategy::None, file_id)
        .unwrap();
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

fn main() {}

// datatest_stable::harness! {
//     { test = test_diagnostics, root = "/home/mkurnikov/code/move-fuzzing/old-sources", pattern = r"^.*\.move$" },
//     // { test = test_diagnostics, root = "/home/mkurnikov/code/move-fuzzing-llm", pattern = r"^.*\.move$" },
// }

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
