use crate::ide_test_utils::completion_utils::{
    check_completions, check_no_completions, do_single_completion,
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
                    let a: CoinType/*caret*/
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
    check_completions(
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
        expect![[r#"
            [
                "create_friend()",
                "create()",
            ]"#]],
    )
}

#[test]
fn test_public_and_public_script_completions_for_script() {
    check_completions(
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
        expect![[r#"
            [
                "create_script()",
                "create()",
            ]"#]],
    )
}

#[test]
fn test_self_completion() {
    check_completions(
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
        expect![[r#"
            [
                "create_friend()",
                "create_script()",
                "create()",
                "create_private()",
                "main()",
            ]"#]],
    )
}

#[test]
fn test_fq_completion_for_use() {
    do_single_completion(
        // language=Move
        r#"
        module 0x1::m1 {
            public fun call() {}
        }
        module 0x1::m2 {
            use 0x1::m1::c/*caret*/
        }
    "#,
        // language=Move
        expect![[r#"
            module 0x1::m1 {
                public fun call() {}
            }
            module 0x1::m2 {
                use 0x1::m1::call/*caret*/
            }
        "#]],
    )
}

#[test]
fn test_fq_completion_for_reference_expr() {
    do_single_completion(
        // language=Move
        r#"
        module 0x1::m1 {
            public fun call() {}
        }
        module 0x1::m2 {
            fun m() {
                0x1::m1::c/*caret*/
            }
        }
    "#,
        // language=Move
        expect![[r#"
            module 0x1::m1 {
                public fun call() {}
            }
            module 0x1::m2 {
                fun m() {
                    0x1::m1::call()/*caret*/
                }
            }
        "#]],
    )
}

// todo: type annotation
// #[test]
// fn test_insert_angle_brackets_for_borrow_global_mut_if_not_inferrable_from_the_context() {
//     do_single_completion(
//         // language=Move
//         r#"
//         module 0x1::m {
//             fun m() {
//                 let a = borrow_global_/*caret*/
//             }
//         }
//     "#,
//         // language=Move
//         expect![[r#"
//             module 0x1::m {
//                 fun m() {
//                     let a = borrow_global_mut(/*caret*/)
//                 }
//             }
//         "#]],
//     )
// }

#[test]
fn test_do_not_insert_parens_if_angle_brackets_exist() {
    do_single_completion(
        // language=Move
        r#"
        module 0x1::m {
            fun m() {
                borrow_global_m/*caret*/<u8>(@0x1);
            }
        }
    "#,
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun m() {
                    borrow_global_mut/*caret*/<u8>(@0x1);
                }
            }
        "#]],
    )
}

// todo: auto-import
// #[test]
// fn test_function_in_path_position_with_auto_import() {
//     do_single_completion(
//         // language=Move
//         r#"
//     module 0x1::signer {
//         public fun address_of(s: &signer): address { @0x1 }
//     }
//     module 0x1::m {
//         fun call() {
//             let a = 1;
//             address_o/*caret*/
//         }
//     }
//     "#,
//         // language=Move
//         expect![[r#"
//             module 0x1::m {
//                 fun m() {
//                     borrow_global_mut/*caret*/<u8>(@0x1);
//                 }
//             }
//         "#]],
//     )
// }

#[test]
fn test_test_only_function_completion_in_test_only_scope() {
    check_completions(
        // language=Move
        r#"
        module 0x1::minter {
            #[test_only]
            public fun get_weekly() {}
        }
        #[test_only]
        module 0x1::minter_tests {
            use 0x1::minter::get/*caret*/
        }
    "#,
        expect![[r#"
            [
                "get_weekly()",
            ]"#]],
    )
}

#[test]
fn test_test_only_function_completion_in_test_only_use_stmt_scope() {
    check_completions(
        // language=Move
        r#"
        module 0x1::minter {
            #[test_only]
            public fun get_weekly() {}
        }
        module 0x1::minter_tests {
            #[test_only]
            use 0x1::minter::get/*caret*/
        }
    "#,
        expect![[r#"
            [
                "get_weekly()",
            ]"#]],
    )
}

// todo: type annotation
// #[test]
// fn test_do_not_add_angle_brackets_if_type_is_inferrable_from_context() {
//     do_single_completion(
//         // language=Move
//         r#"
//     module 0x1::Event {
//         struct EventHandle<phantom E> {}
//
//         struct MyEvent {}
//         struct EventStore {
//             my_events: EventHandle<MyEvent>
//         }
//
//         fun new_event_handle<E>(): EventHandle<E> { EventHandle<E> {} }
//         fun call() {
//             EventStore { my_events: new_eve/*caret*/ };
//         }
//     }
//     "#,
//         // language=Move
//         expect![[""]],
//     )
// }

// todo: type annotation
// #[test]
// fn test_add_angle_brackets_if_untyped_let_pattern() {
//     do_single_completion(
//         // language=Move
//         r#"
//     module 0x1::main {
//         struct Coin<CoinType> {}
//         fun withdraw<CoinType>(): Coin<CoinType> { Coin<CoinType> {} }
//         fun main() {
//             let a = with/*caret*/;
//         }
//     }
//     "#,
//         // language=Move
//         expect![[""]],
//     )
// }

// todo: type annotation
// #[test]
// fn test_no_angle_brackets_for_function_with_generic_vector_param() {
//     do_single_completion(
//         // language=Move
//         r#"
//         module 0x1::m {
//             native public fun destroy_empty<Element>(v: vector<Element>);
//             fun main() {
//                 destroy/*caret*/
//             }
//         }
//     "#,
//         // language=Move
//         expect![[r#"
//         "#]],
//     )
// }

#[test]
fn test_complete_function_from_module_alias() {
    do_single_completion(
        // language=Move
        r#"
    module 0x1::string {
        public fun call() {}
    }
    module 0x1::main {
        use 0x1::string as mystring;
        fun main() {
            mystring::ca/*caret*/
        }
    }
    "#,
        // language=Move
        expect![[r#"
            module 0x1::string {
                public fun call() {}
            }
            module 0x1::main {
                use 0x1::string as mystring;
                fun main() {
                    mystring::call()/*caret*/
                }
            }
        "#]],
    )
}

#[test]
fn test_spec_function_completion_at_lhs_of_equality_expr() {
    do_single_completion(
        // language=Move
        r#"
        module 0x1::m {
            spec fun spec_some(a: u8): u8 { a }
            spec module {
                let a = 1;
                let b = 1;
                spec_/*caret*/ == spec_some(b);
            }
        }
    "#,
        // language=Move
        expect![[r#"
            module 0x1::m {
                spec fun spec_some(a: u8): u8 { a }
                spec module {
                    let a = 1;
                    let b = 1;
                    spec_some(/*caret*/) == spec_some(b);
                }
            }
        "#]],
    )
}

#[test]
fn test_spec_function_completion_at_rhs_of_equality_expr() {
    do_single_completion(
        // language=Move
        r#"
        module 0x1::m {
            spec fun spec_some(a: u8): u8 { a }
            spec module {
                let a = 1;
                let b = 1;
                spec_some(a) == spec_/*caret*/;
            }
        }
    "#,
        // language=Move
        expect![[r#"
            module 0x1::m {
                spec fun spec_some(a: u8): u8 { a }
                spec module {
                    let a = 1;
                    let b = 1;
                    spec_some(a) == spec_some(/*caret*/);
                }
            }
        "#]],
    )
}
