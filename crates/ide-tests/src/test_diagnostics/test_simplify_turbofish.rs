use crate::ide_test_utils::diagnostics::{check_diagnostics, check_diagnostics_and_fix};
use expect_test::expect;

#[test]
fn test_no_warning_if_no_generic_parameters() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::mod {
            struct S<T> { val: T }
            fun receiver<T>(self: &S<T>): T {
                self.val
            }
            fun main(s: &S<u8>) {
                s.receiver();
            }
        }
    "#]])
}

#[test]
fn test_no_warning_if_generic_parameter_without_turbofish() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::mod {
            struct S<T> { val: T }
            fun receiver<T>(self: &S<T>): T {
                self.val
            }
            fun main(s: &S<u8>) {
                s.receiver<u8>();
            }
        }
    "#]])
}

#[test]
fn test_warning_if_turbofish_with_fix() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::mod {
                struct S<T> { val: T }
                fun receiver<T>(self: &S<T>): T {
                    self.val
                }
                fun main(s: &S<u8>) {
                    s.receiver::<u8>();
                            //^^ hint: `::` in method type arguments is deprecated
                }
            }
        "#]],
        expect![[r#"
            module 0x1::mod {
                struct S<T> { val: T }
                fun receiver<T>(self: &S<T>): T {
                    self.val
                }
                fun main(s: &S<u8>) {
                    s.receiver<u8>();
                }
            }
        "#]],
    )
}
