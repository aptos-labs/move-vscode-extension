use crate::resolve::check_resolve;

#[test]
fn test_struct_as_param_type() {
    // language=Move
    check_resolve(r#"
        module 0x1::m {
            struct MyStruct {}
                 //X

            fun call(s: MyStruct) {}
                      //^
        }
    "#)
}

#[test]
fn test_struct_with_generics_as_param_type() {
    // language=Move
    check_resolve(r#"
        module 0x1::m {
            struct Native<T> {}
                 //X
            fun main(n: Native<u8>): u8 {}
                      //^
        }
    "#)
}

#[test]
fn test_struct_as_return_type() {
    // language=Move
    check_resolve(r#"
        module 0x1::m {
            struct MyStruct {}
                 //X
            fun call(): MyStruct {}
                      //^
        }
    "#)
}

#[test]
fn test_struct_as_acquires_type() {
    // language=Move
    check_resolve(r#"
        module 0x1::m {
            struct MyStruct {}
                 //X

            fun call() acquires MyStruct {}
                              //^
        }
    "#)
}

#[test]
fn test_struct_for_struct_literal() {
    // language=Move
    check_resolve(r#"
        module 0x1::m {
            struct MyStruct {}
                 //X

            fun call() {
                let a = MyStruct {};
                      //^
            }
        }
    "#)
}

#[test]
fn test_cannot_resolve_struct_for_struct_literal_in_another_module() {
    // language=Move
    check_resolve(r#"
        module 0x1::s {
            struct MyStruct {}
        }
        module 0x1::m {
            use 0x1::s::MyStruct;
            fun call() {
                let a = MyStruct {};
                      //^ unresolved
            }
        }
    "#)
}

#[test]
fn test_resolve_struct_from_another_module_in_import() {
    // language=Move
    check_resolve(r#"
        module 0x1::s {
            struct MyStruct {}
                    //X
        }
        module 0x1::m {
            use 0x1::s::MyStruct;
                        //^
        }
    "#)
}

#[test]
fn test_resolve_struct_as_struct_pat() {
    // language=Move
    check_resolve(r#"
        module 0x1::m {
            struct MyStruct { val: u8 }
                 //X

            fun call() {
                let MyStruct { val } = get_struct();
                  //^
            }
        }
    "#)
}

#[test]
fn test_resolve_struct_as_type_argument() {
    // language=Move
    check_resolve(r#"
        module 0x1::m {
            struct MyStruct {}
                     //X

            fun call() {
                let a = move_from<MyStruct>();
                                //^
            }
        }
    "#)
}

#[test]
fn test_resolve_type_from_import() {
    // language=Move
    check_resolve(r#"
        address 0x1 {
            module Transaction {
                struct Sender {}
                     //X
            }
        }
        script {
            use 0x1::Transaction::Sender;
                                //^
        }
    "#)
}

#[test]
fn test_resolve_type_from_import_from_usage() {
    // language=Move
    check_resolve(r#"
        address 0x1 {
            module Transaction {
                struct Sender {}
                     //X
            }
        }
        script {
            use 0x1::Transaction::Sender;

            fun main(n: Sender) {}
                      //^
        }
    "#)
}

#[test]
fn test_resolve_type_to_alias() {
    // language=Move
    check_resolve(r#"
        module 0x1::Transaction {
            struct Sender { val: u8 }
        }
        module 0x1::m {
            use 0x1::Transaction::Sender as MySender;
                                          //X
            fun main(n: MySender) {}
                      //^
        }
    "#)
}

#[test]
fn test_unresolved_for_unresolved_alias() {
    // language=Move
    check_resolve(r#"
        module 0x1::m {
            use 0x1::Transaction::Sender as MySender;
            fun main(n: MySender) {}
                      //^ unresolved
        }
    "#)
}

#[test]
fn test_return_type_to_alias() {
    // language=Move
    check_resolve(r#"
        module 0x1::Transaction {
            struct Sender { val: u8 }
        }
        module 0x1::m {
            use 0x1::Transaction::Sender as MySender;
                                          //X
            fun main(): MySender {}
                      //^
        }
    "#)
}
