use crate::ide_test_utils::diagnostics::check_diagnostics_and_fix;
use expect_test::expect;

#[test]
fn test_replace_variable_assignment_with_plus() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::m {
                fun main() {
                    let x = 1;
                    x = x + 1;
                  //^^^^^^^^^ weak: Can be replaced with compound assignment
                }
            }
        "#]],
        expect![[r#"
            module 0x1::m {
                fun main() {
                    let x = 1;
                    x += 1;
                }
            }
        "#]],
    );
}

#[test]
fn test_replace_variable_assignment_with_left_shift() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::m {
                fun main() {
                    let x = 1;
                    x = x << 1;
                  //^^^^^^^^^^ weak: Can be replaced with compound assignment
                }
            }
        "#]],
        expect![[r#"
            module 0x1::m {
                fun main() {
                    let x = 1;
                    x <<= 1;
                }
            }
        "#]],
    );
}

#[test]
fn test_replace_deref_assignment_with_plus() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::m {
                fun main(p: &u8) {
                    *p = *p + 1;
                  //^^^^^^^^^^^ weak: Can be replaced with compound assignment
                }
            }
        "#]],
        expect![[r#"
            module 0x1::m {
                fun main(p: &u8) {
                    *p += 1;
                }
            }
        "#]],
    );
}
