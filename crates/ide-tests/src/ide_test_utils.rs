// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use expect_test::Expect;
use syntax::TextSize;
use syntax::files::FilePosition;
use syntax::pretty_print::{SourceMark, apply_source_marks};
use test_utils::{fixtures, get_and_replace_caret};

pub mod completion_utils;
pub mod diagnostics;

pub fn check_signature_info(source: &str, expect: Expect) {
    let (source, offset) = get_and_replace_caret(source, "/*caret*/");
    let (analysis, file_id) = fixtures::from_single_file(source.to_string());

    let signature_help = analysis
        .signature_help(FilePosition { file_id, offset })
        .unwrap()
        .expect("missing signature info");

    let mut signature_text = signature_help.signature.clone();

    if let Some(active_parameter_range) = signature_help
        .active_parameter
        .and_then(|it| signature_help.parameter_range(it))
    {
        let indent = ">>";
        let mark = SourceMark::at_range(active_parameter_range + TextSize::of(indent), "");
        signature_text = apply_source_marks(&format!("{indent}{signature_text}"), vec![mark]);
    }

    expect.assert_eq(&signature_text)
}
