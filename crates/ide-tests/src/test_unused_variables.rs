use crate::ide_test_utils::diagnostics::check_diagnostics;
use expect_test::expect;

#[test]
fn test_unused_function_parameter() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            fun call(a: u8) {
                   //^ warn: Unused parameter 'a'
            }
        }
    "#]]);
}

#[test]
fn test_no_error_function_parameter() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            fun call(a: u8) {
                a + 1;
            }
        }
    "#]]);
}

#[test]
fn test_unused_variable() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            fun call() {
                let a = 1;
                  //^ warn: Unused variable 'a'
            }
        }
    "#]]);
}

#[test]
fn test_no_error_variable() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            fun call() {
                let a = 1;
                a + 1;
            }
        }
    "#]]);
}

#[test]
fn test_unused_variable_in_tuple() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            native fun t(): (u8, u8, u8, u8);
            fun call() {
                let (a, b, _c, _) = t();
                      //^ warn: Unused variable 'b'
                a;
            }
        }
    "#]]);
}

#[test]
fn test_unused_for_expr_ident() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            fun main() {
                for (a in 0..10) {};
                   //^ warn: Unused variable 'a'
                for (_a in 0..10) {};
                for (_ in 0..10) {};
            }
        }
    "#]]);
}

#[test]
fn test_unused_variable_in_struct_pat() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct S { s: u8, t: u8, u: u8, v: u8, z: u8 }
            fun call(s: S) {
                let S { s, t: my_t, u: _, v: _, z } = s;
                            //^^^^ warn: Unused variable 'my_t'
                                              //^ warn: Unused variable 'z'
                s;
            }
        }
    "#]]);
}

#[test]
fn test_unused_variable_in_lambda() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            fun main() {
                |a, b, _c, _| a;
                  //^ warn: Unused parameter 'b'
                |a, b, _c, _, e| { a + b };
                            //^ warn: Unused parameter 'e'
            }
        }
    "#]]);
}

#[test]
fn test_no_error_prefixed_with_underscore() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            fun call(_a: u8) {
                let _b = 1;
            }
        }
    "#]]);
}

#[test]
fn test_no_error_native_function_parameter() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            native fun main(a: u8);
        }
    "#]]);
}

#[test]
fn test_no_error_uninterpreted_spec_function() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {}
        spec 0x1::main {
            spec fun spec_rewards_amount(
                stake_amount: u64,
                num_successful_proposals: u64,
                num_total_proposals: u64,
                rewards_rate: u64,
                rewards_rate_denominator: u64,
            ): u64;
        }
    "#]]);
}
