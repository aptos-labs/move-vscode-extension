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
                      //^ err: This function takes 1 to 2 parameters, but 0 parameters were supplied
                assert!(true);
                assert!(true, 1);
                assert!(true, 1, 1);            }
                               //^ err: This function takes 1 to 2 parameters, but 3 parameters were supplied
        }
    "#]]);
}
