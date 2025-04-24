use crate::resolve::check_resolve;
use crate::types::check_expr_type;

// language=Move
#[test]
fn test_type_of_inner_field() {
    check_expr_type(
        r#"
module 0x1::m {
    struct Inner { field: u8 }
    enum Outer { One { inner: Inner } }

    public fun non_exhaustive(o: &Outer) {
        match (o) {
            One { inner } => inner
                              //^ &0x1::m::Inner
        }
    }
}
"#,
    )
}

// language=Move
#[test]
fn test_type_of_deep_inner_field() {
    check_expr_type(
        r#"
module 0x1::m {
    struct Inner { field: u8 }
    enum Outer { One { inner: Inner } }

    public fun non_exhaustive(o: &Outer) {
        match (o) {
            One { inner: Inner { field: myfield } }
                => myfield
                    //^ u8
        }
    }
}
"#,
    )
}

#[test]
fn test_resolve_builtin_function_in_module_spec() {
    // language=Move
    check_expr_type(
        r#"
spec std::m {
    spec module {
        (len(vector[1, 2])) == 2;
      //^ num
    }
}
    "#,
    );
}

#[test]
fn test_infer_type_of_lambda_parameter() {
    // language=Move
    check_expr_type(
        r#"
module std::vector {
    public inline fun for_each_ref<Element>(self: &vector<Element>, f: |&Element|)  {}
}
module std::m {
    fun main() {
        vector[vector[true]].for_each_ref(|elem| { elem })
                                                   //^ &vector<bool>
    }
}
    "#,
    );
}
