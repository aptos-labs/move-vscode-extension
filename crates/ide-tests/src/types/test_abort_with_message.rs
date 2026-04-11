use crate::types::check_expr_type;

// language=Move
#[test]
fn test_abort_with_integer_type() {
    check_expr_type(
        r#"
module 0x1::m {
    fun main() {
        abort 1u64;
        abort 1u8;
        abort 1u16;
    }
}
"#,
    )
}

// language=Move
#[test]
fn test_abort_with_vector_u8() {
    check_expr_type(
        r#"
module 0x1::m {
    fun main() {
        abort b"1234";
        abort vector[1, 2, 3, 4];
        abort vector[1u128, 2u128];
    }
}
"#,
    )
}
