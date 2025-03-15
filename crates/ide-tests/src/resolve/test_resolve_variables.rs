use crate::resolve::check_resolve;

#[test]
fn test_function_argument() {
    // language=Move
    check_resolve(
        r#"
        script {
            fun main(account: &signer) {
                   //X
                account;
              //^
            }
        }
    "#,
    )
}

#[test]
fn test_locals() {
    // language=Move
    check_resolve(
        r#"
        script {
            fun main() {
                let z = 1;
                  //X
                z;
              //^
            }
        }
    "#,
    )
}

#[test]
fn test_local_variables_has_a_priority_over_fun_variable() {
    // language=Move
    check_resolve(
        r#"
        script {
            fun main(z: u8) {
                let z = z + 1;
                  //X
                z;
              //^
            }
        }
    "#,
    )
}

#[test]
fn test_shadowing_of_variable_with_another_variable() {
    // language=Move
    check_resolve(
        r#"
        script {
            fun main() {
                let z = 1;
                let z = z + 1;
                  //X
                z;
              //^
            }
        }
    "#,
    )
}

#[test]
fn test_shadowing_does_not_happen_until_the_end_of_stmt() {
    // language=Move
    check_resolve(
        r#"
        script {
            fun main(z: u8) {
                   //X
                let z = z + 1;
                      //^
            }
        }
    "#,
    )
}

#[test]
fn test_redefinition_in_nested_block() {
    // language=Move
    check_resolve(
        r#"
        script {
            fun main() {
                let a = 1;
                  //X
                {
                    let a = 2;
                };
                a;
              //^
            }
        }
    "#,
    )
}

#[test]
fn test_variable_defined_in_nested_block() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::M {
            fun main() {
                let a = {
                    let b = 1;
                      //X
                    b + 1
                  //^
                };
            }
        }
    "#,
    )
}

#[test]
fn test_destructuring_of_struct() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::M {
            struct MyStruct {
                val: u8
            }

            fun destructure() {
                let MyStruct { val } = get_struct();
                             //X
                val;
              //^
            }
        }
    "#,
    )
}

#[test]
fn test_destructuring_of_struct_with_variable_rename() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::M {
            struct MyStruct {
                val: u8
            }

            fun destructure() {
                let MyStruct { val: myval } = get_struct();
                                  //X
                myval;
              //^
            }
        }
    "#,
    )
}

#[test]
fn test_type_params_used_in_cast_expr() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::M {
            fun convert<T>() {
                      //X
                1 as T
                   //^
            }
        }
    "#,
    )
}

#[test]
fn test_consts() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::M {
            const ENOT_GENESIS: u64 = 0;
                //X
            fun main() {
                let a = ENOT_GENESIS;
                      //^
            }
        }
    "#,
    )
}

#[test]
fn test_tuple_destructuring() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::M {
            fun main() {
                let (a, b) = call();
                   //X
                a;
              //^
            }
        }
    "#,
    )
}

#[ignore = "requires rename"]
#[test]
fn test_resolve_test_attribute_to_test_function_parameter() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::M {
            #[test(acc = @0x1)]
                  //^
            fun test_add(acc: signer) {
                        //X

            }
        }
    "#,
    )
}

#[ignore = "requires rename"]
#[test]
fn test_no_test_attribute_resolution_if_not_on_function() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::M {
            fun test_add(acc: signer) {
                #[test(acc = @0x1)]
                      //^ unresolved
                use 0x1::M;
            }
        }
    "#,
    )
}

#[ignore = "requires rename"]
#[test]
fn test_no_test_attribute_resolution_if_not_test_attribute() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::M {
            #[test]
            #[expected_failure(abort_code = 1)]
                                 //^ unresolved
            fun call(abort_code: signer) {

            }
        }
    "#,
    )
}

#[ignore = "requires rename"]
#[test]
fn test_no_attr_item_signer_reference_for_non_direct_children_of_test() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            #[test(unknown_attr(my_signer = @0x1))]
                                 //^ unresolved
            fun test_main(my_signer: signer) {
            }
        }
    "#,
    )
}

#[test]
fn test_test_only_const_in_test_function() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::M {
            #[test_only]
            const TEST_CONST: u64 = 1;
                  //X
            #[test]
            fun test_a() {
                TEST_CONST;
                    //^
            }
        }
    "#,
    )
}

#[test]
fn test_test_only_function_in_test_function() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::M {
            #[test_only]
            fun call() {}
               //X

            #[test]
            fun test_a() {
                call();
               //^
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_const_expected_failure() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::string {
            const ERR_ADMIN: u64 = 1;
                  //X
        }
        #[test_only]
        module 0x1::string_tests {
            use 0x1::string;

            #[test]
            #[expected_failure(abort_code = string::ERR_ADMIN)]
                                                    //^
            fun test_abort() {

            }
        }
    "#,
    )
}

