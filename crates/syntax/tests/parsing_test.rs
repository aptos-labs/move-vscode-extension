use std::path::Path;
use std::{env, fs};
use syntax::{AstNode, SourceFile};

fn test_parse_file(fpath: &Path, allow_errors: bool) -> datatest_stable::Result<()> {
    let input = fs::read_to_string(fpath).unwrap();

    let parse = SourceFile::parse(&input);
    let file = parse.tree();

    let actual_output = format!("{:#?}", file.syntax());
    let output_fpath = fpath.with_extension("txt");

    let expected_output = if output_fpath.exists() {
        let existing = fs::read_to_string(&output_fpath).unwrap();
        Some(existing)
    } else {
        None
    };
    if env::var("UB").is_ok() {
        // generate new files
        fs::write(output_fpath, &actual_output).unwrap();
    }

    pretty_assertions::assert_eq!(&expected_output.unwrap_or("".to_string()), &actual_output);

    let errors = parse.errors();
    if !allow_errors && !errors.is_empty() {
        println!("{:#?}", errors);
        // println!("{}", &actual_output);
        panic!("errors are not expected")
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
    { test = test_complete, root = "tests/complete", pattern = r"^.*\.move" },
    { test = test_partial, root = "tests/partial", pattern = r"^.*\.move" },
}
