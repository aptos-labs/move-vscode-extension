use crate::resolve::check_resolve;

#[test]
fn test_module_with_self_from_use_speck() {
    // language=Move
    check_resolve(r#"
        module 0x1::transaction {
                      //X
        }
        module 0x1::main {
            use 0x1::transaction::Self;
                                 //^
        }
    "#)
}

#[test]
fn test_module_with_self_from_use_group() {
    // language=Move
    check_resolve(r#"
        module 0x1::transaction {
                      //X
        }
        module 0x1::main {
            use 0x1::transaction::{Self};
                                  //^
        }
    "#)
}

#[test]
fn test_module_with_self_from_qual_item() {
    // language=Move
    check_resolve(r#"
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
    "#)
}

#[test]
fn test_module_with_use_group_self_from_qual_item() {
    // language=Move
    check_resolve(r#"
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
    "#)
}

#[test]
fn test_resolve_self_to_current_module() {
    // language=Move
    check_resolve(r#"
        module 0x1::transaction {
                    //X
            fun create() {}
            fun main() {
                let a = Self::create();
                      //^
            }
        }
    "#)
}

#[test]
fn test_resolve_to_imported_module_with_alias() {
    // language=Move
    check_resolve(r#"
        module 0x1::Transaction {}
        module 0x1::m {
            use 0x1::Transaction as MyTransaction;
                                  //X
            fun main() {
                let a = MyTransaction::create();
                      //^
            }
        }
    "#)
}

#[test]
fn test_cannot_resolve_module_if_different_address() {
    // language=Move
    check_resolve(r#"
        module 0x1::transaction {}
        module 0x1::m {
            fun main() {
                let a = 0x3::transaction::create();
                             //^ unresolved
            }
        }
    "#)
}

#[test]
fn test_resolve_module_from_use_with_address_block() {
    // language=Move
    check_resolve(r#"
        address 0x1 {
            module A {
                 //X
            }

            module B {
                use 0x1::A;
                       //^
            }
        }
    "#)
}

#[test]
fn test_resolve_module_from_qual_item_with_address_block() {
    // language=Move
    check_resolve(r#"
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
    "#)
}

#[test]
fn test_cannot_be_resolved_without_import() {
    // language=Move
    check_resolve(r#"
        module 0x1::A {
            public fun create() {}
        }
        module 0x1::B {
            fun main() {
                let a = A::create();
                      //^ unresolved
            }
        }
    "#)
}


