use crate::resolve::check_resolve;

#[test]
fn test_module_with_self_from_use_speck() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::transaction {
                      //X
        }
        module 0x1::main {
            use 0x1::transaction::Self;
                                 //^
        }
    "#,
    )
}

#[test]
fn test_module_with_self_from_use_group() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::transaction {
                      //X
        }
        module 0x1::main {
            use 0x1::transaction::{Self};
                                  //^
        }
    "#,
    )
}

#[test]
fn test_module_with_self_from_qual_item() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
                  //X
            fun create() {}
        }
        module 0x1::main {
            use 0x1::m::Self;
            fun main() {
                let a = m::create();
                      //^
            }
        }
    "#,
    )
}

#[test]
fn test_module_with_use_group_self_from_qual_item() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
                  //X
            fun create() {}
        }
        module 0x1::main {
            use 0x1::m::{Self};
            fun main() {
                let a = m::create();
                      //^
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_self_to_current_module() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::transaction {
                    //X
            fun create() {}
            fun main() {
                let a = Self::create();
                      //^
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_to_imported_module_with_alias() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::Transaction {}
                     //X
        module 0x1::m {
            use 0x1::Transaction as MyTransaction;
            fun main() {
                let a = MyTransaction::create();
                      //^
            }
        }
    "#,
    )
}

#[test]
fn test_cannot_resolve_module_if_different_address() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::transaction {}
        module 0x1::m {
            fun main() {
                let a = 0x3::transaction::create();
                             //^ unresolved
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_module_from_use_with_address_block() {
    // language=Move
    check_resolve(
        r#"
        address 0x1 {
            module A {
                 //X
            }

            module B {
                use 0x1::A;
                       //^
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_module_from_qual_item_with_address_block() {
    // language=Move
    check_resolve(
        r#"
        address 0x1 {
            module A {
                 //X
            }

            module B {
                use 0x1::A;

                fun main() {
                    let a = A::create();
                          //^
                }
            }
        }
    "#,
    )
}

#[test]
fn test_cannot_be_resolved_without_import() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::A {
            public fun create() {}
        }
        module 0x1::B {
            fun main() {
                let a = A::create();
                      //^ unresolved
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_module_with_different_address() {
    // language=Move
    check_resolve(
        r#"
        module 0x2::A {}
                  //X
        module 0x1::B {
            use 0x2::A;

            fun main() {
                let a = A::create();
                      //^
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_module_to_address_block_from_script() {
    // language=Move
    check_resolve(
        r#"
        address 0x2 {
            module A {
                 //X
            }
        }

        script {
            use 0x2::A;

            fun main() {
                let a = A::create();
                      //^
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_to_address_block_fully_qualified() {
    // language=Move
    check_resolve(
        r#"
        address 0x2 {
            module A {
                 //X
            }
        }

        address 0x1 {
            module B {
                fun main() {
                    let a = 0x2::A::create();
                               //^
                }
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_to_address_block_with_address_normalization() {
    // language=Move
    check_resolve(
        r#"
        address 0x0002 {
            module A {
                 //X
            }
        }

        address 0x1 {
            module B {
                use 0x02::A;

                fun main() {
                    let a = A::create();
                          //^
                }
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_module_from_self() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::M {
                  //X
            struct MyStruct {}
        }
        module 0x1::Main {
            use 0x1::M::{Self, MyStruct};
                        //^
        }
    "#,
    )
}

#[test]
fn test_resolve_module_from_self_with_alias() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::M {
                  //X
            struct MyStruct {}
        }
        module 0x1::Main {
            use 0x1::M::{Self as MyM, MyStruct};
                        //^
        }
    "#,
    )
}
