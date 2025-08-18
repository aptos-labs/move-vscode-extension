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
