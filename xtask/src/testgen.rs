// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::codegen::{add_preamble, ensure_file_contents, reformat};
use quote::{format_ident, quote};
use std::fs;
use std::str::FromStr;
use stdx::itertools::Itertools;

pub fn generate() {
    generate_test_category(
        "resolve",
        quote! { use crate::resolve::check_resolve; },
        |test_name, code| {
            let test_name = format_ident!("test_{}", test_name);
            let code = generate_raw_string_literal(code);
            let test_case = quote! {
                #[test]
                fn #test_name() {
                    check_resolve(
                        #code
                    )
                }
            };
            test_case
        },
    );
    generate_test_category(
        "types",
        quote! { use crate::types::check_expr_type; },
        |test_name, code| {
            let test_name = format_ident!("test_{}", test_name);
            let code = generate_raw_string_literal(code);
            let test_case = quote! {
                #[test]
                fn #test_name() {
                    check_expr_type(
                        #code
                    )
                }
            };
            test_case
        },
    );
}

// fn generate_resolve_tests() {
//     let ide_tests_src_dir = crate::project_root().join("crates/ide-tests/src");
//     let ide_tests_resources_dir = crate::project_root().join("crates/ide-tests/resources");
//
//     let resolve_resources = ide_tests_resources_dir.join("resolve");
//     let groups = fs::read_dir(&resolve_resources).unwrap();
//     for group in groups {
//         let group_name = group.unwrap().file_name().to_string_lossy().to_string();
//         let group_tests_folder = resolve_resources.join(&group_name);
//
//         let mut test_cases = vec![];
//         let paths = fs::read_dir(group_tests_folder).unwrap();
//         for path in paths {
//             let test_path = path.unwrap().path();
//             let test_name = test_path.file_stem().unwrap().to_string_lossy().to_string();
//             let test_contents = fs::read_to_string(test_path).unwrap();
//             let test_case = generate_test_case(test_name.as_str(), test_contents.as_str());
//             test_cases.push(test_case);
//         }
//
//         let use_stmts = quote! { use crate::resolve::check_resolve; };
//
//         let mut output_file_contents = "".to_string();
//         output_file_contents += use_stmts.to_string().as_str();
//         output_file_contents += "\n\n";
//         output_file_contents += test_cases
//             .into_iter()
//             .map(|it| format!("// language=Move\n{}", it))
//             .join("\n\n")
//             .as_str();
//
//         let output_file_path = ide_tests_src_dir
//             .join("resolve")
//             .join(format!("test_resolve_{}.rs", group_name));
//         let final_contents = add_preamble("testgen", reformat(output_file_contents.to_string()));
//         ensure_file_contents(output_file_path.as_path(), final_contents.as_str());
//     }
// }

fn generate_test_category(
    test_category: &str,
    use_stmts: proc_macro2::TokenStream,
    generate_test_case: impl Fn(&str, &str) -> proc_macro2::TokenStream,
) {
    let ide_tests_src_dir = crate::project_root().join("crates/ide-tests/src");
    let ide_tests_resources_dir = crate::project_root().join("crates/ide-tests/resources");

    let category_resources = ide_tests_resources_dir.join(test_category);
    let category_groups = fs::read_dir(&category_resources).unwrap();
    for group in category_groups {
        let group_name = group.unwrap().file_name().to_string_lossy().to_string();
        let group_tests_folder = category_resources.join(&group_name);

        let mut test_cases = vec![];
        let paths = fs::read_dir(group_tests_folder).unwrap();
        for path in paths {
            let test_path = path.unwrap().path();
            let test_name = test_path.file_stem().unwrap().to_string_lossy().to_string();
            let test_contents = fs::read_to_string(test_path).unwrap();
            let test_case = generate_test_case(test_name.as_str(), test_contents.as_str());
            test_cases.push(test_case);
        }

        let mut output_file_contents = "".to_string();
        output_file_contents += use_stmts.to_string().as_str();
        output_file_contents += "\n\n";
        output_file_contents += test_cases
            .into_iter()
            .map(|it| format!("// language=Move\n{}", it))
            .join("\n\n")
            .as_str();

        let output_file_path = ide_tests_src_dir
            .join(test_category)
            .join(format!("test_{}_{}.rs", test_category, group_name));
        let final_contents = add_preamble("testgen", reformat(output_file_contents.to_string()));
        ensure_file_contents(output_file_path.as_path(), final_contents.as_str());
    }
}

// fn generate_test_case(test_name: &str, code: &str) -> proc_macro2::TokenStream {
//     let test_name = format_ident!("test_{}", test_name);
//     let code = generate_raw_string_literal(code);
//     let test_case = quote! {
//         #[test]
//         fn #test_name() {
//             check_resolve(
//                 #code
//             )
//         }
//     };
//     test_case
// }

fn generate_raw_string_literal(value: &str) -> proc_macro2::Literal {
    let wrapped = format!("r#\"\n{}\n\"#", value);
    proc_macro2::Literal::from_str(&wrapped).unwrap()
}
