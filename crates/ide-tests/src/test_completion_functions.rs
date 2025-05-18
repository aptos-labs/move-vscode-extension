use crate::test_utils::completion_utils::{
    check_completions_contains, check_completions_contains_expect, check_no_completions,
    do_single_completion,
};
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

// todo: needs type annotation
// #[test]
// fn test_generic_function_call_with_uninferrable_type_parameters() {
//     do_single_completion(
//         // language=Move
//         r#"
//         module 0x1::m {
//             fun frobnicate<T>() {}
//             fun main() {
//                 frob/*caret*/
//             }
//         }
//     "#,
//         // language=Move
//         expect![[r#"
//             module 0x1::m {
//                 fun frobnicate<T>() {}
//                 fun main() {
//                     frobnicate()/*caret*/
//                 }
//             }
//         "#]],
//     )
// }

#[test]
fn test_type_parameters_available_in_let_type_completion() {
    do_single_completion(
        // language=Move
        r#"
        module 0x1::m {
            fun main<CoinType>() {
                let a: Coi/*caret*/
            }
        }
    "#,
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main<CoinType>() {
                    let a: CoinType
                }
            }
        "#]],
    )
}

#[test]
fn test_no_function_completion_in_type_position() {
    check_no_completions(
        // language=Move
        r#"
        module 0x1::m {
            public fun create() {}
        }
        module 0x1::main {
            fun main(a: 0x1::n::cr/*caret*/) {}
        }
    "#,
    )
}

#[test]
fn test_public_friend_functions_for_fq_completion() {
    check_completions_contains_expect(
        // language=Move
        r#"
        module 0x1::m {
            friend 0x1::main;
            public(friend) fun create_friend() {}
            public fun create() {}
        }
        module 0x1::main {
            fun main() {
                0x1::m::cr/*caret*/
            }
        }
    "#,
        expect![[r#"["create_friend()", "create()"]"#]],
    )
}

#[test]
fn test_public_and_public_script_completions_for_script() {
    check_completions_contains_expect(
        // language=Move
        r#"
        module 0x1::m {
            public(script) fun create_script() {}
            public fun create() {}
        }
        script {
            fun main() {
                0x1::m::cr/*caret*/
            }
        }
    "#,
        expect![[r#"["create_script()", "create()"]"#]],
    )
}

#[test]
fn test_self_completion() {
    check_completions_contains_expect(
        // language=Move
        r#"
        module 0x1::m {
            public(friend) fun create_friend() {}
            public(script) fun create_script() {}
            public fun create() {}
            fun create_private() {}

            fun main() {
                Self::/*caret*/
            }
        }
    "#,
        expect![[r#"["create_friend()", "create_script()", "create()", "create_private()", "main()"]"#]],
    )
}
