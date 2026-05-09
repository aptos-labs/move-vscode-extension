use crate::types::check_expr_type;

// language=Move
#[test]
fn test_type_of_forall_apply_lemma_quant() {
    check_expr_type(
        r#"
        module 0x1::main {
            fun main() {}
            spec lemma add_mono(_a: u64) {}
            spec main {} proof {
                forall a: u64 apply add_mono(a);
                                           //^ num
            }
        }
"#,
    )
}

// language=Move
#[test]
fn test_forall_expr_parameter_type() {
    check_expr_type(
        r#"
        module 0x1::main {
            fun main() {}
            spec main {
                forall a: u64: a == 1;
                             //^ num
            }
        }
"#,
    )
}

// language=Move
#[test]
fn lemma_variable_type() {
    check_expr_type(
        r#"
        module 0x1::main {
            spec module {
                lemma add_zero_right(x: u64) {
                    ensures x + 0 == x;
                          //^ num
                }
            }
        }
"#,
    )
}