#[test]
fn test_resolve_fq_const_expected_failure() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::string {
            const ERR_ADMIN: u64 = 1;
                  //X
        }
        #[test_only]
        module 0x1::string_tests {
            #[test]
            #[expected_failure(abort_code = 0x1::string::ERR_ADMIN)]
                                                        //^
            fun test_abort() {

            }
        }
    "#,
    )
}

#[test]
fn test_resolve_const_item_expected_failure() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::string {
            const ERR_ADMIN: u64 = 1;
                  //X
        }
        #[test_only]
        module 0x1::string_tests {
            use 0x1::string::ERR_ADMIN;

            #[test]
            #[expected_failure(abort_code = ERR_ADMIN)]
                                             //^
            fun test_abort() {

            }
        }
    "#,
    )
}

#[test]
fn test_resolve_const_item_same_module_expected_failure() {
    // language=Move
    check_resolve(
        r#"
        #[test_only]
        module 0x1::string_tests1 {
            const ERR_ADMIN: u64 = 1;
                //X

            #[test]
            #[expected_failure(abort_code = ERR_ADMIN)]
                                             //^
            fun test_abort() {

            }
        }
    "#,
    )
}

#[test]
fn test_resolve_const_import_expected_failure() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::string {
            const ERR_ADMIN: u64 = 1;
                  //X
        }
        #[test_only]
        module 0x1::string_tests {
            use 0x1::string::ERR_ADMIN;
                             //^
        }
    "#,
    )
}

#[test]
fn test_for_loop_index() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            fun main() {
                for (ind in 0..10) {
                    //X
                    ind;
                    //^
                }
            }
        }
    "#,
    )
}

#[test]
fn test_cannot_resolve_path_address() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            fun main() {
                0x1::;
                //^ unresolved
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_attribute_location() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
                  //X
            fun main() {
            }
            #[test(location=0x1::m)]
                               //^
            fun test_main() {

            }
        }
    "#,
    )
}

#[test]
fn test_resolve_variable_in_match_expr() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            fun main() {
                let m = 1;
                  //X
                match (m) {
                     //^
                }
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_function_with_match_name() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            fun match() {}
              //X
            fun main() {
                match();
                  //^
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_type_in_match_arm_1() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            enum S { One, Two }
               //X
            fun main() {
                let m = 1;
                match (m) {
                    S::One => true
                  //^
                    S::Two => false
                }
            }
        }
    "#,
    )
}

#[ignore = "requires types"]
#[test]
fn test_resolve_type_in_match_arm_2() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            enum S { One, Two }
                    //X
            fun main() {
                let m = 1;
                match (m) {
                    S::One => true
                      //^
                    S::Two => false
                }
            }
        }
    "#,
    )
}

#[ignore = "requires types"]
#[test]
fn test_resolve_type_in_match_arm_3() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            enum S { One, Two }
                         //X
            fun main() {
                let m = 1;
                match (m) {
                    S::One => true
                    S::Two => false
                      //^
                }
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_type_in_match_arm_body_1() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            enum S { One, Two }
            fun main() {
                let m = 1;
                  //X
                match (m) {
                    S::One => m
                            //^
                }
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_type_in_match_arm_body_2() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            enum S { One, Two }
            fun main(s: S) {
                   //X
                let m = 1;
                match (m) {
                    S::One => s
                            //^
                }
            }
        }
    "#,
    )
}

#[ignore = "requires types"]
#[test]
fn test_enum_variant_with_fields() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            enum S { One { field: u8 }, Two }
                    //X
            fun main() {
                let m = 1;
                match (m) {
                    S::One { field: f } => f
                      //^
                }
            }
        }
    "#,
    )
}

#[ignore = "requires types"]
#[test]
fn test_resolve_fields_for_enum_variant_in_match_arm() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            enum S { One { field: u8 }, Two }
                           //X
            fun main() {
                let m = 1;
                match (m) {
                    S::One { field: f } => f
                            //^
                }
            }
        }
    "#,
    )
}

#[ignore = "requires types"]
#[test]
fn test_resolve_shortcut_field_for_enum_variant_in_match_arm() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            enum S { One { field: u8 }, Two }
                           //X
            fun main() {
                let m = 1;
                match (m) {
                    S::One { field } => field
                            //^
                }
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_binding_for_field_reassignment_for_enum_variant() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            enum S { One { field: u8 }, Two }
            fun main() {
                let m = 1;
                match (m) {
                    S::One { field: myfield }
                                    //X
                        => myfield
                            //^
                }
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_binding_for_shortcut_field_with_enum_variant() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            enum S { One { field: u8 }, Two }
            fun main() {
                let m = 1;
                match (m) {
                    S::One { field }
                            //X
                        => field
                            //^
                }
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_field_for_struct_pat_in_enum() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            enum S { One { field: u8 }, Two }
                            //X
            fun main(s: S::One) {
                let S::One { field } = s;
                            //^
            }
        }
    "#,
    )
}

