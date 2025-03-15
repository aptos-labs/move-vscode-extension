use crate::resolve::check_resolve;

#[test]
fn test_type_param_in_param_pos() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            fun call<T>
                   //X
                    (val: T) {}
                        //^
        }
    "#,
    )
}

#[test]
fn test_type_param_in_return_pos() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            fun call<T>
                   //X
                    (): T {}
                      //^
        }
    "#,
    )
}

#[test]
fn test_type_param_in_acquires() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            fun call<T>
                   //X
                    () acquires T {}
                              //^
        }
    "#,
    )
}

#[test]
fn test_type_param_in_call_expr() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            fun convert<T>() {
                      //X
                call<T>()
                   //^
            }
        }
    "#,
    )
}

#[test]
fn test_struct_type_param() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            struct MyStruct<T> {
                          //X
                val: T
                   //^
            }
        }
    "#,
    )
}

#[test]
fn test_struct_type_param_inside_vector() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            struct MyStruct<T> {
                          //X
                val: vector<T>
                          //^
            }
        }
    "#,
    )
}

#[test]
fn test_function_return_type_to_type_param() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            fun main<Token>()
                   //X
                : Token {}
                //^
        }
    "#,
    )
}

#[test]
fn test_function_return_type_argument_to_type_param() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            struct Coin<Token> {}

            fun main<Token>()
                   //X
                    : Coin<Token> {}
                         //^
        }
    "#,
    )
}

#[test]
fn test_native_function_return_type_argument_to_type_param() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            struct Coin<Token> {}

            native fun main<Token>()
                          //X
                    : Coin<Token>;
                         //^
        }
    "#,
    )
}

#[test]
fn test_resolve_type_param_in_native_function_in_spec() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::M {
            spec module {
                native fun serialize<MoveValue>(
                                        //X
                    v: &MoveValue
                        //^
                ): vector<u8>;
            }
        }
    "#,
    )
}

#[test]
fn test_type_param_in_return() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            public fun remove<K: copy + drop, V>(
                                            //X
                val: V
            ): V {
             //^
                val
            }
        }
"#,
    )
}
