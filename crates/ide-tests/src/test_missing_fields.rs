use crate::ide_test_utils::diagnostics::check_diagnostics;
use expect_test::expect;

#[test]
fn test_missing_fields_for_struct() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct T {
                field: u8
            }

            fun main() {
                let a = T {};
                      //^ err: Missing field for `T` initializer: `field`
                let T {} = a;
                  //^^^^ err: Struct pattern does not mention field `field`
            }
        }
    "#]]);
}

#[test]
fn test_multiple_missing_fields_for_struct() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct T {
                field: u8,
                field_2: u8,
            }

            fun main() {
                let a = T {};
                      //^ err: Missing fields for `T` initializer: `field`, `field_2`
                let T {} = a;
                  //^^^^ err: Struct pattern does not mention fields `field`, `field_2`
            }
        }
    "#]]);
}

#[test]
fn test_no_error_if_rest_pattern_is_present() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            struct S { f1: u8, f2: u8 }
            fun main(s: S) {
                let S { f1: _, .. } = s;
            }
        }
    "#]]);
}

#[test]
fn test_missing_fields_for_enum_variant() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            enum Num { One { val: u8 }, Two { val: u8, val2: u8 }}

            fun main() {
                let a = Num::Two { val: 1 };
                      //^^^^^^^^ err: Missing field for `Num::Two` initializer: `val2`
                match (a) {
                    Num::Two { val } => true
                  //^^^^^^^^^^^^^^^^ err: Enum variant pattern does not mention field `val2`
                };
            }
        }
    "#]]);
}

#[test]
fn test_missing_positional_fields_for_tuple_pattern() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            struct S(u8, u8);
            fun main(s: S) {
                let S (val) = s;
                     //^^^ warn: Unused variable 'val'
            }
        }
    "#]]);
}

#[test]
fn test_missing_positional_fields_with_rest() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            struct S(u8, u8);
            fun main(s: S) {
                let S(val, ..) = s;
                    //^^^ warn: Unused variable 'val'
            }
        }
    "#]]);
}

#[test]
fn test_missing_positional_fields_with_enum_variant() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            enum S { Inner(u8, u8) }
            fun main(s: S) {
                let S::Inner(val) = s;
                           //^^^ warn: Unused variable 'val'
            }
        }
    "#]]);
}
