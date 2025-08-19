use crate::ide_test_utils::diagnostics::check_diagnostics;
use expect_test::expect;

#[test]
fn test_no_error_for_required_ability_present_on_concrete_type() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            struct R1 has store { val: u8 }
            struct S1 has key { val: R1 }

            struct R2 has copy { val: u8 }
            struct S2 has copy { val: R2 }

            struct R3 has drop { val: u8 }
            struct S3 has drop { val: R3 }
        }
    "#]]);
}

#[test]
fn test_no_error_for_required_ability_with_generic_type() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            struct S1<T> has key, drop, copy { val: T }
            struct S2<T> has key, drop, copy { val: vector<T> }
        }
    "#]]);
}

#[test]
fn test_error_no_required_ability_on_concrete_type() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            struct R { val: u8 }
            struct S1 has drop { val: R }
                               //^^^^^^ err: Missing required ability `drop`
            struct S2 has copy { val: R }
                               //^^^^^^ err: Missing required ability `copy`
            struct S3 has key { val: R }
                              //^^^^^^ err: Missing required ability `store`
        }
    "#]]);
}

#[test]
fn test_ability_error_with_explicit_type_args() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            struct R { val: u8 }
            struct S<T: copy> { t: T }
            fun main(r: R) {
                let _s = S<R> { t: r };
                         //^ err: Type `0x1::m::R` does not have required ability `copy`
            }
        }
    "#]]);
}

#[test]
fn test_ability_error_with_explicit_type_args_in_index_expr() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            struct R { val: u8 }
            struct S<T: copy> has key { t: T }
            fun main() {
                let _s = S<R>[@0x1];
                         //^ err: Type `0x1::m::R` does not have required ability `copy`
            }
        }
    "#]]);
}

#[test]
fn test_ability_error_with_inferred_type_args() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            struct R { val: u8 }
            struct S<ST: copy> { t: ST }
            fun main(r: R) {
                let _s = S { t: r };
                              //^ err: Type `0x1::m::R` does not have required ability `copy`
            }
        }
    "#]]);
}

#[test]
fn test_error_missing_multiple_abilities() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            struct R { val: u8 }
            struct S<T: copy + drop> { t: T }
            fun main(r: R) {
                let _s = S<R> { t: r };
                         //^ err: Type `0x1::m::R` does not have required abilities `[Copy, Drop]`
            }
        }
    "#]]);
}

#[test]
fn test_no_ability_errors_in_spec() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            struct R { val: u8 }
            struct S<T: copy + drop> { t: T }
            fun main(r: R) {
            }
            spec main {
                let _s = S<R> { t: r };
                let _s = S { t: r };
            }
        }
    "#]]);
}
