use crate::resolve::check_resolve;
use crate::types::check_expr_type;

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

#[test]
fn test_resolve_lambda_expr_with_lambda_and_wildcard_pattern_with_invalid_binary_expr() {
    // language=Move
    check_resolve(
        r#"
module 0x1::m {
    fun main() {
        let a = 1;
          //X
        let s = self.all(|_| {
            a; 1 + });
          //^
    }
}
"#,
    )
}

#[test]
fn test_resolve_field_of_index_expr() {
    // language=Move
    check_resolve(
        r#"
module 0x1::m {
    struct Features { features: u8 }
                        //X
    fun main() {
        if (true) {
        } else if (true) {
            Features[@std].features;
                           //^
        }
    }
}
    "#,
    );
}

#[test]
fn test_cannot_resolve_borrow_global_mut() {
    // language=Move
    check_resolve(
        r#"
module 0x1::m {
    fun main() {
        borrow_global_mut<u8>(@0x1);
         //^ unresolved
    }
}
    "#,
    );
}

#[test]
fn test_resolve_module_from_module_spec_with_named_address() {
    // language=Move
    check_resolve(
        r#"
module std::m {
          //X
}
spec std::m {
        //^
}
    "#,
    );
}

// language=Move
#[test]
fn test_integer_inference_with_spec_blocks_inside_block() {
    check_resolve(
        r#"
module 0x1::main {
    spec fun get_num(): num { 1 }
    fun main() {
        let myint = 1;
            //X
        myint + 1u8;
        spec {
            myint
            //^ num
        };
    }
}
"#,
    )
}
