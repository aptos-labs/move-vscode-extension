// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use expect_test::{Expect, expect};
use ide::annotations::{AnnotationConfig, AnnotationKind, AnnotationLocation};
use syntax::pretty_print::{SourceMark, apply_source_marks};
use test_utils::fixtures::test_state::named;
use test_utils::{fixtures, remove_marks};

const DEFAULT_CONFIG: AnnotationConfig = AnnotationConfig {
    annotate_runnables: true,
    annotate_fun_specs: true,
    location: AnnotationLocation::AboveName,
};

fn check_code_lens(expect: Expect) {
    let source = stdx::trim_indent(expect.data());
    let trimmed_source = remove_marks(&source, "//^");

    let test_state = fixtures::from_multiple_files_on_tmpfs(vec![named("TestPackage", trimmed_source)]);

    let mut res = String::new();
    for (file_id, file_text) in test_state.all_move_files() {
        let annotations = test_state
            .analysis()
            .annotations(&DEFAULT_CONFIG, file_id)
            .unwrap()
            .into_iter()
            .map(|annotation| test_state.analysis().resolve_annotation(annotation).unwrap())
            .collect::<Vec<_>>();
        let mut marks = vec![];
        for annotation in annotations {
            let label = match annotation.kind {
                AnnotationKind::Runnable(runnable) => runnable.label(),
                AnnotationKind::HasSpecs { .. } => "has specs".to_string(),
            };
            marks.push(SourceMark::at_range(annotation.range, label));
        }
        let file_text_with_marks = apply_source_marks(&file_text, marks);
        res.push_str("//- ");
        res.push_str(&test_state.relpath(file_id));
        res.push_str("\n");
        res.push_str(&file_text_with_marks);
    }

    expect.assert_eq(&res);
}

#[test]
fn test_annotate_specified_fun() {
    check_code_lens(
        // language=Move
        expect![[r#"
            //- /main.spec.move
            spec std::m {
               //^^^^^^ prove mod m
                spec main() {
                   //^^^^ prove fun m::main
                    assert 1 == 1;
                }
                spec main() {
                   //^^^^ prove fun m::main
                    assert 2 == 2;
                }
            }
            //- /main.move
            module std::m {
                fun main() {}
                  //^^^^ has specs
            }
        "#]],
    );
}

#[test]
fn test_annotate_test_module_and_test_fun() {
    check_code_lens(
        // language=Move
        expect![[r#"
            //- /main.move
            module std::m {
                      //^ test m::
                #[test]
                fun test_main() {
                  //^^^^^^^^^ test m::test_main

                }
            }
        "#]],
    );
}

#[test]
fn test_no_test_module_annotation_if_no_test_functions() {
    check_code_lens(
        // language=Move
        expect![[r#"
            //- /main.move
            module std::m {
                fun main() {
                }
            }
        "#]],
    );
}

#[test]
fn test_annotate_item_spec_for_function() {
    check_code_lens(
        // language=Move
        expect![[r#"
            //- /main.move
            module std::m {
                fun main() {
                  //^^^^ has specs
                }
                spec main {
                   //^^^^ prove fun m::main
                }
            }
        "#]],
    );
}

#[test]
fn test_annotate_item_spec_for_function_in_module_spec() {
    check_code_lens(
        // language=Move
        expect![[r#"
            //- /main.spec.move
            spec std::m {
               //^^^^^^ prove mod m
                spec main {
                   //^^^^ prove fun m::main
                }
            }
            //- /main.move
            module std::m {
                fun main() {
                  //^^^^ has specs
                }
            }
        "#]],
    );
}
