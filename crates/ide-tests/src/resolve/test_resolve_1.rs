use crate::resolve::{check_resolve, check_resolve_files};
use expect_test::expect;

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

// language=Move
#[test]
fn test_resolve_spec_fun_from_spec_module_to_fun() {
    check_resolve(
        r#"
module 0x1::main {
    fun main() {}
       //X
}
spec 0x1::main {
    spec main {
        //^

    }
}
"#,
    )
}

// language=Move
#[test]
fn test_prioritize_variable_in_variable_context_in_case_of_multiple_resolution() {
    check_resolve(
        r#"
module 0x1::main {
    fun bytes() {}
    fun main(bytes: vector<u8>) {
            //X
        bytes;
         //^
    }
}
"#,
    )
}

// language=Move
#[test]
fn test_resolve_lambda_return_value_from_struct_field() {
    check_resolve(
        r#"
module 0x1::main {
    struct S<T, U> { settle_trade_f: |T, U| T }
    struct TT { val: u8 }
               //X
    struct UU { val: u16 }
    fun main(self: S<TT, UU>) {
        let tt = TT { val: 1 };
        let uu = UU { val: 1 };
        (self.settle_trade_f)(tt, uu).val;
                                     //^
    }
}
"#,
    )
}

// language=Move
#[test]
fn test_module_item_cross_file() {
    check_resolve_files(
        r#"
//- /m.move
module std::m {
    public fun call() {}
              //X
}
//- /main.move
module std::main {
    use std::m::call;
    public fun main() {
        call();
       //^
    }
}
"#,
    )
}

// language=Move
#[test]
fn test_module_item_cross_file_unresolved() {
    check_resolve_files(
        r#"
//- /m.move
module std::m {
    public fun call() {}
}
//- /main.move
module std::main {
    public fun main() {
        call();
       //^ unresolved
    }
}
"#,
    )
}

// language=Move
#[test]
fn test_module_unresolved_by_name_from_use_speck() {
    check_resolve(
        r#"
module std::m {
}
module std::main {
    use std::m::m;
              //^ unresolved
}
"#,
    )
}

#[test]
fn test_resolve_module_spec_without_address() {
    check_resolve(
        // language=Move
        r#"
spec main {
   //^ unresolved
}
    "#,
    )
}
