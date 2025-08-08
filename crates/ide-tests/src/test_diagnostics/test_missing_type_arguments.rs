// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::ide_test_utils::diagnostics::check_diagnostics;
use expect_test::expect;

#[test]
fn test_missing_type_arguments_for_vector() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            fun m() {
                let _a: vector<address>;
                let _b: vector;
                      //^^^^^^ err: Invalid instantiation of 'vector'. Expected 1 type argument(s), but got 0
                let _c: vector<u8, u8, u8>;
                      //^^^^^^^^^^^^^^^^^^ err: Invalid instantiation of 'vector'. Expected 1 type argument(s), but got 3
            }

            #[test(location = std::vector)]
            fun test_a() {

            }
        }
    "#]]);
}

#[test]
fn test_type_params_could_be_inferred_for_struct_literal() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct MyStruct<T> { field: T }

            fun main() {
                let _a = MyStruct { field: 1 };
            }
        }
    "#]]);
}

#[test]
fn test_no_type_arguments_expected() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct MyStruct { field: u8 }

            fun m() {
                let _a: MyStruct<u8>;
                              //^^^^ err: No type arguments expected for '0x1::M::MyStruct'
            }
        }
    "#]]);
}

#[test]
fn test_no_type_arguments_expected_for_imported_alias() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            struct MyStruct { field: u8 }
        }
        module 0x1::main {
            use 0x1::m::MyStruct as Struct;
            fun main() {
                let _a: Struct<u8>;
                            //^^^^ err: No type arguments expected for '0x1::m::MyStruct'
            }
        }
    "#]]);
}

#[test]
fn test_resource_type_could_be_inferred_for_move_to() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            fun main(s: signer) {
                let _a = move_to(&s, 1);
            }
        }
    "#]]);
}

#[test]
fn test_no_type_arguments_expected_for_function() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            fun call() {}
            fun main() {
                let _a = call<u8>();
                           //^^^^ err: No type arguments expected for '0x1::M::call'
            }
        }
    "#]]);
}

#[test]
fn test_generic_argument_type_can_be_inferred() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            fun call<T>(_val: T) {}
            fun main() {
                let _a = call(1);
            }
        }
    "#]]);
}

#[test]
fn test_too_many_type_arguments_for_function() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            fun call<T>() {}
            fun main() {
                let _a = call<u8, u8>();
                                //^^ err: Invalid instantiation of '0x1::M::call'. Expected 1 type argument(s), but got 2
            }
        }
    "#]]);
}

#[test]
fn test_missing_type_arguments_for_type() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct S<R> { r: R }
            struct Event { val: S }
                              //^ err: Invalid instantiation of '0x1::M::S'. Expected 1 type argument(s), but got 0
        }
    "#]]);
}

#[test]
fn test_too_many_type_arguments_for_type() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct S<R> { r: R }
            struct Event { val: S<u8, u8> }
                                    //^^ err: Invalid instantiation of '0x1::M::S'. Expected 1 type argument(s), but got 2
        }
    "#]]);
}

#[test]
fn test_no_need_for_type_arguments_inside_acquires() {
    // language=Move
    check_diagnostics(expect![[r#"
    module 0x1::M {
        struct S<phantom R> has key {}
        fun m() acquires S {
            borrow_global_mut<S<u8>>(@0x1);
        }
    }
    "#]]);
}

#[test]
fn test_wrong_number_of_type_arguments_for_struct() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct S<R, RR> {}
            fun m() {
                let _a = S<u8> {};
                        //^^^^ err: Invalid instantiation of '0x1::M::S'. Expected 2 type argument(s), but got 1
            }
        }
    "#]]);
}

#[test]
fn test_phantom_type_can_be_inferred_from_explicitly_passed_generic() {
    // language=Move
    check_diagnostics(expect![[r#"
    module 0x1::M {
        struct CapState<phantom Feature> has key {}
        fun m<Feature>(acc: &signer) {
            move_to<CapState<Feature>>(acc, CapState{})
        }
    }
    "#]]);
}

#[test]
fn test_phantom_type_can_be_inferred_from_another_struct_with_phantom_type() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct Slot<phantom Feature> has store {}
            struct Container<phantom Feature> has key { slot: Slot<Feature> }
            fun m<Feature>(_acc: &signer) {
                Container{ slot: Slot<Feature> {} };
            }
        }
    "#]]);
}

#[test]
fn test_not_enough_type_params_for_schema() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            spec schema MySchema<Type1, Type2> {}
            fun call() {}
            spec call {
                include MySchema<u8>;
                              //^^^^ err: Invalid instantiation of '0x1::M::MySchema'. Expected 2 type argument(s), but got 1
            }
        }
    "#]]);
}

// todo:
// #[test]
// fn test_missing_type_params_if_uninferrable_for_schema() {
//     // language=Move
//     check_diagnostics(expect![[r#"
//     module 0x1::M {
//         spec schema MySchema<Type> {}
//         fun call() {}
//         spec call {
//             include MySchema;
//         }
//     }
//     "#]]);
// }

#[test]
fn test_binding_receives_type_in_a_separate_block() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            struct Option<phantom Element> { m: vector<Element> }
            fun some<Element>(m: Element): Option<Element> { Option<Element>{ m: vector[m] }}
            fun none<Element>(): Option<Element> { Option<Element>{ m: vector[] } }
            fun main() {
                let (opt1, opt2);
                if (true) {
                    (opt1, opt2) = (some(1u8), some(1u8));
                } else {
                    (opt1, opt2) = (none(), none());
                }
            }
        }
    "#]]);
}

#[test]
fn test_method_missing_type_arguments() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            struct S<T> { field: T }
            fun receiver<T, U>(self: &S<T>, param: U): U {
                param
            }
            fun main(s: S<u8>) {
                let _b = s.receiver::<u8>(1);
                                 //^^^^^^ err: Invalid instantiation of '0x1::main::receiver'. Expected 2 type argument(s), but got 1
            }
        }
    "#]]);
}

#[test]
fn test_method_missing_type_arguments_without_colon_colon() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            struct S<T> { field: T }
            fun receiver<T, U>(self: &S<T>, param: U): U {
                param
            }
            fun main(s: S<u8>) {
                let _b = s.receiver<u8>(1);
                                 //^^^^ err: Invalid instantiation of '0x1::main::receiver'. Expected 2 type argument(s), but got 1
            }
        }
    "#]]);
}

#[test]
fn test_no_error_for_vector_in_module_position_and_unresolved() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun main() {
                vector::push_back();
            }
        }
    "#]]);
}

#[test]
fn test_no_error_for_vector_in_local_path_expr() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun main() {
                vector;
              //^^^^^^ err: Unresolved reference `vector`: cannot resolve
            }
        }
    "#]]);
}

#[test]
fn test_no_error_for_vector_in_type_position() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun main(_s: vector::Vector) {
            }
        }
    "#]]);
}

#[test]
fn test_no_error_for_vector_in_type_position_with_qualifier() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun main(_s: std::vector) {
            }
        }
    "#]]);
}

#[test]
fn test_type_position_argument_error_in_presence_of_vector_module() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::vector {}
        module 0x1::m {
            use 0x1::vector;
          //^^^^^^^^^^^^^^^^ warn: Unused use item
            fun main(_s: vector) {}
                       //^^^^^^ err: Invalid instantiation of 'vector'. Expected 1 type argument(s), but got 0
        }
    "#]]);
}
