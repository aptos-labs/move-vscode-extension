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
                0x1::original::call();
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
fn test_resolve_aliased_function() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::original {
            public fun call() {}
                      //X
        }
        module 0x1::m {
            use 0x1::original::call as mycall;
            fun main() {
                mycall();
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

#[test]
fn test_resolve_friend_function_with_named_address() {
    // language=Move
    check_resolve(
        r#"
        module aptos_std::original {
            friend aptos_std::m;
            public(friend) fun call() {}
                             //X
        }
        module aptos_std::m {
            use aptos_std::original;
            fun main() {
                original::call();
                         //^
            }
        }
    "#,
    )
}

#[test]
fn test_entry_is_unresolved_in_friend_modules() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::Original {
            friend 0x1::M;
            entry fun call() {}
        }
        module 0x1::M {
            use 0x1::Original;
            fun main() {
                Original::call();
                        //^ unresolved
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_script_function() {
    // language=Move
    check_resolve(
        r#"
        address 0x1 {
        module Original {
            public(script) fun call() {}
                             //X
        }
        }

        script {
            use 0x1::Original;
            fun main() {
                Original::call();
                        //^
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_public_entry_function() {
    // language=Move
    check_resolve(
        r#"
        address 0x1 {
        module Original {
            public entry fun call() {}
                             //X
        }
        }

        script {
            use 0x1::Original;
            fun main() {
                Original::call();
                        //^
            }
        }
    "#,
    )
}

#[test]
fn test_cannot_resolve_private_entry_function_from_script() {
    // language=Move
    check_resolve(
        r#"
        address 0x1 {
        module Original {
            entry fun call() {}
        }
        }

        script {
            use 0x1::Original;
            fun main() {
                Original::call();
                        //^ unresolved
            }
        }
    "#,
    )
}

#[test]
fn test_cannot_resolve_private_entry_function_from_another_module() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            entry fun call() {}
                      //X
        }
        module 0x1::main {
            use 0x1::m;
            fun main() {
                m::call();
                   //^ unresolved
            }
        }
    "#,
    )
}

#[test]
fn test_cannot_resolve_private_entry_function_from_another_module_entry_function() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            entry fun call() {}
                      //X
        }
        module 0x1::main {
            use 0x1::m;
            entry fun main() {
                m::call();
                   //^ unresolved
            }
        }
    "#,
    )
}

#[test]
fn test_public_script_is_the_same_as_public_entry() {
    // language=Move
    check_resolve(
        r#"
        address 0x1 {
        module Original {
            public(script) fun call() {}
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
fn test_cannot_resolve_const_from_other_module() {
    // language=Move
    check_resolve(
        r#"
        address 0x1 {
        module Original {
            const MY_CONST: u8 = 1;
        }

        module M {
            use 0x1::Original::MY_CONST;
            fun main() {
                MY_CONST;
                //^ unresolved
            }
        }
        }
    "#,
    )
}

#[test]
fn test_friend_function_unresolved_in_script() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::original {
            friend 0x1::m;
            public(friend) fun call() {}
        }
        module 0x1::m {}
        script {
            use 0x1::original;
            fun main() {
                original::call();
                        //^ unresolved
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_fully_qualified_path_from_the_same_module() {
    // language=Move
    check_resolve(
        r#"
    module 0x1::M {
        public fun call() {}
                 //X
        fun main() {
            0x1::M::call();
                    //^
        }
    }
    "#,
    )
}

#[test]
fn test_resolve_fun_in_test_only_module_from_another_test_only() {
    // language=Move
    check_resolve(
        r#"
    #[test_only]
    module 0x1::M1 {
        public fun call() {}
                  //X
    }
    #[test_only]
    module 0x1::M2 {
        use 0x1::M1::call;

        fun main() {
            call();
            //^
        }
    }
    "#,
    )
}

#[test]
fn test_test_only_function_is_not_accessible_from_non_test_only_module() {
    // language=Move
    check_resolve(
        r#"
    module 0x1::M1 {
        #[test_only]
        public fun call() {}
    }
    module 0x1::M2 {
        use 0x1::M1;
        fun call() {
            M1::call();
               //^ unresolved
        }
    }
    "#,
    )
}

#[test]
fn test_test_only_module_is_not_accessible_from_non_test_only_module() {
    // language=Move
    check_resolve(
        r#"
    #[test_only]
    module 0x1::M1 {
        public fun call() {}
    }
    module 0x1::M2 {
        use 0x1::M1;
        fun call() {
            M1::call();
           //^ unresolved
        }
    }
    "#,
    )
}

#[test]
fn test_unit_test_functions_not_accessible_as_local_items() {
    // language=Move
    check_resolve(
        r#"
    #[test_only]
    module 0x1::M {
        #[test]
        fun test_a() {}
        fun main() {
            test_a();
           //^ unresolved
        }
    }
    "#,
    )
}

#[test]
fn test_unit_test_functions_not_accessible_as_module_items() {
    // language=Move
    check_resolve(
        r#"
    #[test_only]
    module 0x1::M1 {
        #[test]
        entry fun test_a() {}
    }
    #[test_only]
    module 0x1::M2 {
        use 0x1::M1;

        entry fun main() {
            M1::test_a();
               //^ unresolved
        }
    }
    "#,
    )
}




