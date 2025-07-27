// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::ide_test_utils::completion_utils::check_completions;
use expect_test::expect;

#[test]
fn test_exact_name_match_does_before_everything_else() {
    check_completions(
        // language=Move
        r#"
module std::main {
    fun call(exact_match: u8) {}
    fun main() {
        let exact_match: u8 = 1;
        let exact_match_with_suffix: u8 = 2;
        call(exa/*caret*/);
    }
}
    "#,
        expect![[r#"
            [
                "exact_match -> u8",
                "exact_match_with_suffix -> u8",
            ]"#]],
    );
}

#[test]
fn test_sort_completions_by_function_return_type_in_call_expr_argument() {
    check_completions(
        // language=Move
        r#"
module std::main {
    fun call_longer_invalid_type(): u8 {}
    fun call_valid_type(): u16 {}
    fun receiver(a: u16) {}
    fun main() {
        receiver(ca/*caret*/)
    }
}
    "#,
        expect![[r#"
            [
                "call_valid_type() -> u16",
                "call_longer_invalid_type() -> u8",
            ]"#]],
    );
}

#[test]
fn test_sort_completions_by_function_return_type_in_struct_lit_field() {
    check_completions(
        // language=Move
        r#"
module std::main {
    fun call_longer_invalid_type(): u8 {}
    fun call_valid_type(): u16 {}
    struct S { named: u16 }
    fun main() {
        S { named: ca/*caret*/ };
    }
}
    "#,
        expect![[r#"
            [
                "call_valid_type() -> u16",
                "call_longer_invalid_type() -> u8",
            ]"#]],
    );
}

#[test]
fn test_sort_completions_by_method_return_type() {
    check_completions(
        // language=Move
        r#"
module std::main {
    struct S { val: u8 }
    fun val_u8_not_valid(self: &S): u8 { 1 }
    fun val_u16_valid(self: &S): u16 { 1 }
    fun receiver(a: u16) {}
    fun main(s: S) {
        receiver(val/*caret*/)
    }
}
    "#,
        expect![[r#"
            [
                "val_u16_valid(self: &S) -> u16",
                "val_u8_not_valid(self: &S) -> u8",
            ]"#]],
    );
}

#[test]
fn test_sort_completions_by_method_return_type_with_generic() {
    check_completions(
        // language=Move
        r#"
module std::main {
    struct S<T> { val: T }
    fun val_u8_invalid<T>(self: &S<T>): u8 { 1 }
    fun val_t<T>(self: &S<T>): T { 1 }
    fun receiver(a: u16) {}
    fun main(s: S<u16>) {
        receiver(s.val_/*caret*/)
    }
}
    "#,
        expect![[r#"
            [
                "val_t() -> u16",
                "val_u8_invalid() -> u8",
            ]"#]],
    );
}

#[test]
fn test_sort_completions_by_ident_pat_type() {
    check_completions(
        // language=Move
        r#"
module std::main {
    fun receiver(a: u16) {}
    fun main() {
        let call_valid_type: u16 = 1;
        let call_longer_invalid_type: u8 = 1;
        receiver(ca/*caret*/)
    }
}
    "#,
        expect![[r#"
            [
                "call_valid_type -> u16",
                "call_longer_invalid_type -> u8",
            ]"#]],
    );
}

#[test]
fn test_prioritize_local_idents_over_global_items() {
    check_completions(
        // language=Move
        r#"
module std::string {
    public fun ident_longer(): u8 { 1 }
}
module std::main {
    fun ident_longer(): u8 { 1 }
    fun receiver(a: u8) {}
    fun main(ident: u8) {
        use std::string::ident_longer;
        receiver(ide/*caret*/)
    }
}
    "#,
        expect![[r#"
            [
                "ident -> u8",
                "ident_longer() -> u8",
            ]"#]],
    );
}

#[test]
fn test_sort_types_accounting_for_integer_variables() {
    check_completions(
        // language=Move
        r#"
module std::main {
    fun receiver(a: u16) {}
    fun main() {
        let call_integer_type = 1;
        let call_bool_type: bool = true;
        receiver(ca/*caret*/)
    }
}
    "#,
        expect![[r#"
            [
                "call_integer_type -> integer",
                "call_bool_type -> bool",
            ]"#]],
    );
}

#[test]
fn test_spec_predicate_expected_type() {
    check_completions(
        // language=Move
        r#"
module std::main {
    fun get_u8(): u8 { 1 }
    fun get_bool(): bool { true }
    fun main() {
        spec {
            assume get/*caret*/
        }
    }
}
    "#,
        expect![[r#"
            [
                "get_bool() -> bool",
                "get_u8() -> num",
            ]"#]],
    );
}

#[test]
fn test_aborts_if_expected_type() {
    check_completions(
        // language=Move
        r#"
module std::main {
    fun get_u8(): u8 { 1 }
    fun get_bool(): bool { true }
    fun main() {
    }
    spec main {
        aborts_if get/*caret*/
    }
}
    "#,
        expect![[r#"
            [
                "get_bool() -> bool",
                "get_u8() -> num",
            ]"#]],
    );
}
