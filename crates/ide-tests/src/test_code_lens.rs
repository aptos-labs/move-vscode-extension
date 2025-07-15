use expect_test::{Expect, expect};
use ide::annotations::{Annotation, AnnotationConfig, AnnotationLocation};
use test_utils::fixtures;

const DEFAULT_CONFIG: AnnotationConfig = AnnotationConfig {
    annotate_runnables: true,
    annotate_fun_specs: true,
    location: AnnotationLocation::AboveName,
};

fn check_code_lens(source: &str, expect: Expect) {
    let (analysis, file_id) = fixtures::from_single_file(source.to_string());

    let annotations: Vec<Annotation> = analysis
        .annotations(&DEFAULT_CONFIG, file_id)
        .unwrap()
        .into_iter()
        .map(|annotation| analysis.resolve_annotation(annotation).unwrap())
        .collect();

    expect.assert_debug_eq(&annotations);
}

#[test]
fn test_annotate_specified_fun() {
    check_code_lens(
        // language=Move
        r#"
module std::m {
    fun main() {}
}
spec std::m {
    spec main() {
        assert 1 == 1;
    }
    spec main() {
        assert 2 == 2;
    }
}
    "#,
        expect![[r#"
            [
                Annotation {
                    range: 25..29,
                    kind: HasSpecs {
                        pos: FilePosition {
                            file_id: FileId(
                                1,
                            ),
                            offset: 25,
                        },
                        item_spec_refs: Some(
                            [
                                NavigationTarget {
                                    file_id: FileId(
                                        1,
                                    ),
                                    full_range: 60..64,
                                    name: "main",
                                    kind: Field,
                                },
                                NavigationTarget {
                                    file_id: FileId(
                                        1,
                                    ),
                                    full_range: 107..111,
                                    name: "main",
                                    kind: Field,
                                },
                            ],
                        ),
                    },
                },
            ]
        "#]],
    );
}

#[test]
fn test_annotate_test_fun() {
    check_code_lens(
        // language=Move
        r#"
module std::m {
    #[test]
    fun test_main() {

    }
}
    "#,
        expect![[r#"
            [
                Annotation {
                    range: 37..46,
                    kind: Runnable(
                        Runnable {
                            nav_item: NavigationTarget {
                                file_id: FileId(
                                    1,
                                ),
                                full_range: 21..57,
                                focus_range: 37..46,
                                name: "test_main",
                                kind: Function,
                            },
                            test_path: "m::test_main",
                        },
                    ),
                },
            ]
        "#]],
    );
}
