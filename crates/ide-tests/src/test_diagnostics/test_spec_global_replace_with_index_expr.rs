// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::ide_test_utils::diagnostics::{check_diagnostics, check_diagnostics_and_fix};
use expect_test::expect;

#[test]
fn test_global_t_addr_in_spec_can_be_replaced_with_t_addr() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::main {
                struct Res has key { a: u8 }
                fun main() {
                }
                spec main {
                    global<Res>(@0x1);
                  //^^^^^^^^^^^^^^^^^ weak: Replace with resource index expr
                }
            }
        "#]],
        expect![[r#"
            module 0x1::main {
                struct Res has key { a: u8 }
                fun main() {
                }
                spec main {
                    Res[@0x1];
                }
            }
        "#]],
    );
}

#[test]
fn test_no_trigger_if_global_function_has_invalid_parameters() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            struct Res has key { a: u8 }
            fun main() {
            }
            spec main {
                global<Res>(1);
                          //^ err: Incompatible type 'num', expected 'address'
            }
        }
    "#]]);
}

#[test]
fn test_no_trigger_if_global_function_has_invalid_type_param() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            struct Res { a: u8 }
            fun main() {
            }
            spec main {
                global<u8>(@0x1);
                global<Res>(@0x1);
            }
        }
    "#]]);
}

#[test]
fn test_no_trigger_if_global_function_user_defined() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            struct Res has key { a: u8 }
            fun global<T: key>(addr: address): T { move_from(addr) }
            fun main() {
                global<Res>(@0x1);
            }
            spec main {
                global<Res>(@0x1);
              //^^^^^^^^^^^^^^^^^ weak: Replace with resource index expr
            }
        }
    "#]]);
}
