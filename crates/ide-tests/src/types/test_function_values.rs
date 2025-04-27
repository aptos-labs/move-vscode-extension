use crate::resolve::check_resolve;
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

// language=Move
#[test]
fn test_function_value_named_wrapper() {
    check_expr_type(
        r#"
module 0x1::main {
    struct Predicate<T>(|&T|bool) has copy;
    fun main() {
        let a = Predicate(&22);
        a;
      //^ bool
    }
}
"#,
    )
}
