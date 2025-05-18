use crate::test_utils::completion_utils::{
    check_completion_exact, check_completions_contains, check_completions_with_prefix_exact,
    check_no_completions, do_single_completion,
};
use expect_test::expect;

#[rustfmt::skip]
#[test]
fn test_module_item_list_completion() {
    check_completion_exact(
        // language=Move
        r#"
module 0x1::m {
    fu/*caret*/
}
    "#,
        vec![
            "fun", "struct", "const", "enum", "use", "spec", "friend",
            "public", "entry", "native", "inline", "package",
        ],
    );
}

#[test]
fn test_top_level_completion_items() {
    check_completion_exact(
        // language=Move
        r#"
mod/*caret*/
    "#,
        vec!["module", "script", "spec"],
    );
}

#[test]
fn test_top_level_module_completion() {
    do_single_completion(
        // language=Move
        r#"
mod/*caret*/
    "#,
        // language=Move
        expect![[r#"
            module /*caret*/
        "#]],
    );
}

#[test]
fn test_complete_fun_keyword() {
    check_completions_contains(
        // language=Move
        r#"
module 0x1::m {
    public fu/*caret*/
}
    "#,
        vec!["fun"],
    );
}

// #[test]
// fn test_no_friend_after_public() {
//     check_no_completions(
//         // language=Move
//         r#"
// module 0x1::m {
//     public fri/*caret*/
// }
//     "#,
//     );
// }

// #[test]
// fn test_no_package_after_public() {
//     check_no_completions(
//         // language=Move
//         r#"
// module 0x1::m {
//     public pack/*caret*/
// }
//     "#,
//     );
// }

#[test]
fn test_expr_start_completion() {
    check_completions_contains(
        // language=Move
        r#"
module 0x1::m {
    fun main() {
        i/*caret*/
    }
}
    "#,
        vec!["if", "while", "let", "loop", "match", "for", "true", "false"],
    );
}

#[test]
fn test_no_completions_on_completed_let_keyword() {
    check_no_completions(
        // language=Move
        r#"
module 0x1::m {
    fun main() {
        let/*caret*/
    }
}
    "#,
    );
}

#[test]
fn test_complete_function_item() {
    check_completions_with_prefix_exact(
        // language=Move
        r#"
module 0x1::m {
    fun call() {}
    fun main() {
        ca/*caret*/
    }
}
    "#,
        vec!["call()"],
    );
}

#[test]
fn test_complete_function_parameter() {
    check_completions_with_prefix_exact(
        // language=Move
        r#"
module 0x1::m {
    fun main(my_param: u8) {
        my/*caret*/
    }
}
    "#,
        vec!["my_param"],
    );
}

#[test]
fn test_complete_variable_with_same_name_parameter() {
    check_completions_with_prefix_exact(
        // language=Move
        r#"
module 0x1::m {
    fun main(my_param: u8) {
        let my_param = 1;
        my/*caret*/
    }
}
    "#,
        vec!["my_param"],
    );
}

#[test]
fn test_complete_function_item_inserts_parens_zero_params() {
    do_single_completion(
        // language=Move
        r#"
module 0x1::m {
    fun call() {}
    fun main() {
        ca/*caret*/
    }
}
    "#,
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun call() {}
                fun main() {
                    call()/*caret*/
                }
            }
        "#]],
    );
}

#[test]
fn test_complete_function_item_inserts_parens_one_param() {
    do_single_completion(
        // language=Move
        r#"
module 0x1::m {
    fun call(a: u8) {}
    fun main() {
        ca/*caret*/
    }
}
    "#,
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun call(a: u8) {}
                fun main() {
                    call(/*caret*/)
                }
            }
        "#]],
    );
}

#[test]
fn test_no_keyword_completion_after_colon_colon_in_expr() {
    check_no_completions(
        // language=Move
        r#"
module 0x1::m {
    fun main() {
        Option::/*caret*/
    }
}
    "#,
    );
}

#[test]
fn test_local_type_completion() {
    check_completions_contains(
        // language=Move
        r#"
module 0x1::m {
    struct VestingContract { val: u8 }
    fun main() {
        Ves/*caret*/
    }
}
    "#,
        vec!["VestingContract"],
    );
}

