use crate::resolve::{check_resolve, check_resolve_tmpfs};
use test_utils::fixtures::test_state::named;

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

#[test]
fn test_module_item_cross_file() {
    check_resolve_tmpfs(vec![named(
        "Main",
        // language=Move
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
    )])
}

#[test]
fn test_module_item_cross_file_unresolved() {
    check_resolve_tmpfs(vec![named(
        "Main",
        // language=Move
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
    )])
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

#[test]
fn test_resolve_spec_function_from_module_spec_with_no_path() {
    check_resolve(
        // language=Move
        r#"
module 0x1::main {}
spec {
    spec main {
        //^ unresolved
    }
}
    "#,
    )
}

#[test]
fn test_resolve_spec_function_from_module_spec_with_no_path_with_address() {
    check_resolve(
        // language=Move
        r#"
module 0x1::main {}
spec {
    spec 0x1::main {
             //^ no reference
    }
}
    "#,
    )
}

#[test]
fn test_resolve_spec_function_parameter_with_invalid_pat() {
    check_resolve(
        // language=Move
        r#"
module 0x1::main {
    fun main(a: u8, b: u8, _: u8) {}
           //X
}
spec 0x1::main {
    spec main(a: u8, b: u8, _: u8) {}
            //^
}
    "#,
    )
}

#[test]
fn test_resolve_friend_function_with_non_standard_named_address() {
    check_resolve(
        // language=Move
        r#"
module aptos_token_objects::collection {
    friend aptos_token_objects::token;
    friend fun decrement_supply() {}
                //X
}
module aptos_token_objects::token {
    use aptos_token_objects::collection;
    fun main() {
        collection::decrement_supply();
                     //^
    }
}
    "#,
    )
}

#[test]
fn test_resolve_to_variable_in_presence_of_global_var_with_same_name() {
    check_resolve(
        // language=Move
        r#"
spec std::m {
    spec module {
        global supply<CoinType>: num;
    }
}
module std::m {
    fun supply(): u8 { 1 }
    fun main() {
        let supply = 1;
           //X
        spec {
            supply;
            //^
        }
    }
}
    "#,
    )
}

#[test]
fn test_resolve_const_in_script() {
    check_resolve(
        // language=Move
        r#"
script {
    const MY_CONST: u64 = 1;
          //X
    fun main() {
        MY_CONST;
         //^
    }
}
    "#,
    )
}

#[test]
fn test_resolve_match_struct_lit_enum_field_to_the_function_value() {
    check_resolve(
        // language=Move
        r#"
module 0x1::main {
    enum S { Variant { field: |bool| bool }}
                       //X
    fun main() {
        ()(S::Variant { field: myfield });
                          //^
    }
}    "#,
    )
}

#[test]
fn test_resolve_match_struct_lit_field_to_the_function_value() {
    check_resolve(
        // language=Move
        r#"
module 0x1::main {
    struct S { field: |bool| bool }
                //X
    fun main() {
        ()(S { field: |_| true });
                //^
    }
}    "#,
    )
}
