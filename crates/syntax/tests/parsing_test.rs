// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use std::path::Path;
use std::{env, fs, panic};
use stdext::line_endings::LineEndings;
use syntax::{AstNode, SourceFile, algo, ast};
use test_utils::{SourceMark, apply_source_marks, fixtures};

fn test_parse_file(input_fpath: &Path, allow_errors: bool) -> datatest_stable::Result<()> {
    let (input, line_endings) = fs_read_file(input_fpath).unwrap();
    // let (input, line_endings) = LineEndings::normalize(input);

    let parse = SourceFile::parse(&input);
    let file = parse.tree();

    if env::var("FUZZ").is_ok() {
        let mut modified_input = input.clone();
        while !modified_input.is_empty() {
            run_fuzzer_once(&mut modified_input);
        }
    }

    let output_fpath = input_fpath.with_extension("").with_extension("txt");
    let errors_fpath = input_fpath.with_extension("").with_extension("exp");

    let syntax_errors = parse.errors();

    let mut error_marks = vec![];
    for syntax_error in syntax_errors.iter() {
        if let Some(error) =
            algo::find_node_at_offset::<ast::AstError>(file.syntax(), syntax_error.range().start())
        {
            error_marks.push(SourceMark::at_range(
                error.syntax().text_range(),
                syntax_error.to_string(),
            ));
            continue;
        }
        error_marks.push(SourceMark::at_range(
            syntax_error.range(),
            syntax_error.to_string(),
        ));
    }
    let error_output = apply_source_marks(&input, error_marks);

    let actual_output = format!("{:#?}", file.syntax());
    let expected_output = if output_fpath.exists() {
        let (existing_expected_output, _) = fs_read_file(&output_fpath).unwrap();
        Some(existing_expected_output)
    } else {
        None
    };

    let expected_errors_output = fs_read_file(&errors_fpath);

    if env::var("UB").is_ok() {
        // generate new files
        fs_write_file(&output_fpath, &actual_output, line_endings).unwrap();
        if allow_errors {
            fs_write_file(errors_fpath, &error_output, line_endings).unwrap();
        }
    }

    // check whether it can be highlighted without crashes
    highlight_file(input.clone());

    pretty_assertions::assert_eq!(&expected_output.unwrap_or("".to_string()), &actual_output);

    if !syntax_errors.is_empty() {
        if allow_errors {
            pretty_assertions::assert_eq!(
                &expected_errors_output
                    .unwrap_or(("".to_string(), LineEndings::Unix))
                    .0,
                &error_output
            );
        } else {
            panic!("errors are not expected: \n {}", error_output)
        }
    }

    Ok(())
}

fn fs_read_file(fpath: impl AsRef<Path>) -> Option<(String, LineEndings)> {
    fs::read_to_string(&fpath)
        .ok()
        .map(|it| LineEndings::normalize(it))
}

fn fs_write_file(fpath: impl AsRef<Path>, contents: &String, line_endings: LineEndings) -> Option<()> {
    let contents = line_endings.map(contents.clone());
    fs::write(&fpath, contents).ok()
}

fn run_fuzzer_once(modified_input: &mut String) {
    // modified_input.pop();
    // if !modified_input.is_empty() && modified_input.is_char_boundary(0) {
    //     modified_input.remove(0);
    // }
    let rand_idx = rand::random_range(0..modified_input.len());
    if modified_input.is_char_boundary(rand_idx) {
        modified_input.remove(rand_idx);
    }
    let parsed = panic::catch_unwind(|| SourceFile::parse(&modified_input));
    if let Err(err) = parsed {
        println!("modified_input:\n{}", &modified_input);
        println!("==========");
        panic!("parse error \n{:?}", err);
    }
    let highlighted = panic::catch_unwind(|| highlight_file(modified_input.clone()));
    if let Err(err) = highlighted {
        println!("modified_input:\n{}", &modified_input);
        println!("==========");
        panic!("highlight error \n{:?}", err);
    }
}

fn highlight_file(input: String) -> String {
    let (analysis, file_id) = fixtures::from_single_file(input.clone());
    let html_output = analysis
        .highlight_as_html(file_id, vec!["unresolved_reference".to_string()])
        .unwrap();
    html_output
}

fn test_complete(fpath: &Path) -> datatest_stable::Result<()> {
    test_parse_file(fpath, false)
}

fn test_partial(fpath: &Path) -> datatest_stable::Result<()> {
    test_parse_file(fpath, true)
}

datatest_stable::harness! {
    { test = test_complete, root = "tests/complete", pattern = r"^.*\.move$" },
    { test = test_partial, root = "tests/partial", pattern = r"^.*\.move$" },
}
