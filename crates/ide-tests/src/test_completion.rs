// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ide_test_utils::completion_utils::{
    check_completions, check_no_completions, do_single_completion,
};
use expect_test::expect;

#[test]
fn test_module_item_list_completion() {
    check_completions(
        // language=Move
        r#"
module 0x1::m {
    /*caret*/
}
    "#,
        expect![[r#"
            [
                "use",
                "fun",
                "struct",
                "const",
                "enum",
                "spec",
                "friend",
                "public",
                "native",
                "entry",
                "inline",
                "package",
            ]"#]],
    );
}

#[test]
fn test_top_level_completion_items() {
    check_completions(
        // language=Move
        r#"
/*caret*/
    "#,
        expect![[r#"
            [
                "module",
                "script",
                "spec",
            ]"#]],
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
    check_completions(
        // language=Move
        r#"
module 0x1::m {
    public fu/*caret*/
}
    "#,
        expect![[r#"
            [
                "fun",
            ]"#]],
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
    check_completions(
        // language=Move
        r#"
module 0x1::m {
    fun main() {
        i/*caret*/
    }
}
    "#,
        expect![[r#"
            [
                "if",
            ]"#]],
    );
}

#[test]
fn test_on_let_keyword_only_let() {
    check_completions(
        // language=Move
        r#"
module 0x1::m {
    fun main() {
        let/*caret*/
    }
}
    "#,
        expect![[r#"
        [
            "let",
        ]"#]],
    );
}

#[test]
fn test_complete_function_item() {
    check_completions(
        // language=Move
        r#"
module 0x1::m {
    fun call() {}
    fun main() {
        ca/*caret*/
    }
}
    "#,
        expect![[r#"
            [
                "call()",
            ]"#]],
    );
}

#[test]
fn test_complete_function_parameter() {
    check_completions(
        // language=Move
        r#"
module 0x1::m {
    fun main(my_param: u8) {
        my/*caret*/
    }
}
    "#,
        expect![[r#"
            [
                "my_param -> u8",
            ]"#]],
    );
}

#[test]
fn test_complete_variable_with_same_name_parameter() {
    check_completions(
        // language=Move
        r#"
module 0x1::m {
    fun main(my_param: u8) {
        let my_param = 1;
        my/*caret*/
    }
}
    "#,
        expect![[r#"
            [
                "my_param -> integer",
            ]"#]],
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
    check_completions(
        // language=Move
        r#"
module 0x1::m {
    struct VestingContract { val: u8 }
    fun main() {
        Ves/*caret*/
    }
}
    "#,
        expect![[r#"
            [
                "VestingContract",
            ]"#]],
    );
}

#[test]
fn test_external_module_item_completion() {
    check_completions(
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
        expect![[r#"
            [
                "call1()",
                "call2()",
            ]"#]],
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
                    T[@0x1].s.field/*caret*/;
                }
            }
        "#]],
    );
}

#[test]
fn test_field_completion_detail() {
    check_completions(
        // language=Move
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
            [
                "field -> u8",
            ]"#]],
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
    check_completions(
        // language=Move
        r#"
module 0x1::m {
    struct S<T> { val: T }
    fun main(s: S<u8>) {
        s.va/*caret*/;
    }
}
    "#,
        expect![[r#"
            [
                "val -> u8",
            ]"#]],
    );
}

#[test]
fn test_method_completion_with_substituted_parameters() {
    check_completions(
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
        expect![[r#"
            [
                "receiver(my_val: u8) -> u8",
            ]"#]],
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
                    S[@0x1].field/*caret*/;
                }
            }
        "#]],
    );
}

#[test]
fn test_variable_completion_in_nested_block() {
    check_completions(
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
        expect![[r#"
            [
                "var -> u8",
            ]"#]],
    );
}

#[test]
fn test_variable_completion_in_if_block_after_incomplete_call_expr() {
    check_completions(
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
        expect![[r#"
            [
                "var -> u8",
            ]"#]],
    );
}

#[test]
fn test_for_each_ref_lambda_parameter() {
    check_completions(
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
        expect![[r#"
            [
                "module_address -> address",
            ]"#]],
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

#[test]
fn test_no_item_completion_if_no_l_brace() {
    check_no_completions(
        // language=Move
        r#"
module std::option pa/*caret*/
    "#,
    );
}

#[test]
fn test_no_item_completion_before_l_brace_in_module() {
    check_no_completions(
        // language=Move
        r#"
module std::option pa/*caret*/ {
}
    "#,
    );
}

#[test]
fn test_completion_in_expr_position_in_struct_literal() {
    do_single_completion(
        // language=Move
        r#"
module std::string {}
module std::option {
    use std::string;
    struct Option { vec: u8 }
    fun main() {
        let my_vec = 1;
        Option { vec: stri/*caret*/ }
    }
}
    "#,
        // language=Move
        expect![[r#"
            module std::string {}
            module std::option {
                use std::string;
                struct Option { vec: u8 }
                fun main() {
                    let my_vec = 1;
                    Option { vec: string/*caret*/ }
                }
            }
        "#]],
    );
}

#[test]
fn test_complete_built_signer_in_type_position() {
    do_single_completion(
        // language=Move
        r#"
module std::string {}
module std::option {
    use std::string;
    struct Option { vec: sig/*caret*/ }
}
    "#,
        // language=Move
        expect![[r#"
            module std::string {}
            module std::option {
                use std::string;
                struct Option { vec: signer/*caret*/ }
            }
        "#]],
    );
}

#[test]
fn test_no_builtin_type_if_path_qualifier_is_present() {
    check_no_completions(
        // language=Move
        r#"
module std::string {}
module std::option {
    use std::string;
    struct Option { vec: string::sig/*caret*/ }
}
    "#,
    );
}

#[test]
fn test_complete_vector_literal() {
    do_single_completion(
        // language=Move
        r#"
module std::option {
    fun main() {
        vec/*caret*/
    }
}
    "#,
        // language=Move
        expect![[r#"
            module std::option {
                fun main() {
                    vector[/*caret*/]
                }
            }
        "#]],
    );
}

#[test]
fn test_complete_assert_macro() {
    do_single_completion(
        // language=Move
        r#"
module std::option {
    fun main() {
        ass/*caret*/
    }
}
    "#,
        // language=Move
        expect![[r#"
            module std::option {
                fun main() {
                    assert!(/*caret*/)
                }
            }
        "#]],
    );
}

#[test]
fn test_complete_function_in_item_spec() {
    do_single_completion(
        // language=Move
        r#"
module std::option {
    fun main() {
    }
    spec ma/*caret*/
}
    "#,
        // language=Move
        expect![[r#"
            module std::option {
                fun main() {
                }
                spec main /*caret*/
            }
        "#]],
    );
}

#[test]
fn test_complete_module_keyword_at_item_spec() {
    do_single_completion(
        // language=Move
        r#"
module std::option {
    fun main() {
    }
    spec mod/*caret*/
}
    "#,
        // language=Move
        expect![[r#"
            module std::option {
                fun main() {
                }
                spec module /*caret*/
            }
        "#]],
    );
}

#[test]
fn test_complete_other_item_spec_keywords() {
    check_completions(
        // language=Move
        r#"
module std::option {
    spec /*caret*/
}
    "#,
        expect![[r#"
            [
                "module",
                "schema",
                "fun",
            ]"#]],
    );
}

#[test]
fn test_no_spec_block_keywords_in_block() {
    check_no_completions(
        // language=Move
        r#"
module std::option {
    fun main() {
        assum/*caret*/
    }
}
    "#,
    );
}

#[test]
fn test_no_spec_block_keywords_in_spec_block_but_not_directly_at_stmt() {
    check_no_completions(
        // language=Move
        r#"
module std::option {
    fun main() {
        &assum/*caret*/
    }
}
    "#,
    );
}

#[test]
fn test_spec_predicate_keywords_in_spec_module_block() {
    check_completions(
        // language=Move
        r#"
module std::option {
    spec module {
        /*caret*/
    }
}
    "#,
        expect![[r#"
            [
                "if",
                "match",
                "loop",
                "while",
                "for",
                "let",
                "true",
                "false",
                "pragma",
                "axiom",
                "invariant",
                "Self",
                "max_u8() -> num",
                "max_u64() -> num",
                "max_u128() -> num",
                "global(addr: address) -> T",
                "old(t: T) -> T",
                "update_field(s: S, fname: F, val: V) -> S",
                "TRACE(t: T) -> T",
                "concat(v1: vector<T>, v2: vector<T>) -> vector<T>",
                "vec(t: T) -> vector<T>",
                "len(t: vector<T>) -> num",
                "contains(v: vector<T>, e: T) -> bool",
                "index_of(v: vector<T>, e: T) -> num",
                "range(v: vector<T>) -> range<num>",
                "update(v: vector<T>, i: num, t: T) -> vector<T>",
                "in_range(v: vector<T>, i: num) -> bool",
                "int2bv(i: num) -> bv",
                "bv2int(b: bv) -> num",
                "exists(addr: address) -> bool",
                "__COMPILE_FOR_TESTING__",
                "MAX_U8",
                "MAX_U16",
                "MAX_U32",
                "MAX_U64",
                "MAX_U128",
                "MAX_U256",
                "MAX_I8",
                "MAX_I16",
                "MAX_I32",
                "MAX_I64",
                "MAX_I128",
                "MAX_I256",
                "vector[]",
            ]"#]],
    );
}

#[test]
fn test_spec_predicate_keywords_in_item_spec_block() {
    check_completions(
        // language=Move
        r#"
module std::option {
    fun main() {}
    spec main {
        /*caret*/
    }
}
    "#,
        expect![[r#"
            [
                "if",
                "match",
                "loop",
                "while",
                "for",
                "let",
                "true",
                "false",
                "pragma",
                "requires",
                "decreases",
                "ensures",
                "modifies",
                "include",
                "apply",
                "aborts_if",
                "aborts_with",
                "emits",
                "invariant",
                "Self",
                "max_u8() -> num",
                "max_u64() -> num",
                "max_u128() -> num",
                "global(addr: address) -> T",
                "old(t: T) -> T",
                "update_field(s: S, fname: F, val: V) -> S",
                "TRACE(t: T) -> T",
                "concat(v1: vector<T>, v2: vector<T>) -> vector<T>",
                "vec(t: T) -> vector<T>",
                "len(t: vector<T>) -> num",
                "contains(v: vector<T>, e: T) -> bool",
                "index_of(v: vector<T>, e: T) -> num",
                "range(v: vector<T>) -> range<num>",
                "update(v: vector<T>, i: num, t: T) -> vector<T>",
                "in_range(v: vector<T>, i: num) -> bool",
                "int2bv(i: num) -> bv",
                "bv2int(b: bv) -> num",
                "main()",
                "exists(addr: address) -> bool",
                "__COMPILE_FOR_TESTING__",
                "MAX_U8",
                "MAX_U16",
                "MAX_U32",
                "MAX_U64",
                "MAX_U128",
                "MAX_U256",
                "MAX_I8",
                "MAX_I16",
                "MAX_I32",
                "MAX_I64",
                "MAX_I128",
                "MAX_I256",
                "vector[]",
            ]"#]],
    );
}

#[test]
fn test_no_spec_predicates_in_spec_fun() {
    check_no_completions(
        // language=Move
        r#"
module std::option {
    spec fun main(): u8 {
        a/*caret*/
    }
}
    "#,
    );
}

#[test]
fn test_spec_predicate_keywords_in_spec_block_assume() {
    do_single_completion(
        // language=Move
        r#"
module std::option {
    fun main() {
        spec {
            assu/*caret*/
        }
    }
}
    "#,
        // language=Move
        expect![[r#"
            module std::option {
                fun main() {
                    spec {
                        assume /*caret*/
                    }
                }
            }
        "#]],
    );
}

#[test]
fn test_spec_predicate_keywords_in_inner_spec_block() {
    check_completions(
        // language=Move
        r#"
module std::option {
    spec module {
        {
            /*caret*/
        }
    }
}
    "#,
        expect![[r#"
            [
                "if",
                "match",
                "loop",
                "while",
                "for",
                "let",
                "true",
                "false",
                "pragma",
                "axiom",
                "invariant",
                "Self",
                "max_u8() -> num",
                "max_u64() -> num",
                "max_u128() -> num",
                "global(addr: address) -> T",
                "old(t: T) -> T",
                "update_field(s: S, fname: F, val: V) -> S",
                "TRACE(t: T) -> T",
                "concat(v1: vector<T>, v2: vector<T>) -> vector<T>",
                "vec(t: T) -> vector<T>",
                "len(t: vector<T>) -> num",
                "contains(v: vector<T>, e: T) -> bool",
                "index_of(v: vector<T>, e: T) -> num",
                "range(v: vector<T>) -> range<num>",
                "update(v: vector<T>, i: num, t: T) -> vector<T>",
                "in_range(v: vector<T>, i: num) -> bool",
                "int2bv(i: num) -> bv",
                "bv2int(b: bv) -> num",
                "exists(addr: address) -> bool",
                "__COMPILE_FOR_TESTING__",
                "MAX_U8",
                "MAX_U16",
                "MAX_U32",
                "MAX_U64",
                "MAX_U128",
                "MAX_U256",
                "MAX_I8",
                "MAX_I16",
                "MAX_I32",
                "MAX_I64",
                "MAX_I128",
                "MAX_I256",
                "vector[]",
            ]"#]],
    );
}

#[test]
fn test_no_expr_keywords_in_path_type() {
    check_no_completions(
        // language=Move
        r#"
module std::option {
    fun main() {
        let a: wh/*caret*/
    }
}
    "#,
    );
}

#[test]
fn test_path_completion_without_ident() {
    check_completions(
        // language=Move
        r#"
module std::option {
    fun call() {}
    fun main() {
        /*caret*/
    }
}
    "#,
        expect![[r#"
            [
                "if",
                "match",
                "loop",
                "while",
                "for",
                "let",
                "true",
                "false",
                "Self",
                "call()",
                "main()",
                "move_from(addr: address) -> T",
                "move_to(acc: &signer, res: T)",
                "borrow_global(addr: address) -> &T",
                "borrow_global_mut(addr: address) -> &mut T",
                "exists(addr: address) -> bool",
                "freeze(mut_ref: &mut S) -> &S",
                "__COMPILE_FOR_TESTING__",
                "MAX_U8",
                "MAX_U16",
                "MAX_U32",
                "MAX_U64",
                "MAX_U128",
                "MAX_U256",
                "MAX_I8",
                "MAX_I16",
                "MAX_I32",
                "MAX_I64",
                "MAX_I128",
                "MAX_I256",
                "vector[]",
                "assert!(_: bool, err: u64)",
            ]"#]],
    );
}

// language=Move
#[test]
fn test_named_field_completion_in_struct_lit() {
    do_single_completion(
        r#"
module std::option {
    struct S { named_val: u8 }
    fun main() {
        S { na/*caret*/ }
    }
}
    "#,
        expect![[r#"
            module std::option {
                struct S { named_val: u8 }
                fun main() {
                    S { named_val/*caret*/ }
                }
            }
        "#]],
    );
}

// language=Move
#[test]
fn test_complete_vector_type_with_angle_brackets() {
    do_single_completion(
        r#"
module std::option {
    fun main() {
        let a: vec/*caret*/
    }
}
    "#,
        expect![[r#"
            module std::option {
                fun main() {
                    let a: vector</*caret*/>
                }
            }
        "#]],
    );
}

// language=Move
#[test]
fn test_struct_type_no_type_params() {
    do_single_completion(
        r#"
module std::option {
    struct MyStruct { val: u8 }
    fun main() {
        let a: MyS/*caret*/
    }
}
    "#,
        expect![[r#"
            module std::option {
                struct MyStruct { val: u8 }
                fun main() {
                    let a: MyStruct/*caret*/
                }
            }
        "#]],
    );
}

// language=Move
#[test]
fn test_struct_type_with_type_params() {
    do_single_completion(
        r#"
module std::option {
    struct MyStruct<T> { val: T }
    fun main() {
        let a: MyS/*caret*/
    }
}
    "#,
        expect![[r#"
            module std::option {
                struct MyStruct<T> { val: T }
                fun main() {
                    let a: MyStruct</*caret*/>
                }
            }
        "#]],
    );
}

#[test]
fn test_no_completion_for_lambda_param() {
    check_no_completions(
        // language=Move
        r#"
module std::vector {
    public inline fun for_each<T>(self: vector<T>, f: |T|) {}
}
module std::main {
    fun main() {
        vector[1].for_each(|el/*caret*/|)
    }
}
    "#,
    );
}

#[test]
fn test_named_field_completions_detail() {
    check_completions(
        // language=Move
        r#"
module std::main {
    struct S { val_1: u8, val_2: u16 }
    fun main() {
        S { va/*caret*/ }
    }
}
    "#,
        expect![[r#"
            [
                "val_1 -> u8",
                "val_2 -> u16",
            ]"#]],
    );
}

#[test]
fn test_do_not_show_already_present_fields_in_completion() {
    check_completions(
        // language=Move
        r#"
module std::main {
    struct S { val_1: u8, val_2: u16 }
    fun main() {
        S { val_1: 1, va/*caret*/ }
    }
}
    "#,
        expect![[r#"
            [
                "val_2 -> u16",
            ]"#]],
    );
}

#[test]
fn test_show_current_field_in_completion() {
    check_completions(
        // language=Move
        r#"
module std::main {
    struct S { val_1: u8 }
    fun main() {
        S { val_1/*caret*/ }
    }
}
    "#,
        expect![[r#"
            [
                "val_1 -> u8",
            ]"#]],
    );
}

#[test]
fn test_module_completion_in_types() {
    check_completions(
        // language=Move
        r#"
module std::table {
    struct Table {}
}
module std::main {
    struct S { val_1: std::tab/*caret*/ }
}
    "#,
        expect![[r#"
            [
                "table",
            ]"#]],
    );
}

#[test]
fn test_enum_variants_should_not_be_present_in_types() {
    check_no_completions(
        // language=Move
        r#"
module std::table {
    enum Table { One, Two }
}
module std::main {
    struct S { val_1: std::table::Table::O/*caret*/ }
}
    "#,
    );
}

#[test]
fn test_items_already_present_in_the_use_group_not_in_completion() {
    check_completions(
        // language=Move
        r#"
module std::table {
    public fun fun_1(): u8 {}
    public fun fun_2(): u8 {}
}
module std::main {
    use std::table::{fun_1, fu/*caret*/};
}
    "#,
        expect![[r#"
            [
                "fun_2() -> u8",
            ]"#]],
    );
}

#[test]
fn test_struct_pat_field_completion() {
    check_completions(
        // language=Move
        r#"
module std::main {
    struct S { field_1: u8, field_2: u8 }
    fun main() {
        let S { fiel/*caret*/ };
    }
}
    "#,
        expect![[r#"
            [
                "field_1 -> u8",
                "field_2 -> u8",
            ]"#]],
    );
}

#[test]
fn test_struct_pat_field_completion_filter_existing_fields() {
    check_completions(
        // language=Move
        r#"
module std::main {
    struct S { field_1: u8, field_2: u8 }
    fun main() {
        let S { field_1, fiel/*caret*/ };
    }
}
    "#,
        expect![[r#"
            [
                "field_2 -> u8",
            ]"#]],
    );
}

#[test]
fn test_no_fields_completion_in_expr() {
    check_no_completions(
        // language=Move
        r#"
module std::main {
    struct S { field_1: u8, field_2: u8 }
    fun main() {
        let S { field_1: fi/*caret*/ };
    }
}
    "#,
    );
}

#[test]
fn test_completion_for_acquires_type() {
    check_completions(
        // language=Move
        r#"
module std::main {
    struct String {}
    fun main() acquires /*caret*/ {
    }
}
    "#,
        expect![[r#"
            [
                "String",
            ]"#]],
    );
}

#[test]
fn test_completion_for_acquires_type_from_other_module() {
    check_completions(
        // language=Move
        r#"
module std::string {
    struct String {}
}
module std::main {
    use std::string;
    fun main() acquires string::S/*caret*/ {
    }
}
    "#,
        expect![[r#"
            [
                "String",
            ]"#]],
    );
}

#[test]
fn test_complete_assert_from_as_keyword() {
    check_completions(
        // language=Move
        r#"
module std::main {
    fun main() {
        as/*caret*/
    }
}
    "#,
        expect![[r#"
            [
                "assert!(_: bool, err: u64)",
            ]"#]],
    );
}

#[test]
fn test_no_completions_for_test_functions() {
    check_no_completions(
        // language=Move
        r#"
module std::main {
    #[test]
    fun test_main() {
    }
    #[test]
    fun test_main_2() {
        test_/*caret*/
    }
}"#,
    )
}

#[test]
fn test_completions_for_test_only_functions() {
    check_completions(
        // language=Move
        r#"
module std::main {
    #[test_only]
    fun test_main() {
    }
    #[test]
    fun test_main_2() {
        test_/*caret*/
    }
}"#,
        expect![[r#"
            [
                "test_main()",
            ]"#]],
    )
}

#[test]
fn test_no_self_in_module_completion_outside_use_stmt() {
    check_no_completions(
        // language=Move
        r#"
module std::string {
}
module std::main {
    use std::string;
    fun main() {
        string::/*caret*/
    }
}"#,
    )
}

#[test]
fn test_completion_for_fq_name_if_next_is_fq_name() {
    check_completions(
        // language=Move
        r#"
module std::string {
    public fun call() {}
}
module std::main {
    use std::string;
    fun main() {
        string::/*caret*/
        string::call();
    }
}"#,
        expect![[r#"
            [
                "call()",
            ]"#]],
    )
}

#[test]
fn test_no_completion_of_assert_if_bool() {
    check_no_completions(
        // language=Move
        r#"
module std::main {
    fun main() {
        bool/*caret*/
    }
}"#,
    )
}

#[test]
fn test_spec_keywords_in_spec_blocks() {
    check_completions(
        // language=Move
        r#"
module std::main {
    fun main() {
        spec {
            a/*caret*/
        }
    }
}"#,
        expect![[r#"
            [
                "assume",
                "assert",
            ]"#]],
    )
}

#[test]
fn test_add_schemas_for_include_completion() {
    check_completions(
        // language=Move
        r#"
module std::main {
    fun main() {
    }
    spec schema MySchema {}
    spec main {
        include My/*caret*/
    }
}"#,
        expect![[r#"
            [
                "MySchema",
            ]"#]],
    )
}

#[test]
fn test_add_schemas_for_include_completion_with_generic_params() {
    do_single_completion(
        // language=Move
        r#"
module std::main {
    fun main() {
    }
    spec schema MySchema<T> {}
    spec main {
        include My/*caret*/
    }
}"#,
        expect![[r#"
            module std::main {
                fun main() {
                }
                spec schema MySchema<T> {}
                spec main {
                    include MySchema</*caret*/>
                }
            }
        "#]],
    )
}

#[test]
fn test_add_schemas_for_include_and_completion() {
    check_completions(
        // language=Move
        r#"
module std::main {
    fun main() {
    }
    spec schema MySchema {}
    spec main {
        include MySchema && My/*caret*/
    }
}"#,
        expect![[r#"
            [
                "MySchema",
            ]"#]],
    )
}

#[test]
fn test_no_angle_brackets_for_struct_expr() {
    do_single_completion(
        // language=Move
        r#"
module std::main {
    struct Any<T> { val: T }
    fun main() {
        let a = An/*caret*/
    }
}"#,
        expect![[r#"
            module std::main {
                struct Any<T> { val: T }
                fun main() {
                    let a = Any/*caret*/
                }
            }
        "#]],
    )
}

#[test]
fn test_phantom_keyword_completion() {
    do_single_completion(
        // language=Move
        r#"
module std::main {
    struct Any</*caret*/> { val: T }
}"#,
        expect![[r#"
        module std::main {
            struct Any<phantom /*caret*/> { val: T }
        }
    "#]],
    )
}

#[test]
fn test_phantom_keyword_completion_with_type() {
    do_single_completion(
        // language=Move
        r#"
module std::main {
    struct Any<ph/*caret*/T> { val: T }
}"#,
        expect![[r#"
            module std::main {
                struct Any<phantom /*caret*/T> { val: T }
            }
        "#]],
    )
}

#[test]
fn test_no_phantom_kw_in_functions() {
    check_no_completions(
        // language=Move
        r#"
module std::main {
    fun main<ph/*caret*/>() {}
}"#,
    )
}

#[test]
fn test_no_phantom_kw_if_already_there() {
    check_no_completions(
        // language=Move
        r#"
module std::main {
    struct Any<phantom ph/*caret*/> { val: T }
}"#,
    )
}

#[test]
fn test_no_builtin_storage_functions_in_specs_for_module() {
    check_no_completions(
        // language=Move
        r#"
module std::main {
    fun main() {
    }
    spec main {
        ensures borr/*caret*/
    }
}"#,
    )
}

#[test]
fn test_no_builtin_storage_functions_in_specs_for_module_spec() {
    check_no_completions(
        // language=Move
        r#"
module std::main {
    fun main() {
    }
}
spec std::main {
    spec main {
        ensures borr/*caret*/
    }
}
"#,
    )
}

#[test]
fn test_exists_function_is_available_in_spec_completion() {
    do_single_completion(
        // language=Move
        r#"
module std::main {
    fun main() {
    }
}
spec std::main {
    spec main {
        ensures exist/*caret*/
    }
}
"#,
        expect![[r#"
        module std::main {
            fun main() {
            }
        }
        spec std::main {
            spec main {
                ensures exists(/*caret*/)
            }
        }
    "#]],
    )
}

#[test]
fn test_has_keyword_after_struct() {
    check_completions(
        // language=Move
        r#"
module 0x1::m {
    struct S ha/*caret*/
}
"#,
        expect![[r#"
            [
                "has",
            ]"#]],
    )
}

#[test]
fn test_abilities_after_has() {
    check_completions(
        // language=Move
        r#"
module 0x1::m {
    struct S has /*caret*/
}
"#,
        expect![[r#"
            [
                "key",
                "store",
                "copy",
                "drop",
            ]"#]],
    )
}

#[test]
fn test_no_ability_in_completion_if_already_present() {
    check_completions(
        // language=Move
        r#"
module 0x1::m {
    struct S has store, drop, /*caret*/
}
"#,
        expect![[r#"
            [
                "key",
                "copy",
            ]"#]],
    )
}

#[test]
fn test_abilities_after_type_bounds() {
    check_completions(
        // language=Move
        r#"
module 0x1::m {
    fun main<T: /*caret*/>() {}
}
"#,
        expect![[r#"
            [
                "key",
                "store",
                "copy",
                "drop",
            ]"#]],
    )
}

#[test]
fn test_no_ability_in_completion_if_already_present_type_bounds() {
    check_completions(
        // language=Move
        r#"
module 0x1::m {
    fun main<T: store + drop + /*caret*/>() {}
}
"#,
        expect![[r#"
            [
                "key",
                "copy",
            ]"#]],
    )
}

#[test]
fn test_no_extra_space_after_public_if_fun_is_present() {
    do_single_completion(
        // language=Move
        r#"
module 0x1::m {
    pub/*caret*/ fun main() {

    }
}
"#,
        expect![[r#"
            module 0x1::m {
                public/*caret*/ fun main() {

                }
            }
        "#]],
    )
}

#[test]
fn test_no_extra_space_after_public_if_fun_is_present_two_spaces() {
    do_single_completion(
        // language=Move
        r#"
module 0x1::m {
    pub/*caret*/  fun main() {

    }
}
"#,
        expect![[r#"
            module 0x1::m {
                public/*caret*/  fun main() {

                }
            }
        "#]],
    )
}

#[test]
fn test_package_friend_inside_public_modifier() {
    check_completions(
        // language=Move
        r#"
module 0x1::m {
    public(/*caret*/) fun main() {

    }
}
"#,
        expect![[r#"
            [
                "friend",
                "package",
            ]"#]],
    )
}

#[test]
fn test_package_inside_public_modifier() {
    do_single_completion(
        // language=Move
        r#"
module 0x1::m {
    public(pack/*caret*/) fun main() {

    }
}
"#,
        expect![[r#"
            module 0x1::m {
                public(package/*caret*/) fun main() {

                }
            }
        "#]],
    )
}

#[test]
fn test_no_aborts_if_under_struct_item_spec() {
    check_no_completions(
        // language=Move
        r#"
module 0x1::m {
    struct S {}
    spec S {
        abor/*caret*/
    }
}
"#,
    )
}
