// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ide_test_utils::diagnostics::{check_diagnostics, check_diagnostics_on_tmpfs};
use expect_test::expect;
use test_utils::fixtures;
use test_utils::fixtures::test_state::{named_with_deps, raw};

#[test]
fn test_unresolved_variable() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            fun main() {
                x;
              //^ err: Unresolved reference `x`: cannot resolve
            }
        }
    "#]]);
}

#[test]
fn test_unresolved_function_call() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            fun main() {
                call();
              //^^^^ err: Unresolved reference `call`: cannot resolve
            }
        }
    "#]]);
}

#[test]
fn test_unresolved_module_member_with_unresolved_module() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            use 0x1::mod::call;
                   //^^^ err: Unresolved reference `mod`: cannot resolve

            fun main() {
                call();
              //^^^^ err: Unresolved reference `call`: cannot resolve
            }
        }
    "#]]);
}

#[test]
fn test_no_unresolved_reference_for_builtin() {
    // language=Move
    check_diagnostics(expect![[r#"
module 0x1::m {
    fun main() {
        move_from<u8>(@0x1);
    }
}
"#]]);
}

#[test]
fn test_no_unresolved_reference_for_primitive_type() {
    // language=Move
    check_diagnostics(expect![[r#"
script {
    fun main(_s: &signer) {
    }
}
"#]]);
}

#[test]
fn test_unresolved_reference_for_variable_in_struct_lit_field() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct T {
                my_field: u8
            }

            fun main() {
                let _t = T { my_field: my_unknown };
                                     //^^^^^^^^^^ err: Unresolved reference `my_unknown`: cannot resolve
            }
        }
    "#]]);
}

#[test]
fn test_no_unresolved_reference_for_field_shorthand() {
    // language=Move
    check_diagnostics(expect![[r#"
module 0x1::M {
    struct T {
        my_field: u8
    }

    fun main() {
        let my_field = 1;
        let _t = T { my_field };
    }
}
"#]]);
}

#[test]
fn test_unresolved_field_in_struct_lit() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct T {
                my_field: u8
            }

            fun main() {
                let _t = T { my_unknown_field: 1, my_field: 1 };
                           //^^^^^^^^^^^^^^^^ err: Unresolved reference `my_unknown_field`: cannot resolve

            }
        }
    "#]]);
}

#[test]
fn test_unresolved_field_in_struct_pat() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct T {
                my_field: u8
            }

            fun main() {
                let T { my_unknown_field: _, my_field: _ } = T { my_field: 1 };
                      //^^^^^^^^^^^^^^^^ err: Unresolved reference `my_unknown_field`: cannot resolve

            }
        }
    "#]]);
}

#[test]
fn test_unresolved_field_in_struct_pat_shorthand() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct T {
                my_field: u8
            }

            fun main() {
                let T { my_unknown_field, my_field: _ } = T { my_field: 1 };
                my_unknown_field;
            }
        }
    "#]]);
}

// todo: should be unresolved with named addresses info present
#[test]
fn test_unresolved_module() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            fun main() {
                let _t = transaction::create();
            }
        }
    "#]]);
}

#[test]
fn test_unresolved_fq_module() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            fun main() {
                let _t = std::transaction::create();
                            //^^^^^^^^^^^ err: Unresolved reference `transaction`: cannot resolve
            }
        }
    "#]]);
}

#[test]
fn test_unresolved_reference_for_method_of_another_module() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::other {}
        module 0x1::m {
            use 0x1::other;
            fun main() {
                other::emit();
                     //^^^^ err: Unresolved reference `emit`: cannot resolve
            }
        }
    "#]]);
}

#[test]
fn test_unresolved_reference_for_type_in_generic() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun deposit<Token> () {}

            fun main() {
                deposit<PONT>()
                      //^^^^ err: Unresolved reference `PONT`: cannot resolve
            }
        }
    "#]]);
}

