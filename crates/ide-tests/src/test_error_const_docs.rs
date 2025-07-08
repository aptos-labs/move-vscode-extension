use crate::ide_test_utils::diagnostics::check_diagnostics;
use expect_test::expect;

#[test]
fn test_no_error_if_const_is_not_used_as_error_in_assert_or_abort() {
    // language=Move
    check_diagnostics(expect![[r#"
            module 0x1::m {
                const ERR_ONE: u8 = 1;
                fun main() {
                    ERR_ONE + 1;
                }
            }
        "#]]);
}

#[test]
fn test_no_error_if_const_has_doc_comment() {
    // language=Move
    check_diagnostics(expect![[r#"
            module 0x1::m {
                /// docs
                const ERR_ONE: u8 = 1;
                /// docs
                const ERR_TWO: u8 = 1;
                fun main() {
                    assert!(true, ERR_ONE);
                    abort ERR_TWO;
                }
            }
        "#]]);
}

#[test]
fn test_no_error_path_has_colon_colon() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            const ERR_ONE: u8 = 1;
            fun main() {
                assert!(true, Self::ERR_ONE);
            }
        }
    "#]]);
}

#[test]
fn test_error_no_doc_comment_assert() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            const ERR_ONE: u8 = 1;
                //^^^^^^^ warn: Missing documentation comment (used as a human-readable error message on-chain)
            fun main() {
                assert!(true, ERR_ONE);
            }
        }
    "#]]);
}

#[test]
fn test_error_no_doc_comment_abort() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            const ERR_ONE: u8 = 1;
                //^^^^^^^ warn: Missing documentation comment (used as a human-readable error message on-chain)
            fun main() {
                abort ERR_ONE;
            }
        }
    "#]]);
}

#[test]
fn test_error_no_doc_comment_simple_comment_assert() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            // not docs
            const ERR_ONE: u8 = 1;
                //^^^^^^^ warn: Missing documentation comment (used as a human-readable error message on-chain)
            fun main() {
                assert!(true, ERR_ONE);
            }
        }
    "#]]);
}
