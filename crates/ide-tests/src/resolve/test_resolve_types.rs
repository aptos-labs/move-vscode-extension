use crate::resolve::check_resolve;

#[test]
fn test_struct_as_param_type() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            struct MyStruct {}
                 //X

            fun call(s: MyStruct) {}
                      //^
        }
    "#,
    )
}

#[test]
fn test_struct_with_generics_as_param_type() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            struct Native<T> {}
                 //X
            fun main(n: Native<u8>): u8 {}
                      //^
        }
    "#,
    )
}

#[test]
fn test_struct_as_return_type() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            struct MyStruct {}
                 //X
            fun call(): MyStruct {}
                      //^
        }
    "#,
    )
}

#[test]
fn test_struct_as_acquires_type() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            struct MyStruct {}
                 //X

            fun call() acquires MyStruct {}
                              //^
        }
    "#,
    )
}

#[test]
fn test_struct_for_struct_literal() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            struct MyStruct {}
                 //X

            fun call() {
                let a = MyStruct {};
                      //^
            }
        }
    "#,
    )
}

#[test]
fn test_cannot_resolve_struct_for_struct_literal_in_another_module() {
    // language=Move
    check_resolve(
        r#"
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
    "#,
    )
}

#[test]
fn test_resolve_struct_from_another_module_in_import() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::s {
            struct MyStruct {}
                    //X
        }
        module 0x1::m {
            use 0x1::s::MyStruct;
                        //^
        }
    "#,
    )
}

#[test]
fn test_resolve_struct_as_struct_pat() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            struct MyStruct { val: u8 }
                 //X

            fun call() {
                let MyStruct { val } = get_struct();
                  //^
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_struct_as_type_argument() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            struct MyStruct {}
                     //X

            fun call() {
                let a = move_from<MyStruct>();
                                //^
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_type_from_import() {
    // language=Move
    check_resolve(
        r#"
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
    "#,
    )
}

#[test]
fn test_resolve_type_from_import_from_usage() {
    // language=Move
    check_resolve(
        r#"
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
    "#,
    )
}

#[test]
fn test_resolve_type_to_alias() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::Transaction {
            struct Sender { val: u8 }
                  //X
        }
        module 0x1::m {
            use 0x1::Transaction::Sender as MySender;
            fun main(n: MySender) {}
                      //^
        }
    "#,
    )
}

#[test]
fn test_unresolved_for_unresolved_alias() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            use 0x1::Transaction::Sender as MySender;
            fun main(n: MySender) {}
                      //^ unresolved
        }
    "#,
    )
}

#[test]
fn test_return_type_to_alias() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::Transaction {
            struct Sender { val: u8 }
                 //X
        }
        module 0x1::m {
            use 0x1::Transaction::Sender as MySender;
            fun main(): MySender {}
                      //^
        }
    "#,
    )
}

#[test]
fn test_struct_unresolved_in_name_expr() {
    // language=Move
    check_resolve(
        r#"
        address 0x1 {
            module A {
                struct S {}
            }
            module B {
                use 0x1::A;
                fun call() {
                    A::S
                     //^ unresolved
                }
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_type_from_use_item() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::M {
            struct MyStruct {}
                   //X
        }
        module 0x1::Main {
            use 0x1::M::{Self, MyStruct};
                              //^
        }
    "#,
    )
}

#[test]
fn test_resolve_type_for_local_import() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::table {
            struct Table {}
                   //X
        }
        module 0x1::main {
            struct S<phantom T> has key {}
            fun main() {
                use 0x1::table::Table;

                assert!(exists<S<Table>>(@0x1), 1);
                                 //^
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_type_for_local_import_in_spec() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::table {
            struct Table {}
                   //X
        }
        module 0x1::main {
            struct S<phantom T> has key {}
            fun main() {}
        }
        spec 0x1::main {
            spec main {
                use 0x1::table::Table;

                assert!(exists<S<Table>>(@0x1), 1);
                                 //^
            }
        }
    "#,
    )
}

#[test]
fn test_resource_index_expr() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            struct S has key {}
                 //X
            fun main() {
                S[@0x1];
              //^
            }
        }
    "#,
    )
}

#[test]
fn test_no_module_at_type_position() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::Transaction {
            struct Type {
                val: u8
            }
        }
        module 0x1::M {
            fun main(a: 0x1::Transaction::Transaction) {
                                           //^ unresolved
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_enum_as_qualifier_of_variant_at_type_position() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            enum S { One, Two }
               //X
            fun main(one: S::One) {
                        //^
            }
        }
    "#,
    )
}

#[test]
fn test_cannot_resolve_enum_variant_at_type_position() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            enum S { One, Two }
            fun main(one: S::One) {
                            //^ unresolved
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_enum_type_from_module_import() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            enum S { One, Two }
               //X
        }
        module 0x1::main {
            use 0x1::m;
            fun main(one: m::S) {
                           //^
            }
        }
    "#,
    )
}
