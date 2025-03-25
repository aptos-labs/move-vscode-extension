use crate::resolve::check_resolve;

#[test]
fn test_resolve_base_for_index_expr() {
    // language=Move
    check_resolve(
        r#"
module 0x1::m {
    struct S { val: u8 }
         //X
    fun main() acquires S {
        S[@0x1].val;
      //^
    }
}
"#,
    )
}

#[test]
fn test_resolve_dot_field_for_index_expr() {
    // language=Move
    check_resolve(
        r#"
module 0x1::m {
    struct S { val: u8 }
              //X
    fun main() acquires S {
        S[@0x1].val;
               //^
    }
}
"#,
    )
}

#[test]
fn test_resolve_dot_field_for_dot_field_for_index_expr() {
    // language=Move
    check_resolve(
        r#"
module 0x1::m {
    struct S { val: u8 }
              //X
    struct T { val: S }
    fun main() acquires T {
        T[@0x1].val.val;
                   //^
    }
}
"#,
    )
}