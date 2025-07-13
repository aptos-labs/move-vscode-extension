use crate::ide_test_utils::diagnostics::{check_diagnostics, check_diagnostics_and_fix};
use expect_test::expect;

#[test]
fn test_not_a_field_shorthand() {
    // language=Move
    check_diagnostics(expect![[r#"
        module std::main {
            struct S {
                field1: u8,
                field2: &u8
            }
            fun main() {
                let foo = 1;
                let field2 = 2;
                let _ = S { field1: foo, field2: &field2 };
            }
        }
    "#]]);
}

#[test]
fn test_can_be_shorthand_for_struct_lit() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
        module std::main {
            struct S {
                field1: u8,
            }
            fun main() {
                let field1 = 1;
                let _ = S { field1: field1 };
                          //^^^^^^^^^^^^^^ weak: Expression can be simplified
            }
        }
    "#]],
        expect![[r#"
        module std::main {
            struct S {
                field1: u8,
            }
            fun main() {
                let field1 = 1;
                let _ = S { field1 };
            }
        }
    "#]],
    );
}

#[test]
fn test_can_be_shorthand_for_struct_pat() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
        module std::main {
            struct S {
                field1: u8
            }
            fun main() {
                let S { field1: field1 };
                      //^^^^^^^^^^^^^^ weak: Expression can be simplified
                field1;
            }
        }
    "#]],
        expect![[r#"
            module std::main {
                struct S {
                    field1: u8
                }
                fun main() {
                    let S { field1 };
                    field1;
                }
            }
        "#]],
    );
}

#[test]
fn test_can_be_shorthand_for_struct_pat_in_match() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
        module std::main {
            enum S {
                One { field1: u8 }
            }
            fun main(s: S) {
                let _ = match (s) {
                    One { field1: field1 } => field1,
                        //^^^^^^^^^^^^^^ weak: Expression can be simplified
                };
            }
        }
    "#]],
        expect![[r#"
            module std::main {
                enum S {
                    One { field1: u8 }
                }
                fun main(s: S) {
                    let _ = match (s) {
                        One { field1 } => field1,
                    };
                }
            }
        "#]],
    );
}

#[test]
fn test_can_be_shorthand_for_schema_lit() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
        module std::main {
            spec schema S {
                field1: u8;
            }
            fun main() {
                spec {
                    let field1 = 1;
                    include S { field1: field1 };
                              //^^^^^^^^^^^^^^ weak: Expression can be simplified
                };
            }
        }
    "#]],
        expect![[r#"
            module std::main {
                spec schema S {
                    field1: u8;
                }
                fun main() {
                    spec {
                        let field1 = 1;
                        include S { field1 };
                    };
                }
            }
        "#]],
    );
}
