use crate::resolve::check_resolve;

#[test]
fn test_type_param_in_param_pos() {
    // language=Move
    check_resolve(r#"
        module 0x1::m {
            fun call<T>
                   //X
                    (val: T) {}
                        //^
        }
    "#)
}

#[test]
fn test_type_param_in_return_pos() {
    // language=Move
    check_resolve(r#"
        module 0x1::m {
            fun call<T>
                   //X
                    (): T {}
                      //^
        }
    "#)
}

#[test]
fn test_type_param_in_acquires() {
    // language=Move
    check_resolve(r#"
        module 0x1::m {
            fun call<T>
                   //X
                    () acquires T {}
                              //^
        }
    "#)
}

#[test]
fn test_type_param_in_call_expr() {
    // language=Move
    check_resolve(r#"
        module 0x1::m {
            fun convert<T>() {
                      //X
                call<T>()
                   //^
            }
        }
    "#)
}
