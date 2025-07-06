// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ide_test_utils::check_signature_info;
use expect_test::expect;

#[test]
fn test_call_expr_inside_struct_lit() {
    // language=Move
    let source = r#"
    module 0x1::m {
        struct S { s: u8 }
        fun call() {}
        fun m() {
            S { s: call(/*caret*/) };
        }
    }
            "#;
    check_signature_info(source, expect!["<no arguments>"]);
}

#[test]
fn test_struct_lit_inside_call_expr() {
    // language=Move
    let source = r#"
    module 0x1::m {
        struct S {}
        fun call(s: S) {}
        fun m() {
            call(S { /*caret*/ });
        }
    }
            "#;
    check_signature_info(source, expect!["<no fields>"]);
}

#[test]
fn test_show_field_with_position_1() {
    // language=Move
    let source = r#"
    module 0x1::m {
        struct S { a: u8, b: u8 }
        fun m() {
            S { a: 1, b: /*caret*/ };
        }
    }
            "#;
    check_signature_info(
        source,
        expect![[r#"
        >>a: u8, b: u8
               //^^^^^
    "#]],
    );
}

#[test]
fn test_do_not_highlight_anything_if_all_filled_and_not_in_position() {
    // language=Move
    let source = r#"
    module 0x1::m {
        struct S { a: u8, b: u8 }
        fun m() {
            S { a: 1, b: 2,/*caret*/ };
        }
    }
            "#;
    check_signature_info(source, expect!["a: u8, b: u8"]);
}

#[test]
fn test_do_not_highlight_anything_if_all_filled_and_not_in_position_with_vectors() {
    // language=Move
    let source = r#"
    module 0x1::m {
        struct S { val1: vector<u8>, val2: vector<u8> }
        fun m() {
            S { val1: vector[1], val2: vector[2], /*caret*/ };
        }
    }
            "#;
    check_signature_info(source, expect!["val1: vector<u8>, val2: vector<u8>"]);
}

#[test]
fn test_show_field_with_position_0_passed_in_reverse_order() {
    // language=Move
    let source = r#"
    module 0x1::m {
        struct S { a: u8, b: u8 }
        fun m() {
            S { b: 2, a: 1/*caret*/ };
        }
    }
            "#;
    check_signature_info(
        source,
        expect![[r#"
        >>a: u8, b: u8
        //^^^^^
    "#]],
    );
}

#[test]
fn test_show_next_unfilled_field_in_presence_of_filled() {
    // language=Move
    let source = r#"
    module 0x1::m {
        struct S { a: u8, b: u8, c: u8, d: u8 }
        fun m() {
            S { a: 1, c: 2, /*caret*/ };
        }
    }
            "#;
    check_signature_info(
        source,
        expect![[r#"
        >>a: u8, b: u8, c: u8, d: u8
               //^^^^^
    "#]],
    );
}

#[test]
fn test_highlight_field_caret_in_the_middle() {
    // language=Move
    let source = r#"
    module 0x1::m {
        struct Collection { items: vector<u8>, items2: vector<u8> }
        fun m() {
            let myitems = b"123";
            Collection { items: myi/*caret*/tems, items2: myitems };
        }
    }
            "#;
    check_signature_info(
        source,
        expect![[r#"
        >>items: vector<u8>, items2: vector<u8>
        //^^^^^^^^^^^^^^^^^
    "#]],
    );
}

#[test]
fn test_highlight_field_caret_in_the_end() {
    // language=Move
    let source = r#"
    module 0x1::m {
        struct Collection { items: vector<u8>, items2: vector<u8> }
        fun m() {
            let myitems = b"123";
            Collection { items: myitems/*caret*/, items2: myitems };
        }
    }
            "#;
    check_signature_info(
        source,
        expect![[r#"
        >>items: vector<u8>, items2: vector<u8>
        //^^^^^^^^^^^^^^^^^
    "#]],
    );
}
