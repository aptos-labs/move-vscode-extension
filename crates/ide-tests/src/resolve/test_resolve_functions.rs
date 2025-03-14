use crate::resolve::check_resolve;

#[test]
fn test_reference_to_function() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            fun call(): u8 {
              //X
                1
            }

            fun main() {
                call();
              //^
            }
        }
    "#,
    )
}

#[test]
fn test_reference_to_native_function() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            native fun call(): u8;
                     //X

            fun main() {
                call();
              //^
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_to_the_same_module_full_path() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            public fun call() {}
                     //X
            fun main() {
                0x1::m::call();
                      //^
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_to_another_module_by_full_path() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::original {
            public fun call() {}
                     //X
        }
        module 0x1::m {
            fun call() {}

            fun main() {
                0x1::Original::call();
                             //^
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_to_another_module_by_module_import() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::original {
            public fun call() {}
                     //X
        }
        module 0x1::m {
            use 0x1::original;

            fun call() {}

            fun main() {
                original::call();
                        //^
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_to_another_module_from_import() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::original {
            public fun call() {}
                       //X
        }
        module 0x1::m {
            use 0x1::original::call;
                              //^
        }
    "#,
    )
}

#[test]
fn test_resolve_to_another_module_by_member_import() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::original {
            public fun call() {}
                       //X
        }
        module 0x1::m {
            use 0x1::original::call;
            fun main() {
                call();
               //^
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_function_to_alias() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::original {
            public fun call() {}
        }
        module 0x1::m {
            use 0x1::original::call as mycall;
                                       //X
            fun main() {
                call();
               //^
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_to_another_module_by_member_import_on_another_address() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::original {
            public fun call() {}
                       //X
        }
        module 0x2::m {
            use 0x1::original::call;
            fun main() {
                call();
               //^
            }
        }
    "#,
    )
}

#[test]
fn test_cannot_resolve_private_function_from_import() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::original {
            fun call() {}
        }
        module 0x1::m {
            use 0x1::original::call;
                             //^ unresolved
        }
    "#,
    )
}

#[test]
fn test_resolve_function_by_module_import_in_address_blocks() {
    // language=Move
    check_resolve(
        r#"
        address 0x1 {
            module Original {
                public fun call() {}
                         //X
            }
        }
        address 0x2 {
            module M {
                use 0x1::Original;

                fun main() {
                    Original::call();
                            //^
                }
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_via_self() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            fun call(): u8 {
              //X
                1
            }

            fun main() {
                Self::call();
                    //^
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_friend_function_with_public_friend_modifier() {
    // language=Move
    check_resolve(
        r#"
        address 0x1 {
        module Original {
            friend 0x1::M;
            public(friend) fun call() {}
                             //X
        }

        module M {
            use 0x1::Original;
            fun main() {
                Original::call();
                        //^
            }
        }
        }
    "#,
    )
}

#[test]
fn test_resolve_friend_function_with_friend_modifier() {
    // language=Move
    check_resolve(
        r#"
        address 0x1 {
        module Original {
            friend 0x1::M;
            friend fun call() {}
                       //X
        }

        module M {
            use 0x1::Original;
            fun main() {
                Original::call();
                        //^
            }
        }
        }
    "#,
    )
}

#[test]
fn test_unresolved_friend_function_if_friend_of_another_module() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m1 {
            friend 0x1::main;
        }
        module 0x1::m2 {
            friend fun call() {}
        }
        module 0x1::main {
            use 0x1::m2;
            fun main() {
                m2::call();
                   //^ unresolved
            }
        }
    "#,
    )
}
