// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use expect_test::{Expect, expect};
use syntax::TextSize;
use syntax::files::FilePosition;
use syntax::pretty_print::{SourceMark, apply_source_marks};
use test_utils::{fixtures, get_and_replace_caret};

pub(crate) fn check_signature_info(source: &str, expect: Expect) {
    let (source, offset) = get_and_replace_caret(source, "/*caret*/");
    let (analysis, file_id) = fixtures::from_single_file(source.to_string());

    let signature_help = analysis
        .signature_help(FilePosition { file_id, offset })
        .unwrap()
        .expect("missing signature info");

    let mut signature_text = signature_help.signature.clone();

    if let Some(active_parameter_range) = signature_help
        .active_parameter
        .and_then(|it| signature_help.parameter_range(it))
    {
        let indent = ">>";
        let mark = SourceMark::at_range(active_parameter_range + TextSize::of(indent), "");
        signature_text = apply_source_marks(&format!("{indent}{signature_text}"), vec![mark]);
    }

    expect.assert_eq(&signature_text)
}

#[test]
fn test_fun_no_args() {
    // language=Move
    let source = r#"
        module 0x1::m {
            fun foo() {}
            fun main() { foo(/*caret*/); }
        }
            "#;

    check_signature_info(source, expect!["<no arguments>"]);
}

