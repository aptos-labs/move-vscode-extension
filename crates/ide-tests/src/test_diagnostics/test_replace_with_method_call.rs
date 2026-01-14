// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ide_test_utils::diagnostics::{check_diagnostics, check_diagnostics_and_fix};
use expect_test::expect;

#[test]
fn test_no_warning_if_parameter_is_not_self() {
    // language=Move
    check_diagnostics(expect![[r#"
module 0x1::main {
    struct S { field: u8 }
    fun get_field(s: &S): u8 { s.field }
    fun main(s: S) {
        get_field(&s);
    }
}
"#]]);
}

#[test]
fn test_no_warning_if_first_parameter_has_different_type() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            struct S { field: u8 }
            struct T { field: u8 }
            fun get_field(self: &T): u8 { self.field }
            fun main(s: S) {
                get_field(&s);
                        //^^ err: Incompatible type '&0x1::main::S', expected '&0x1::main::T'
            }
        }
    "#]]);
}

#[test]
fn test_no_warning_if_references_are_incompatible() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            struct S { field: u8 }
            fun get_field(s: &mut S): u8 { s.field }
            fun main(s: &S) {
                get_field(s);
                        //^ err: Incompatible type '&0x1::main::S', expected '&mut 0x1::main::S'
            }
        }
    "#]]);
}

#[test]
fn test_no_warning_if_self_parameter_struct_is_from_another_module() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            struct S { field: u8 }
        }
        module 0x1::main {
            use 0x1::m::S;
            fun get_field(self: S): u8 { self.field }
                                            //^^^^^ err: Unresolved reference `field`: cannot resolve
            fun main(s: S) {
                get_field(s);
            }
        }
    "#]]);
}

#[test]
fn test_no_warning_if_self_parameter_is_not_provided() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            struct S { field: u8 }
            fun get_field(s: S): u8 { s.field }
            fun main(_s: S) {
                get_field();
                        //^ err: This function takes 1 parameters, but 0 parameters were supplied
            }
        }
    "#]]);
}

#[test]
fn test_no_warning_if_not_enough_parameters() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            struct S { field: u8 }
            fun get_field(s: S, _a: u8, _b: u8): u8 { s.field }
            fun main(s: S) {
                get_field(s, 1);
                            //^ err: This function takes 3 parameters, but 2 parameters were supplied
            }
        }
    "#]]);
}

#[test]
fn test_no_warning_if_generics_are_incompatible() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            struct S<T> { field: T }
            fun get_field(self: &S<u8>): u8 { self.field }
            fun main(s: &S<u16>) {
                get_field(s);
                        //^ err: Incompatible type '&0x1::main::S<u16>', expected '&0x1::main::S<u8>'
            }
        }
    "#]]);
}

