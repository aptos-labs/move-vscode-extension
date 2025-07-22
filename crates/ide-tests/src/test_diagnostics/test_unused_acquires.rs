use crate::ide_test_utils::diagnostics::{check_diagnostics, check_diagnostics_and_fix};
use expect_test::expect;

#[test]
fn test_no_unused_acquires_on_simple_function() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            struct S has key { val: u8 }
            fun main() acquires S {
                move_from<S>(@0x1);
            }
        }
    "#]]);
}

#[test]
fn test_unused_acquires_on_inline_fun() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
        module 0x1::m {
            struct S has key { val: u8 }
            inline fun main() acquires S {
                            //^^^^^^^^^^ weak: Acquires on inline functions are not checked by the compiler and can be safely removed.
                move_from<S>(@0x1);
            }
        }
    "#]],
        expect![[r#"
        module 0x1::m {
            struct S has key { val: u8 }
            inline fun main() {
                move_from<S>(@0x1);
            }
        }
    "#]],
    );
}
