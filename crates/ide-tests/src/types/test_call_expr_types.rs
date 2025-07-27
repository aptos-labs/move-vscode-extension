// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use syntax::files::FilePosition;
use test_utils::tracing::init_tracing_for_test;
use test_utils::{fixtures, get_marked_position_offset_with_data};

pub fn check_call_expr_type(source: &str) {
    init_tracing_for_test();

    let (analysis, file_id) = fixtures::from_single_file(source.to_string());

    let (ref_offset, data) = get_marked_position_offset_with_data(&source, "//^");
    let position = FilePosition { file_id, offset: ref_offset };

    let opt_ty = analysis.call_expr_type_info(position).unwrap();
    let expr_ty = opt_ty.expect("could not find an call expr / outside inference context");

    assert_eq!(expr_ty, data);
}

#[test]
fn test_infer_call_expr_option_none_from_lit_field_1() {
    // language=Move
    check_call_expr_type(
        r#"
        module 0x1::main {
            struct Option<Element> { vec: vector<Element> }
            struct Iterable<Element> { el: Option<Element> }
            public fun none<Element>(): Option<Element> {
                Option { vec: vector[] }
            }
            public fun main() {
                Iterable<u8> { el: none() };
                                  //^ fn() -> 0x1::main::Option<u8>
            }
        }
    "#,
    )
}

#[test]
fn test_infer_call_expr_option_none_from_lit_field_2() {
    // language=Move
    check_call_expr_type(
        r#"
        module 0x1::option {
            struct Option<Element> {
                vec: vector<Element>
            }
            public fun none<Element>(): Option<Element> {
                Option { vec: vector[] }
            }
        }
        module 0x1::main {
            use 0x1::option;
            struct IterableValue<K> {
                prev: option::Option<K>,
            }
            public fun new() {
                IterableValue { prev: option::none() };
                                             //^ fn() -> 0x1::option::Option<Element>
            }
        }    "#,
    )
}

#[test]
fn test_call_expr_with_unset_generic_parameter() {
    // language=Move
    check_call_expr_type(
        r#"
        module std::main {
            fun call<T>(_t: &vector<T>) {}
            fun main() {
                call(vector[]);
                //^ fn(&vector<T>)
            }
        }
        "#,
    )
}

#[test]
fn test_call_expr_with_missing_generic_typed_parameter() {
    // language=Move
    check_call_expr_type(
        r#"
        module std::main {
            fun call<T>(_t: &T) {}
            fun main() {
                call();
                //^ fn(&T)
            }
        }
        "#,
    )
}

#[test]
fn test_call_expr_with_set_generic_type_parameter_but_missing_params() {
    // language=Move
    check_call_expr_type(
        r#"
        module std::main {
            fun call<T>(_t: &T, a: u8) {}
            fun main() {
                call(&vector[1]);
                //^ fn(&vector<integer>, u8)
            }
        }
        "#,
    )
}
