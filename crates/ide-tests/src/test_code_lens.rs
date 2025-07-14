use expect_test::{Expect, expect};
use ide::annotations::{Annotation, AnnotationConfig, AnnotationLocation};
use test_utils::fixtures;

const DEFAULT_CONFIG: AnnotationConfig = AnnotationConfig {
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
fn test_annotate_fun() {
    check_code_lens(
        // language=Move
        r#"
module std::m {
    fun main() {}
}
spec std::m {
    spec main {
        assert 1 == 1;
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
                        item_specs: Some(
                            [
                                NavigationTarget {
                                    file_id: FileId(
                                        1,
                                    ),
                                    full_range: 55..95,
                                    focus_range: 60..64,
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
