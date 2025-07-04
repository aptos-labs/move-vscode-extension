// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

// #[test]
// fn test_struct_with_key_from_current_module() {
//     // language=Move
//     check_diagnostics(expect![[r#"
//         module 0x1::main {
//             struct S has key { val: u8 }
//             fun main(acc: &signer) acquires S {
//                 let s = move_from<S>(@0x1);
//                 move_to(acc, s);
//
//                 borrow_global<S>(@0x1);
//                 borrow_global_mut<S>(@0x1);
//
//                 S[@0x1];
//                 &S[@0x1];
//                 &mut S[@0x1];
//             }
//         }
//     "#]]);
// }

// #[test]
// fn test_struct_no_key_from_current_module_explicit_type_args() {
//     // language=Move
//     check_diagnostics(expect![[r#"
//         module 0x1::main {
//             struct S { val: u8 }
//             struct T<Res: store> has key { r: Res }
//             fun main() {
//                 T<S> { r: S { val: 1 } };
//             }
//         }
//     "#]]);
// }