#[test]
fn test_no_error_for_wildcard_in_struct_pat() {
    // language=Move
    check_diagnostics(expect![[r#"
module 0x1::M {
    struct Coin { value: u64 }
    fun call(): Coin { Coin { value: 1 } }
    fun m() {
        Coin { value: _ } = call();
    }
}
"#]]);
}

#[test]
fn test_no_error_correct_destructuring() {
    // language=Move
    check_diagnostics(expect![[r#"
module 0x1::M {
    struct Coin { value: u64 }
    fun call(): Coin { Coin { value: 1 } }
    fun m() {
        let val;
        Coin { value: val } = call();
    }
}
"#]]);
}

#[test]
fn test_error_for_unbound_destructured_value() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct Coin { value: u64 }
            fun call(): Coin { Coin { value: 1 } }
            fun m() {
                Coin { value: val } = call();
                            //^^^ err: Unresolved reference `val`: cannot resolve
            }
        }
    "#]]);
}

#[test]
fn test_no_error_for_result_variable_in_spec() {
    // language=Move
    check_diagnostics(expect![[r#"
module 0x1::M {
    fun call(): u8 { 1 }
    spec call {
        ensures result >= 1;
    }
}
"#]]);
}

#[test]
fn test_no_error_for_self_variable_in_spec() {
    // language=Move
    check_diagnostics(expect![[r#"
module 0x1::M {
    struct S { }
    spec S {
        self;
    }
}
"#]]);
}

#[test]
fn test_unresolved_reference_for_schema_field() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            spec schema Schema {}
            spec module {
                include Schema { addr: @0x1 };
            }
        }
    "#]]);
}

#[test]
fn test_unresolved_reference_for_schema_field_shorthand() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            spec schema Schema {}
            spec module {
                include Schema { addr };
                               //^^^^ err: Unresolved reference `addr`: cannot resolve
            }
        }
    "#]]);
}

#[test]
fn test_no_unresolved_reference_for_schema_field_and_function_param() {
    // language=Move
    check_diagnostics(expect![[r#"
    module 0x1::M {
        spec schema Schema {
            root_account: signer;
        }
        fun call(root_account: &signer) {}
        spec call {
            include Schema { root_account };
        }
    }
"#]]);
}

#[test]
fn test_no_error_for_tuple_result() {
    // language=Move
    check_diagnostics(expect![[r#"
    module 0x1::M {
        fun call(): (u8, u8) { (1, 1) }
        spec call {
            ensures result_1 == result_2
        }
    }
"#]]);
}

#[test]
fn test_no_error_for_update_field_arguments() {
    // language=Move
    check_diagnostics(expect![[r#"
    module 0x1::M {
        struct S { val: u8 }
        spec module {
            let s = S { val: 1 };
            ensures update_field(s, val, s.val + 1) == S { val: 2 };
        }
    }
"#]]);
}

#[test]
fn test_num_type() {
    // language=Move
    check_diagnostics(expect![[r#"
    module 0x1::M {
        spec schema SS {
            val: num;
        }
    }
"#]]);
}

#[test]
fn test_unresolved_field_for_dot_expr() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct S has key {}
            fun call() acquires S {
                let a = borrow_global_mut<S>(@0x1);
                a.val;
                //^^^ err: Unresolved reference `val`: cannot resolve
            }
        }
    "#]]);
}

#[test]
fn test_unresolved_module_import() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            use 0x1::M1;
          //^^^^^^^^^^^^ warn: Unused use item
                   //^^ err: Unresolved reference `M1`: cannot resolve
        }
    "#]]);
}

#[test]
fn test_unresolved_module_import_in_item_import() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            use 0x1::M1::call;
          //^^^^^^^^^^^^^^^^^^ warn: Unused use item
                   //^^ err: Unresolved reference `M1`: cannot resolve
        }
    "#]]);
}

#[test]
fn test_unresolved_item_import() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M1 {}
        module 0x1::Main {
            use 0x1::M1::call;
          //^^^^^^^^^^^^^^^^^^ warn: Unused use item
                       //^^^^ err: Unresolved reference `call`: cannot resolve
        }
    "#]]);
}

#[test]
fn test_no_error_for_field_of_item_of_unknown_type() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            fun main() {
                let var = (1 + false);
                             //^^^^^ err: Invalid argument to '+': expected integer type, but found 'bool'
                var.key;
            }
        }
    "#]]);
}

#[test]
fn test_no_error_for_field_of_reference_of_unknown_type() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            fun call<T>(t: T): &T { &t }
            fun main() {
                let var = &(1 + false);
                              //^^^^^ err: Invalid argument to '+': expected integer type, but found 'bool'
                var.key;
            }
        }
    "#]]);
}

#[test]
fn test_no_error_for_named_address_in_test_location() {
    // language=Move
    check_diagnostics(expect![[r#"
#[test_only]
module 0x1::string_tests {
    #[expected_failure(location = aptos_framework::coin)]
    fun test_abort() {
    }
}
"#]]);
}

#[test]
fn test_no_error_for_self_module_in_location() {
    // language=Move
    check_diagnostics(expect![[r#"
#[test_only]
module 0x1::string_tests {
    #[test]
    #[expected_failure(location = Self)]
    fun test_a() {

    }
}
"#]]);
}

#[test]
fn test_lhs_of_dot_assignment() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::mod {
            struct S { val: u8 }
            fun main() {
                s.val = 1;
              //^ err: Unresolved reference `s`: cannot resolve
            }
        }
    "#]]);
}

#[test]
fn test_no_error_for_attribute_item() {
    // language=Move
    check_diagnostics(expect![[r#"
module 0x1::m {
    #[resource_group(scope = global)]
    /// A shared resource group for storing object resources together in storage.
    struct ObjectGroup { }
}
"#]]);
}

#[test]
fn test_spec_builtin_not_available_outside_specs() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun main() {
                MAX_U128;
              //^^^^^^^^ err: Unresolved reference `MAX_U128`: cannot resolve
            }
        }
    "#]]);
}

#[test]
fn test_spec_builtin_const_inside_spec() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun main() {
                spec {
                    MAX_U128;
                }
            }
        }
    "#]]);
}

#[test]
fn test_no_unresolved_reference_in_pragma() {
    // language=Move
    check_diagnostics(expect![[r#"
module 0x1::m {
    spec module {
        pragma intrinsic = map;
    }
}
"#]]);
}

#[test]
fn test_no_unresolved_for_named_address_in_use() {
    // language=Move
    check_diagnostics(expect![[r#"
        module std::m {
        }
        module std::main {
            use std::m;
          //^^^^^^^^^^^ warn: Unused use item
        }
    "#]]);
}

#[test]
fn test_no_unresolved_for_named_address_in_fq() {
    // language=Move
    check_diagnostics(expect![[r#"
        module std::mymodule {
            public fun call() {}
        }
        module 0x1::main {
            fun main() {
                std::mymodule::call();
            }
        }
    "#]]);
}

#[test]
fn test_no_error_for_invariant_index_variable() {
    // language=Move
    check_diagnostics(expect![[r#"
module 0x1::m {
    spec module {
        let vec = vector[1, 2, 3];
        let ind = 1;
        invariant forall ind in 0..10: vec[ind] < 10;
    }
}
"#]]);
}

#[test]
fn test_unresolved_method() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            struct S { field: u8 }
            fun main(s: S) {
                s.receiver();
                //^^^^^^^^ err: Unresolved reference `receiver`: cannot resolve
            }
        }
    "#]]);
}

#[test]
fn test_no_unresolved_method_error() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            struct S { field: u8 }
            fun receiver(self: S): u8 { self.field }
            fun main(s: S) {
                s.receiver();
            }
        }
    "#]]);
}

#[test]
fn test_no_error_if_method_receiver_of_type_unknown() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            struct S { field: u8 }
            fun receiver(self: S): u8 { self.field }
            fun main() {
                let t = &(1 + false);
                            //^^^^^ err: Invalid argument to '+': expected integer type, but found 'bool'
                t.receiver();
            }
        }
    "#]]);
}

#[test]
fn test_no_error_for_fields_if_destructuring_unknown_struct() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun main() {
                let S { val } = 1;
                  //^ err: Unresolved reference `S`: cannot resolve
                val;
                let S(val) = 1;
                  //^ err: Unresolved reference `S`: cannot resolve
                val;
            }
        }
    "#]]);
}

#[test]
fn test_no_error_for_fields_if_destructuring_unknown_struct_with_qualifier() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            enum R {}
            fun main() {
                let R::Inner { val } = 1;
                     //^^^^^ err: Unresolved reference `Inner`: cannot resolve
                val;
                let R::Inner(val) = 1;
                     //^^^^^ err: Unresolved reference `Inner`: cannot resolve
                val;
            }
        }
    "#]]);
}

#[test]
fn test_no_error_path_in_attr() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            #[lint::my_lint]
            fun main() {}
        }
"#]]);
}

#[test]
fn test_no_error_for_unknown_receiver_method_of_result_of_unknown_resource_borrow() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun main() {
                let perm_storage = &PermissionStorage[@0x1];
                                  //^^^^^^^^^^^^^^^^^ err: Unresolved reference `PermissionStorage`: cannot resolve
                perm_storage.contains();
            }
        }
    "#]]);
}

#[test]
fn test_no_error_for_unknown_receiver_method_of_result_of_unknown_mut_resource_borrow() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun main() {
                let perm_storage = &mut PermissionStorage[@0x1];
                                      //^^^^^^^^^^^^^^^^^ err: Unresolved reference `PermissionStorage`: cannot resolve
                perm_storage.contains();
            }
        }
    "#]]);
}

#[test]
fn test_no_error_on_module_for_unresolved_module_if_same_name_as_address() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun main() {
                aptos_std::call();
                         //^^^^ err: Unresolved reference `call`: cannot resolve
            }
        }
    "#]]);
}

#[test]
fn test_error_on_known_item_of_module_with_the_same_name_as_address() {
    // language=Move
    check_diagnostics(expect![[r#"
        module aptos_std::aptos_std {
        }
        module 0x1::m {
            use aptos_std::aptos_std;
            fun main() {
                aptos_std::call();
                         //^^^^ err: Unresolved reference `call`: cannot resolve
            }
        }
    "#]]);
}

#[test]
fn test_no_error_for_const_in_spec() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::features {
            const PERMISSIONED_SIGNER: u64 = 84;

        }
        module 0x1::m {}
        spec 0x1::m {
            spec fun is_permissioned_signer(): bool {
                use 0x1::features::PERMISSIONED_SIGNER;
                PERMISSIONED_SIGNER;
                true
            }
        }
    "#]]);
}

#[test]
fn test_no_unresolved_reference_on_non_standard_named_address_in_friend_decl() {
    // language=Move
    check_diagnostics(expect![[r#"
        module publisher_address::features {
            const PERMISSIONED_SIGNER: u64 = 84;

        }
        module 0x1::m {
            friend publisher_address::features;
        }
"#]]);
}

#[test]
fn test_special_spec_constants_ignore_consts_with_the_same_names_from_the_module_scope() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            const MAX_U64: u128 = 18446744073709551615;
            fun main() {
            }
        }
        spec 0x1::m {
            spec main {
                MAX_U64;
            }
        }
"#]]);
}

#[test]
fn test_no_unresolved_address_for_spec_module() {
    // language=Move
    check_diagnostics(expect![[r#"
module aptos_experimental::mod {}
spec aptos_experimental::mod {}
"#]]);
}

#[test]
fn test_no_unresolved_address_for_fq_item_on_non_standard_address() {
    // language=Move
    check_diagnostics(expect![[r#"
        module aptos_experimental::mod {
            public fun call() {}
        }
        module 0x1::m {
            fun main() {
                aptos_experimental::mod::call();
            }
        }
    "#]]);
}

// language=Move
#[test]
fn test_no_unresolved_reference_for_multi_resolve() {
    check_diagnostics(expect![[r#"
module 0x1::M {
    struct T {
        my_field: u8
    }

    fun main() {
        let my_field = 1;
        T { my_field };
    }
}
"#]])
}

#[test]
fn test_shorthand_with_struct_lit_field_and_schema_field() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            struct S { val: u8 }
            spec schema SS {
                val: u8;
                S { val }
            }
        }
    "#]])
}

// todo: when named addresses are available
#[test]
fn test_unresolved_vector_module_in_expr() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun main() {
                vector::push_back();
            }
        }
    "#]])
}

// todo: when named addresses are available
#[test]
fn test_unresolved_vector_module_in_type_as_qualifier() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun main(
                _s: vector::Vector,
            ) {}
        }
    "#]])
}

#[test]
fn test_if_there_s_two_different_stdlibs_they_are_deduplicated_based_on_package_name() {
    let test_packages = vec![
        raw(
            "Std",
            "std_1",
            // language=Move
            r#"
//- vector.move
module std::vector {
    public native fun new<T>(): vector<T>;
}
"#,
        ),
        raw(
            "Std",
            "std_2",
            // language=Move
            r#"
//- vector.move
module std::vector {
    public native fun new<T>(): vector<T>;
}
"#,
        ),
        named_with_deps(
            "Dep",
            // language=TOML
            r#"
[dependencies]
Std = { local = "../std_1" }
        "#,
            // language=Move
            r#"
//- dep.move
module std::dep {}
            "#,
        ),
        named_with_deps(
            "Main",
            // language=TOML
            r#"
[dependencies]
Std = { local = "../std_2" }
Dep = { local = "../Dep" }
        "#,
            // language=Move
            r#"
//- main.move
module std::main {
    use std::vector::new;/*caret*/
    fun main() {
        new();
    }
}
            "#,
        ),
    ];
    let test_state = fixtures::from_multiple_files_on_tmpfs(test_packages);
    // language=Move
    check_diagnostics_on_tmpfs(
        test_state,
        expect![[r#"
            module std::main {
                use std::vector::new;/*caret*/
                fun main() {
                    new();
                  //^^^ err: Could not infer this type. Try adding a type annotation
                }
            }
        "#]],
    );
}
