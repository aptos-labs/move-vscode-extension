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
