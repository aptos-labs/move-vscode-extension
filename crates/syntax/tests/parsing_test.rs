use std::path::Path;
use std::{env, fs, panic};
use syntax::{AstNode, SourceFile};
use test_utils::{apply_error_marks, ErrorMark};

fn test_parse_file(fpath: &Path, allow_errors: bool) -> datatest_stable::Result<()> {
    let input = fs::read_to_string(fpath).unwrap();

    let parse = SourceFile::parse(&input);
    let file = parse.tree();

    if env::var("FUZZ").is_ok() {
        let mut modified_input = input.clone();
        while !modified_input.is_empty() {
            modified_input.pop();
            let res = panic::catch_unwind(|| SourceFile::parse(&modified_input));
            match res {
                Ok(_) => continue,
                Err(err) => {
                    println!("modified_input:\n{}", &modified_input);
                    panic!("{:?}", err);
                }
            }
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
        })
        .collect();
    let input_with_marks = apply_error_marks(&input, marks);

    let expected_output = if output_fpath.exists() {
        let existing = fs::read_to_string(&output_fpath).unwrap();
        Some(existing)
    } else {
        None
    };

    let expected_errors_output = fs::read_to_string(&errors_fpath).ok();

    if env::var("UB").is_ok() {
        // generate new files
        fs::write(&output_fpath, &actual_output).unwrap();
        if allow_errors {
            fs::write(errors_fpath, input_with_marks.clone()).unwrap();
        }
    }

    pretty_assertions::assert_eq!(&expected_output.unwrap_or("".to_string()), &actual_output);

    if !errors.is_empty() {
        if allow_errors {
            pretty_assertions::assert_eq!(
                &expected_errors_output.unwrap_or("".to_string()),
                &input_with_marks
            );
        } else {
            panic!("errors are not expected: \n {}", input_with_marks)
        }
    }

    Ok(())
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
