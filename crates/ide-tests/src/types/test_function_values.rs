// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::types::check_expr_type;

#[test]
fn test_fetch_function_value_from_struct_and_call() {
    // language=Move
    check_expr_type(
        r#"
module 0x1::m {
    struct R { val: u8 }
    struct S { fn: |address| R }
    fun main(s: &S) {
        (s.fn)(@0x1).val;
                   //^ u8
    }
}
    "#,
    );
}

// language=Move
#[test]
fn test_function_value_named_wrapper() {
    check_expr_type(
        r#"
module 0x1::main {
    struct Predicate<T>(|&T|bool) has copy;
    fun main() {
        let a: Predicate<u64> = |x| *x > 0;
        (a(&22));
      //^ bool
    }
}
"#,
    )
}

// language=Move
#[test]
fn test_function_value_named_wrapper_infer_lambda_type_let_stmt() {
    check_expr_type(
        r#"
module 0x1::main {
    struct Predicate<T>(|&T|bool) has copy;
    fun main() {
        let a: Predicate<u64> = |x| *x > 0;
                                   //^ &u64
    }
}
"#,
    )
}

// language=Move
#[test]
fn test_function_value_named_wrapper_infer_lambda_type_call_expr_type() {
    check_expr_type(
        r#"
module 0x1::main {
    struct Predicate<T>(|&T|bool) has copy;
    fun call(predicate: Predicate<u64>) {}
    fun main() {
        call(|x| *x > 0);
                //^ &u64
    }
}
"#,
    )
}

#[test]
fn test_infer_lambda_type_from_parameters() {
    // language=Move
    check_expr_type(
        r#"
module 0x1::main {
    fun main() {
        let lambda = |a: u8, b: u8| a + b;
        lambda;
         //^ |u8, u8| -> u8
    }
}
"#,
    )
}

#[test]
fn test_infer_lambda_type_from_generic_parameters() {
    // language=Move
    check_expr_type(
        r#"
module 0x1::main {
    fun main() {
        let lambda = |a, b| a + b;
        lambda;
         //^ |<unknown>, <unknown>| -> <unknown>
    }
}
"#,
    )
}