#[test]
fn test_fun_one_arg() {
    // language=Move
    let source = r#"
        module 0x1::m {
            fun foo(arg: u8) {}
            fun main() { foo(/*caret*/); }
        }
            "#;

    check_signature_info(
        source,
        expect![[r#"
        >>arg: u8
        //^^^^^^^
    "#]],
    );
}

#[test]
fn test_fun_one_arg_end() {
    // language=Move
    let source = r#"
        module 0x1::m {
            fun foo(arg: u8) {}
            fun main() { foo(42/*caret*/); }
        }
            "#;

    check_signature_info(
        source,
        expect![[r#"
        >>arg: u8
        //^^^^^^^
    "#]],
    );
}

#[test]
fn test_fun_many_args() {
    // language=Move
    let source = r#"
        module 0x1::m {
            fun foo(arg: u8, s: &signer, v: vector<u8>) {}
            fun main() { foo(/*caret*/); }
        }
            "#;

    check_signature_info(
        source,
        expect![[r#"
        >>arg: u8, s: &signer, v: vector<u8>
        //^^^^^^^
    "#]],
    );
}

#[test]
fn test_fun_many_args_vector_u8() {
    // language=Move
    let source = r#"
        module 0x1::M {
            fun call(a: u8, b: vector<u8>, c: vector<u8>) {}
            fun m() {
                call(1, b"11", b"22"/*caret*/);
            }
        }
            "#;

    check_signature_info(
        source,
        expect![[r#"
        >>a: u8, b: vector<u8>, c: vector<u8>
                              //^^^^^^^^^^^^^
    "#]],
    );
}

#[test]
fn test_fun_poorly_formatted_args() {
    // language=Move
    let source = r#"
        module 0x1::m {
            fun foo(arg:          u8,     s:    &signer,    v   : vector<u8>) {}
            fun main() { foo(/*caret*/); }
        }
            "#;

    check_signature_info(
        source,
        expect![[r#"
        >>arg: u8, s: &signer, v: vector<u8>
        //^^^^^^^
    "#]],
    );
}

#[test]
fn test_fun_args_index_0() {
    // language=Move
    let source = r#"
        module 0x1::m {
            fun foo(val1: u8, val2: u8) {}
            fun main() { foo(42/*caret*/); }
        }
            "#;

    check_signature_info(
        source,
        expect![[r#"
        >>val1: u8, val2: u8
        //^^^^^^^^
    "#]],
    );
}

#[test]
fn test_fun_args_index_1() {
    // language=Move
    let source = r#"
        module 0x1::m {
            fun foo(val1: u8, val2: u8) {}
            fun main() { foo(42, 10/*caret*/); }
        }
            "#;

    check_signature_info(
        source,
        expect![[r#"
        >>val1: u8, val2: u8
                  //^^^^^^^^
    "#]],
    );
}

#[test]
fn test_fun_args_multiline_call() {
    // language=Move
    let source = r#"
        module 0x1::m {
            fun foo(val1: u8, val2: u8) {}
            fun main() {
                foo(
                    42/*caret*/,
                    10
                );
            }
        }
            "#;

    check_signature_info(
        source,
        expect![[r#"
        >>val1: u8, val2: u8
        //^^^^^^^^
    "#]],
    );
}

#[test]
fn test_fun_args_builtin_function() {
    // language=Move
    let source = r#"
        module 0x1::m {
            fun main() {
                borrow_global(/*caret*/);
            }
        }
            "#;

    check_signature_info(
        source,
        expect![[r#"
        >>addr: address
        //^^^^^^^^^^^^^
    "#]],
    );
}

#[test]
fn test_fun_args_aliased_function() {
    // language=Move
    let source = r#"
        module 0x1::string {
            public fun call(addr: address) {}
        }
        module 0x1::m {
            use 0x1::string::call as mycall;
            fun main() {
                mycall(/*caret*/);
            }
        }
        "#;

    check_signature_info(
        source,
        expect![[r#"
        >>addr: address
        //^^^^^^^^^^^^^
    "#]],
    );
}

#[test]
fn test_incomplete_args_index_1() {
    // language=Move
    let source = r#"
        module 0x1::m {
            fun call(val1: u8, val2: u8) {}
            fun main() { call(42, /*caret*/); }
        }
        "#;

    check_signature_info(
        source,
        expect![[r#"
        >>val1: u8, val2: u8
                  //^^^^^^^^
    "#]],
    );
}

#[test]
fn test_incomplete_args_index_2() {
    // language=Move
    let source = r#"
        module 0x1::m {
            fun call(val1: u8, val2: u8, val3: u8) {}
            fun main() { call(42, 10, /*caret*/); }
        }
        "#;

    check_signature_info(
        source,
        expect![[r#"
        >>val1: u8, val2: u8, val3: u8
                            //^^^^^^^^
    "#]],
    );
}

#[test]
fn test_method_call() {
    // language=Move
    let source = r#"
        module 0x1::m {
            struct S { val: u8 }
            fun get_val(self: &S, modifier: bool): u8 { self.val }
            fun main(s: S) {
                s.get_val(/*caret*/);
            }
        }
        "#;

    check_signature_info(
        source,
        expect![[r#"
        >>modifier: bool
        //^^^^^^^^^^^^^^
    "#]],
    );
}

#[test]
fn test_method_call_called_as_function() {
    // language=Move
    let source = r#"
        module 0x1::m {
            struct S { val: u8 }
            fun get_val(self: &S, modifier: bool): u8 { self.val }
            fun main(s: S) {
                get_val(/*caret*/);
            }
        }
        "#;

    check_signature_info(
        source,
        expect![[r#"
        >>self: &S, modifier: bool
        //^^^^^^^^
    "#]],
    );
}

#[test]
fn test_fun_with_uninferred_generic_parameter() {
    // language=Move
    let source = r#"
        module 0x1::m {
            struct S<R> {}
            fun push_back<R>(self: &mut S<R>, e: R);
            fun main(s: S<u8>) {
                push_back(/*caret*/);
            }
        }
        "#;

    check_signature_info(
        source,
        expect![[r#"
        >>self: &mut S<R>, e: R
        //^^^^^^^^^^^^^^^
    "#]],
    );
}

#[test]
fn test_method_call_with_generic_parameter() {
    // language=Move
    let source = r#"
        module 0x1::m {
            struct S<R> {}
            fun push_back<R>(self: &mut S<R>, e: R);
            fun main(s: S<u8>) {
                s.push_back(/*caret*/);
            }
        }
        "#;

    check_signature_info(
        source,
        expect![[r#"
        >>e: u8
        //^^^^^
    "#]],
    );
}

#[test]
fn test_assert_macro() {
    // language=Move
    let source = r#"
        module 0x1::m {
            fun main() {
                assert!(/*caret*/);
            }
        }
        "#;

    check_signature_info(
        source,
        expect![[r#"
        >>_: bool, err: u64
        //^^^^^^^
    "#]],
    );
}

#[test]
fn test_named_tuple_struct() {
    // language=Move
    let source = r#"
        module 0x1::m {
            struct S(u8, u16);
            fun main() {
                S(/*caret*/);
            }
        }
        "#;

    check_signature_info(
        source,
        expect![[r#"
        >>u8, u16
        //^^
    "#]],
    );
}

#[test]
fn test_named_tuple_struct_with_generic_param() {
    // language=Move
    let source = r#"
        module 0x1::m {
            struct S<T>(T, T);
            fun main() {
                S<u8>(/*caret*/);
            }
        }
        "#;

    check_signature_info(
        source,
        expect![[r#"
        >>u8, u8
        //^^
    "#]],
    );
}

#[test]
fn test_tuple_enum_variant() {
    // language=Move
    let source = r#"
        module 0x1::m {
            enum S { One(u8, u8) }
            fun main() {
                S::One(/*caret*/);
            }
        }
        "#;

    check_signature_info(
        source,
        expect![[r#"
        >>u8, u8
        //^^
    "#]],
    );
}

#[test]
fn test_tuple_enum_variant_with_generics() {
    // language=Move
    let source = r#"
        module 0x1::m {
            enum S<T> { One(T, T) }
            fun main() {
                S<u8>::One(/*caret*/);
            }
        }
        "#;

    check_signature_info(
        source,
        expect![[r#"
        >>u8, u8
        //^^
    "#]],
    );
}
