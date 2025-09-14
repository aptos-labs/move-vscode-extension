// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ide_test_utils::completion_utils::{check_completions, do_single_completion};
use expect_test::expect;

#[test]
fn test_method_completion() {
    do_single_completion(
        // language=Move
        r#"
        module 0x1::main {
            struct S { field: u8 }
            fun receiver(self: &S): u8 {}
            fun main(s: S) {
                s.rece/*caret*/
            }
        }
    "#,
        // language=Move
        expect![[r#"
        module 0x1::main {
            struct S { field: u8 }
            fun receiver(self: &S): u8 {}
            fun main(s: S) {
                s.receiver()/*caret*/
            }
        }
    "#]],
    )
}

#[test]
fn test_method_completion_from_another_module() {
    do_single_completion(
        // language=Move
        r#"
        module 0x1::m {
            struct S { field: u8 }
            public fun receiver(self: &S): u8 {}
        }
        module 0x1::main {
            use 0x1::m::S;
            fun main(s: S) {
                s.rece/*caret*/
            }
        } 
    "#,
        // language=Move
        expect![[r#"
            module 0x1::m {
                struct S { field: u8 }
                public fun receiver(self: &S): u8 {}
            }
            module 0x1::main {
                use 0x1::m::S;
                fun main(s: S) {
                    s.receiver()/*caret*/
                }
            }
        "#]],
    )
}

#[test]
fn test_no_test_only_methods_in_main_item_scope_completion() {
    check_completions(
        // language=Move
        r#"
module std::main {
    struct S { val: u8 }
    fun method(self: S) {}
    #[test_only]
    fun method_test_only(self: S) {}
    fun main(s: S) {
        s.meth/*caret*/
    }
}
    "#,
        expect![[r#"
        [
            "method()",
            "method_test_only()",
        ]"#]],
    )
}

#[test]
fn test_no_private_methods_in_completion() {
    check_completions(
        // language=Move
        r#"
module std::m {
    struct S { val: u8 }
    public fun method(self: S) {}
    fun method_private(self: S) {}
}
module std::main {
    use std::m::S;
    fun main(s: S) {
        s.meth/*caret*/
    }
}
    "#,
        expect![[r#"
        [
            "method()",
            "method_private()",
        ]"#]],
    )
}

// todo: type annotation
// #[test]
// fn test_method_completion_with_assignment() {
//     do_single_completion(
//         // language=Move
//         r#"
//         module 0x1::main {
//             struct S { field: u8 }
//             fun receiver<Z>(self: &S): Z {}
//             fun main(s: S) {
//                 let f: u8 = s.rece/*caret*/
//             }
//         }
//     "#,
//         // language=Move
//         expect![[r#"
//         module 0x1::main {
//             struct S { field: u8 }
//             fun receiver(self: &S): u8 {}
//             fun main(s: S) {
//                 s.receiver()/*caret*/
//             }
//         }
//     "#]])
// }

// todo: type annotation
// #[test]
// fn test_method_completion_type_annotation_required() {
//     do_single_completion(
//         // language=Move
//         r#"
//         module 0x1::main {
//             struct S { field: u8 }
//             fun receiver<Z>(self: &S): Z {}
//             fun main(s: S) {
//                 s.rece/*caret*/;
//             }
//         }
//     "#,
//         // language=Move
//         expect![[r#"
//         module 0x1::main {
//             struct S { field: u8 }
//             fun receiver(self: &S): u8 {}
//             fun main(s: S) {
//                 s.receiver()/*caret*/
//             }
//         }
//     "#]])
// }

// todo: type annotation
// #[test]
// fn test_method_completion_type_annotation_required_with_angle_brackets_present() {
//     do_single_completion(
//         // language=Move
//         r#"
//         module 0x1::main {
//             struct S { field: u8 }
//             fun receiver<Z>(self: &S): Z {}
//             fun main(s: S) {
//                 s.rece/*caret*/::<>()
//             }
//         }
//     "#,
//         // language=Move
//         expect![[r#"
//         module 0x1::main {
//             struct S { field: u8 }
//             fun receiver(self: &S): u8 {}
//             fun main(s: S) {
//                 s.receiver()/*caret*/
//             }
//         }
//     "#]])
// }
