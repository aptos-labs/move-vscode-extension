use crate::test_utils::completion_utils::do_single_completion;
use expect_test::expect;

#[test]
fn test_function_call_zero_args() {
    do_single_completion(
        // language=Move
        r#"
        module 0x1::m {
            fun frobnicate() {}
            fun main() {
                frob/*caret*/
            }
        }
    "#,
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun frobnicate() {}
                fun main() {
                    frobnicate()/*caret*/
                }
            }
        "#]],
    )
}

#[test]
fn test_function_call_one_arg() {
    do_single_completion(
        // language=Move
        r#"
        module 0x1::m {
            fun frobnicate(a: u8) {}
            fun main() {
                frob/*caret*/
            }
        }
    "#,
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun frobnicate(a: u8) {}
                fun main() {
                    frobnicate(/*caret*/)
                }
            }
        "#]],
    )
}

#[test]
fn test_function_call_with_parens() {
    do_single_completion(
        // language=Move
        r#"
        module 0x1::m {
            fun frobnicate() {}
            fun main() {
                frob/*caret*/()
            }
        }
    "#,
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun frobnicate() {}
                fun main() {
                    frobnicate/*caret*/()
                }
            }
        "#]],
    )
}

#[test]
fn test_function_call_one_arg_with_parens() {
    do_single_completion(
        // language=Move
        r#"
        module 0x1::m {
            fun frobnicate(a: u8) {}
            fun main() {
                frob/*caret*/(1)
            }
        }
    "#,
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun frobnicate(a: u8) {}
                fun main() {
                    frobnicate/*caret*/(1)
                }
            }
        "#]],
    )
}
