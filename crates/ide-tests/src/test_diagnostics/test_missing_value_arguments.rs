// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ide_test_utils::diagnostics::check_diagnostics;
use expect_test::expect;

#[test]
fn test_valid_number_of_arguments() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            native fun params_0();
            native fun params_1(val: u8);
            native fun params_3(val: u8, val2: u64, s: bool);

            fun main() {
                params_0();
                params_1(1);
                params_3(1, 1, true);
            }
        }
    "#]]);
}

#[test]
fn test_invalid_number_of_arguments_local() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            native fun params_0();
            native fun params_1(val: u8);
            native fun params_3(val: u8, val2: u64, s: &signer);

            fun main() {
                params_0(4);
                       //^ err: This function takes 0 parameters, but 1 parameters were supplied
                params_1();
                       //^ err: This function takes 1 parameters, but 0 parameters were supplied
                params_1(1, 4);
                          //^ err: This function takes 1 parameters, but 2 parameters were supplied
                params_3(5, 1);
                           //^ err: This function takes 3 parameters, but 2 parameters were supplied
            }
        }
    "#]]);
}

#[test]
fn test_invalid_number_of_arguments_receiver_style() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct S { field: u8 }
            native fun get_field_0(self: &S): u8;
            native fun get_field_1(self: &S, a: u8): u8;
            native fun get_field_3(self: &S, a: u8, b: u8, c: u8): u8;

            fun main(s: S) {
                s.get_field_0(4);
                            //^ err: This function takes 0 parameters, but 1 parameters were supplied
                s.get_field_1();
                            //^ err: This function takes 1 parameters, but 0 parameters were supplied
                s.get_field_1(1, 4);
                               //^ err: This function takes 1 parameters, but 2 parameters were supplied
                s.get_field_3(5, 1);
                                //^ err: This function takes 3 parameters, but 2 parameters were supplied
            }
        }
    "#]]);
}

#[test]
fn test_invalid_number_of_arguments_with_import() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::p {
            public native fun params_3(val: u8, val2: u64, s: &signer);
        }
        module 0x1::M {
            use 0x1::p::params_3;
            fun main() {
                params_3(5, 1);
                           //^ err: This function takes 3 parameters, but 2 parameters were supplied
            }
        }
    "#]]);
}

#[test]
fn test_invalid_number_of_arguments_with_import_alias() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::p {
            public native fun params_3(val: u8, val2: u64, s: &signer);
        }
        module 0x1::M {
            use 0x1::p::params_3 as params_alias;
            fun main() {
                params_alias(5, 1);
                               //^ err: This function takes 3 parameters, but 2 parameters were supplied
            }
        }
    "#]]);
}

#[test]
fn test_lambda_expr_expect_single_parameter() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            inline fun main<Element>(_e: Element, f: |Element| u8) {
                f();
                //^ err: This function takes 1 parameters, but 0 parameters were supplied
            }
        }
    "#]]);
}

#[test]
fn test_assert_macro_expects_one_or_two_parameters() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun call() {
                assert!();
                      //^ err: This function takes 1 to 6 parameters, but 0 parameters were supplied
                assert!(true);
                assert!(true, 1);
                assert!(true, 1, 1);            }
        }
    "#]]);
}

#[test]
fn test_missing_value_arguments_for_tuple_struct_contructor() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            struct S(u8, bool, vector<u8>);
            public fun main() {
                S(1);
                 //^ err: This function takes 3 parameters, but 1 parameters were supplied
                S(1, true);
                       //^ err: This function takes 3 parameters, but 2 parameters were supplied
                S(1, true, b"1234");
                S(1, true, b"1234", b"1234");
                                  //^^^^^^^ err: This function takes 3 parameters, but 4 parameters were supplied
            }
        }
    "#]]);
}

#[test]
fn test_assert_with_message_and_format_arguments() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            public fun main() {
                assert!(true, b"1234 {}");
                assert!(true, b"1234 {}", 1);
                assert!(true, b"1234 {}", 1, 2);
                assert!(true, b"1234 {}", 1, 2, 3);
                assert!(true, b"1234 {}", 1, 2, 3, 4);
                assert!(true, b"1234 {}", 1, 2, 3, 4, 5);            
                                                    //^ err: This function takes 1 to 6 parameters, but 7 parameters were supplied
            }
        }
    "#]]);
}

#[test]
fn test_assert_eq_ne_with_message() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            public fun main() {
                assert_eq!(1, 1);
                assert_ne!(1, 1);

                assert_eq!(1, 1, b"1234");
                assert_eq!(1, 1, b"1234", 1, 2, 3, 4);
                assert_eq!(1, 1, b"1234", 1, 2, 3, 4, 5);
                                                    //^ err: This function takes 2 to 7 parameters, but 8 parameters were supplied
            }
        }
    "#]]);
}

#[test]
fn test_debug_assert() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            public fun main() {
                debug_assert!(true, b"1234 {}", 1, 2, 3, 4, 5);
                                                          //^ err: This function takes 1 to 6 parameters, but 7 parameters were supplied

                debug_assert_eq!(1, 1);
                debug_assert_ne!(1, 1);

                debug_assert_eq!(1, 1, b"1234", 1, 2, 3, 4, 5);
                                                          //^ err: This function takes 2 to 7 parameters, but 8 parameters were supplied
            }
        }
    "#]]);
}

#[test]
fn test_check_apply_lemma_arguments() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            fun main() {}
            spec lemma add_mono(_a: u64) {}
            spec main {} proof {
                forall _a: u64 apply add_mono();
                                            //^ err: This function takes 1 parameters, but 0 parameters were supplied
                forall _a: u64 apply add_mono(1);
                forall _a: u64 apply add_mono(1, 1);
                                               //^ err: This function takes 1 parameters, but 2 parameters were supplied
                forall _a: u64 apply add_mono(1, 1, 1);
                                               //^ err: This function takes 1 parameters, but 3 parameters were supplied
            }
        }
    "#]]);
}

#[test]
fn test_invalid_number_of_arguments_behaviour_predicates() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            native fun params_2(val: u8, val2: u64);

            fun main() {
            }
            spec main {
                aborts_of<params_2>();
                                  //^ err: This function takes 2 parameters, but 0 parameters were supplied
                aborts_of<params_2>(1);
                                   //^ err: This function takes 2 parameters, but 1 parameters were supplied
                aborts_of<params_2>(1, 2);
                aborts_of<params_2>(1, 2, 3);
                                        //^ err: This function takes 2 parameters, but 3 parameters were supplied
                aborts_of<params_2>(1, 2, 3, 4);
                                        //^ err: This function takes 2 parameters, but 4 parameters were supplied

            }
        }
    "#]]);
}