#[test]
fn test_external_module_item_completion() {
    check_completions_contains(
        // language=Move
        r#"
module 0x1::v {
    public fun call1() {}
    public fun call2() {}
}
module 0x1::m {
    use 0x1::v;
    fun main() {
        v::ca/*caret*/
    }
}
    "#,
        vec!["call1()", "call2()"],
    );
}

// language=Move

#[test]
fn test_field_completion() {
    do_single_completion(
        r#"
module 0x1::m {
    struct S { field: u8 }
    struct T { s: S }
    fun main() {
        T[@0x1].s.fi/*caret*/;
    }
}
    "#,
        expect![[r#"
            module 0x1::m {
                struct S { field: u8 }
                struct T { s: S }
                fun main() {
                    T[@0x1].s.field;
                }
            }
        "#]],
    );
}

// language=Move

#[test]
fn test_method_call_completions() {
    do_single_completion(
        r#"
module 0x1::m {
    struct S { val: u8 }
    struct T { s: S }
    fun receiver(self: &mut S): u8 {
        self.val
    }
    fun main() {
        T[@0x1].s.rec/*caret*/;
    }
}
    "#,
        expect![[r#"
            module 0x1::m {
                struct S { val: u8 }
                struct T { s: S }
                fun receiver(self: &mut S): u8 {
                    self.val
                }
                fun main() {
                    T[@0x1].s.receiver()/*caret*/;
                }
            }
        "#]],
    );
}

#[test]
fn test_field_completion_with_substituted_type() {
    check_completions_contains(
        // language=Move
        r#"
module 0x1::m {
    struct S<T> { val: T }
    fun main(s: S<u8>) {
        s.va/*caret*/;
    }
}
    "#,
        vec!["val -> u8"],
    );
}

#[test]
fn test_method_completion_with_substituted_parameters() {
    check_completions_contains(
        // language=Move
        r#"
module 0x1::m {
    struct S<T> { val: T }
    fun receiver<T>(self: &mut S<T>, my_val: T): T {
        self.val
    }
    fun main(s: S<u8>) {
        s.rec/*caret*/;
    }
}
    "#,
        vec!["receiver(my_val: u8) -> u8"],
    );
}

// language=Move

#[test]
fn test_field_completion_from_dot() {
    do_single_completion(
        r#"
module 0x1::m {
    struct S { field: u8 }
    fun main() {
        S[@0x1]./*caret*/;
    }
}
    "#,
        expect![[r#"
            module 0x1::m {
                struct S { field: u8 }
                fun main() {
                    S[@0x1].field;
                }
            }
    "#]],
    );
}

#[test]
fn test_variable_completion_in_nested_block() {
    check_completions_contains(
        // language=Move
        r#"
module 0x1::m {
    fun main(var: u8) {
        {
            va/*caret*/
        }
    }
}
    "#,
        vec!["var"],
    );
}

#[test]
fn test_variable_completion_in_if_block_after_incomplete_call_expr() {
    check_completions_contains(
        // language=Move
        r#"
module 0x1::m {
    fun main(var: u8) {
        if (true) {
            call(;
            va/*caret*/
        }
    }
}
    "#,
        vec!["var"],
    );
}

#[test]
fn test_for_each_ref_lambda_parameter() {
    check_completions_contains(
        // language=Move
        r#"
module std::option {
    struct Option<Element> has copy, drop, store {
        vec: vector<Element>
    }
    public inline fun for_each_ref<Element>(self: &Option<Element>, f: |&Element|) {
    }
}
module std::asset {
    use std::option::Option;
    struct FunctionInfo has copy, drop, store {
        module_address: address,
    }
    public fun main(function: Option<FunctionInfo>) {
        function.for_each_ref(|function| {
            function.mod/*caret*/;
        })
    }
}
    "#,
        vec!["module_address -> address"],
    );
}

#[test]
fn test_no_completion_after_single_colon_in_use() {
    check_no_completions(
        // language=Move
        r#"
module std::option {
    use aptos_std:/*caret*/
}
    "#,
    );
}

#[test]
fn test_no_completion_after_double_colon_colon() {
    check_no_completions(
        // language=Move
        r#"
module std::option {
    use aptos_std:::/*caret*/
}
    "#,
    );
}
