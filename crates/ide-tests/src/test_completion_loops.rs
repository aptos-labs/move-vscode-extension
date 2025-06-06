use crate::ide_test_utils::completion_utils::{check_no_completions, do_single_completion};
use expect_test::expect;

#[test]
fn test_no_break_keyword_outside_loop() {
    check_no_completions(
        // language=Move
        r#"
        module 0x1::m {
            fun main() {
                bre/*caret*/
            }
        }
    "#,
    )
}

#[test]
fn test_break_keyword_inside_loop() {
    do_single_completion(
        // language=Move
        r#"
        module 0x1::m {
            fun main() {
                loop {
                    bre/*caret*/
                }
            }
        }
    "#,
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    loop {
                        break/*caret*/
                    }
                }
            }
        "#]],
    )
}

#[test]
fn test_loop_label_for_break() {
    do_single_completion(
        // language=Move
        r#"
        module 0x1::m {
            fun main() {
                'label: loop {
                    break 'la/*caret*/;
                }
            }
        }
    "#,
        // language=Move
        expect![[r#"
        module 0x1::m {
            fun main() {
                'label: loop {
                    break 'label;
                }
            }
        }
    "#]],
    )
}

#[test]
fn test_loop_label_for_break_from_single_quote() {
    do_single_completion(
        // language=Move
        r#"
        module 0x1::m {
            fun main() {
                'label: loop {
                    break '/*caret*/;
                }
            }
        }
    "#,
        // language=Move
        expect![[r#"
        module 0x1::m {
            fun main() {
                'label: loop {
                    break 'label;
                }
            }
        }
    "#]],
    )
}
