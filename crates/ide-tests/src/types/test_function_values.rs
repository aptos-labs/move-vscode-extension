use crate::types::check_expr_type;

#[test]
fn test_fetch_function_value_from_struct_and_call() {
    // language=Move
    check_expr_type(
        r#"
module 0x1::m {
    struct R { val: u8 }
    struct S { fn: |address| R }
    fun main(s: &S) {
        (s.fn)(@0x1).val;
                   //^ u8
    }
}
    "#,
    );
}