#[test]
fn test_resolve_field_assignment_for_struct_pat_in_enum() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            enum S { One { field: u8 }, Two }
                            //X
            fun main(s: S) {
                let S::One { field: f } = s;
                            //^
            }
        }
            "#,
    )
}

#[test]
fn test_resolve_field_assignment_for_struct_pat_in_enum_binding() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            enum S { One { field: u8 }, Two }
            fun main(s: S::One) {
                let S::One { field: f } = s;
                                  //X
                f;
              //^
            }
        }
            "#,
    )
}

#[test]
fn test_resolve_enum_variant_for_struct_lit() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            enum S { One { field: u8 }, Two }
                           //X
            fun main(s: S::One) {
                let f = 1;
                let s = S::One { field: f };
                                 //^
            }
        }
            "#,
    )
}

#[test]
fn test_resolve_enum_variant_for_struct_pat_1() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            enum S { One { field: u8 }, Two }
               //X
            fun main(s: S::One) {
                let S::One { field } = s;
                  //^
            }
        }
            "#,
    )
}

#[test]
fn test_resolve_enum_variant_for_struct_pat_2() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            enum S { One { field: u8 }, Two }
                   //X
            fun main(s: S::One) {
                let S::One { field } = s;
                      //^
            }
        }
            "#,
    )
}

#[test]
fn test_resolve_field_assignment_for_struct_lit_enum_variant() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            enum S { One { field: u8 }, Two }
                           //X
            fun main(s: S::One) {
                let f = 1;
                let s = S::One { field: f };
                                 //^
            }
        }
            "#,
    )
}

#[test]
fn test_resolve_field_assignment_for_struct_lit_enum_variant_binding() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            enum S { One { field: u8 }, Two }
            fun main(s: S::One) {
                let f = 1;
                  //X
                let s = S::One { field: f };
                                      //^
            }
        }
            "#,
    )
}

#[test]
fn test_shadow_global_spec_variable_with_local_one() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            spec module {
                global supply<CoinType>: num;
            }
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
fn test_outer_block_variable_with_inner_block_variable() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            fun main() {
                let supply = 1;
                spec {
                    let supply = 2;
                        //X
                    supply;
                    //^
                }
            }
        }
            "#,
    )
}

#[test]
fn test_unknown_struct_lit_variable_is_resolvable_with_shorthand() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            fun main() {
                let myfield = 1;
                     //X
                Unknown { myfield };
                           //^
            }
        }
            "#,
    )
}

#[test]
fn test_unknown_struct_lit_variable_is_resolvable_with_full_field() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            fun main() {
                let myfield = 1;
                     //X
                Unknown { field: myfield };
                                //^
            }
        }
            "#,
    )
}

#[test]
fn test_resolve_tuple_struct_pattern() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            struct S(u8, u8);
                 //X
            fun main(s: S) {
                let S ( field1, field2 ) = s;
                  //^
            }
        }
            "#,
    )
}

#[test]
fn test_resolve_variables_in_tuple_struct_pattern() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            struct S(u8, u8);
            fun main(s: S) {
                let S ( field1, field2 ) = s;
                          //X
                field1;
                //^
            }
        }
            "#,
    )
}

#[test]
fn test_pattern_with_rest() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            struct S { f1: u8, f2: u8 }
                     //X
            fun main(s: S) {
                let S { f1, .. } = s;
                       //^
            }
        }
            "#,
    )
}

#[test]
fn test_compound_assignment_lhs_binding() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            fun main() {
                let x = 1;
                  //X
                x += 1;
              //^
            }
        }
            "#,
    )
}

#[test]
fn test_compound_assignment_rhs_binding() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::m {
            fun main() {
                let x = 1;
                let y = 2;
                  //X
                x += y;
                   //^
            }
        }
            "#,
    )
}

#[test]
fn test_const_accessible_from_spec_function() {
    // language=Move
    check_resolve(
        r#"
        module 0x1::features {
            const PERMISSIONED_SIGNER: u64 = 84;
                   //X

        }
        module 0x1::m {}
        spec 0x1::m {
            spec fun is_permissioned_signer(): bool {
                use 0x1::features::PERMISSIONED_SIGNER;
                PERMISSIONED_SIGNER;
                //^
            }
        }
                "#,
    )
}