#[test]
fn test_no_warning_if_generic_is_unknown() {
    // language=Move
    check_diagnostics(expect![[r#"
module 0x1::main {
    struct S<T> { field: T }
    fun get_field(self: &S<u8>): u8 { self.field }
    fun main(s: &S<u12345>) {
                 //^^^^^^ err: Unresolved reference `u12345`: cannot resolve
        get_field(s);
    }
}
"#]]);
}

#[test]
fn test_method_with_fix() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::m {
                struct S { val: u8 }
                fun method(self: S): u8 {
                    self.val
                }
                fun main(s: S) {
                    method(s);
                  //^^^^^^^^^ weak: Can be replaced with method call
                }
            }
        "#]],
        expect![[r#"
            module 0x1::m {
                struct S { val: u8 }
                fun method(self: S): u8 {
                    self.val
                }
                fun main(s: S) {
                    s.method();
                }
            }
        "#]],
    );
}

#[test]
fn test_method_with_parameters_with_fix() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::m {
                struct S { val: u8 }
                fun method(self: S, _a: u8, _b: u8): u8 {
                    self.val
                }
                fun main(s: S) {
                    method(s, 1, 2);
                  //^^^^^^^^^^^^^^^ weak: Can be replaced with method call
                }
            }
        "#]],
        expect![[r#"
            module 0x1::m {
                struct S { val: u8 }
                fun method(self: S, _a: u8, _b: u8): u8 {
                    self.val
                }
                fun main(s: S) {
                    s.method(1, 2);
                }
            }
        "#]],
    );
}

#[test]
fn test_method_of_imported_struct_with_fix() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::m {
                struct S { field: u8 }
                public fun get_field(self: S): u8 { self.field }
            }
            module 0x1::main {
                use 0x1::m::S;
                use 0x1::m::get_field;
                fun main(s: S) {
                    get_field(s);
                  //^^^^^^^^^^^^ weak: Can be replaced with method call
                }
            }
        "#]],
        expect![[r#"
            module 0x1::m {
                struct S { field: u8 }
                public fun get_field(self: S): u8 { self.field }
            }
            module 0x1::main {
                use 0x1::m::S;
                use 0x1::m::get_field;
                fun main(s: S) {
                    s.get_field();
                }
            }
        "#]],
    );
}

#[test]
fn test_method_with_autoborrow_with_fix() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::m {
                struct S { field: u8 }
                public fun get_field(self: S): u8 { self.field }
            }
            module 0x1::main {
                use 0x1::m::S;
                use 0x1::m::get_field;
                fun main(s: S) {
                    get_field(s);
                  //^^^^^^^^^^^^ weak: Can be replaced with method call
                }
            }
        "#]],
        expect![[r#"
            module 0x1::m {
                struct S { field: u8 }
                public fun get_field(self: S): u8 { self.field }
            }
            module 0x1::main {
                use 0x1::m::S;
                use 0x1::m::get_field;
                fun main(s: S) {
                    s.get_field();
                }
            }
        "#]],
    );
}

#[test]
fn test_method_with_compatible_reference_with_fix() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::main {
                struct S { field: u8 }
                fun get_field(self: &S): u8 { self.field }
                fun main(s: &mut S) {
                    get_field(s);
                  //^^^^^^^^^^^^ weak: Can be replaced with method call
                }
            }
        "#]],
        expect![[r#"
            module 0x1::main {
                struct S { field: u8 }
                fun get_field(self: &S): u8 { self.field }
                fun main(s: &mut S) {
                    s.get_field();
                }
            }
        "#]],
    );
}

#[test]
fn test_method_with_fix_transfer_type_arguments() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::main {
                struct S<T> { field: u8 }
                native fun get_type<U, T>(self: &S<U>): T;
                fun main<T>(s: S<T>) {
                    get_type<T, u8>(&s);
                  //^^^^^^^^^^^^^^^^^^^ weak: Can be replaced with method call
                }
            }
        "#]],
        expect![[r#"
            module 0x1::main {
                struct S<T> { field: u8 }
                native fun get_type<U, T>(self: &S<U>): T;
                fun main<T>(s: S<T>) {
                    s.get_type::<T, u8>();
                }
            }
        "#]],
    );
}

#[test]
fn test_method_with_fix_wrap_deref_expr_into_parens() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::main {
                struct String { bytes: vector<u8> }
                public native fun sub_string(self: &String, i: u64, j: u64): String;
                fun main(key: &String) {
                    sub_string(&*key, 1, 2);
                  //^^^^^^^^^^^^^^^^^^^^^^^ weak: Can be replaced with method call
                }
            }
        "#]],
        expect![[r#"
            module 0x1::main {
                struct String { bytes: vector<u8> }
                public native fun sub_string(self: &String, i: u64, j: u64): String;
                fun main(key: &String) {
                    (*key).sub_string(1, 2);
                }
            }
        "#]],
    );
}

#[test]
fn test_method_with_fix_wrap_copy_expr_into_parens() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::main {
                struct String { bytes: vector<u8> }
                public native fun sub_string(self: &String, i: u64, j: u64): String;
                fun main(key: &String) {
                    sub_string(copy key, 1, 2);
                  //^^^^^^^^^^^^^^^^^^^^^^^^^^ weak: Can be replaced with method call
                }
            }
        "#]],
        expect![[r#"
            module 0x1::main {
                struct String { bytes: vector<u8> }
                public native fun sub_string(self: &String, i: u64, j: u64): String;
                fun main(key: &String) {
                    (copy key).sub_string(1, 2);
                }
            }
        "#]],
    );
}

#[test]
fn test_no_warning_for_vector_borrow_as_it_has_vector_index_expr_present_instead() {
    // language=Move
    check_diagnostics(expect![[r#"
            module 0x1::vector {
                native public fun borrow<Element>(self: &vector<Element>, i: u64): &Element;
            }
            module 0x1::m {
                use 0x1::vector;

                fun main() {
                    let v = vector[1, 2];
                    let vv = &v;
                    *vector::borrow(vv, 0);
                  //^^^^^^^^^^^^^^^^^^^^^^ weak: Can be replaced with index expr
                }
            }
        "#]]);
}

#[test]
fn test_no_suggestion_for_item_type_of_which_is_determined_inside_lambda() {
    // language=Move
    check_diagnostics(expect![[r#"
            module 0x1::m {
                struct Container { val: u8 }
                fun borrow(self: &Container): &Container { self }
                inline fun apply<T>(t: T, f: |&T|) {
                    f(&t);
                }
                fun main(c: Container) {
                    apply(c, |item| { let _ = borrow(item); });
                }
            }
        "#]]);
}
