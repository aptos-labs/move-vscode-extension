use std::path::Path;
use std::{env, fs, panic};
use syntax::line_index::LineEndings;
use syntax::{AstNode, SourceFile};
use test_utils::{apply_error_marks, fixtures, ErrorMark};

fn test_parse_file(fpath: &Path, allow_errors: bool) -> datatest_stable::Result<()> {
    let input = fs_read_file(fpath).unwrap();

    let parse = SourceFile::parse(&input);
    let file = parse.tree();

    if env::var("FUZZ").is_ok() {
        let mut modified_input = input.clone();
        while !modified_input.is_empty() {
            run_fuzzer_once(&mut modified_input);
        }
    }

    let actual_output = format!("{:#?}", file.syntax());
    let output_fpath = fpath.with_extension("").with_extension("txt");
    let errors_fpath = fpath.with_extension("").with_extension("exp");

    let errors = parse.errors();
    let marks = errors
        .iter()
        .map(|it| ErrorMark {
            text_range: it.range(),
            message: it.to_string(),
            custom_symbol: None,
        })
        .collect();
    let error_output = apply_error_marks(&input, marks);

    let expected_output = output_fpath
        .exists()
        .then_some(fs_read_file(&output_fpath).unwrap());
    let expected_errors_output = fs_read_file(&errors_fpath);

    if env::var("UB").is_ok() {
        // generate new files
        fs::write(&output_fpath, &actual_output).unwrap();
        if allow_errors {
            fs::write(errors_fpath, error_output.clone()).unwrap();
        }
        // fs::write(&html_fpath, &html_output).unwrap();
    }

    // check whether it can be highlighted without crashes
    highlight_file(input.clone());

    pretty_assertions::assert_eq!(&expected_output.unwrap_or("".to_string()), &actual_output);

    // pretty_assertions::assert_eq!(&expected_html_output.unwrap_or("".to_string()), &html_output);

    if !errors.is_empty() {
        if allow_errors {
            pretty_assertions::assert_eq!(
                &expected_errors_output.unwrap_or("".to_string()),
                &error_output
            );
        } else {
            panic!("errors are not expected: \n {}", error_output)
        }
    }

    Ok(())
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

fn fs_read_file(path: impl AsRef<Path>) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    let (normalized_text, _) = LineEndings::normalize(text);
    Some(normalized_text)
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
