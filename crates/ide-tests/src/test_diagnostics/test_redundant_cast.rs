use crate::ide_test_utils::diagnostics::{check_diagnostics, check_diagnostics_and_fix};
use expect_test::expect;

#[test]
fn test_no_error_on_correct_cast_from_integer() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            fun main() {
                1 as u64;
            }
        }
    "#]]);
}

#[test]
fn test_no_error_on_correct_case_from_u8_to_u64() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            fun main() {
                1u8 as u64;
            }
        }
    "#]]);
}

#[test]
fn test_u64_to_u64() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::main {
                fun main() {
                    1u64 as u64;
                       //^^^^^^ hint: No cast needed
                }
            }
        "#]],
        expect![[r#"
        module 0x1::main {
            fun main() {
                1u64;
            }
        }
    "#]],
    );
}

#[test]
fn test_no_error_in_msl() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            spec module {
                1u64 as u64;
            }
        }
    "#]]);
}
