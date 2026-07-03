use expect_test::{Expect, expect};
use fmt::config::CstFormatConfig;
use fmt::fmt::format_content;

fn check_fmt(config: CstFormatConfig, input: &str, expected: Expect) {
    let actual = format_content(&input, config).unwrap();
    expected.assert_eq(stdx::trim_indent(&actual).as_str());
}

#[test]
fn test_call_args_break_long_line() {
    check_fmt(
        CstFormatConfig::default().with_max_width(80),
        // language=Move
        "
        module 0x1::m {
            fun main() {
                my_function(first_argument, second_argument, third_argument, fourth_argument);
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    my_function(
                        first_argument,
                        second_argument,
                        third_argument,
                        fourth_argument
                    );
                }
            }
        "#]],
    );
}

#[test]
fn test_call_args_break_before_nested_call_args() {
    check_fmt(
        CstFormatConfig::default().with_max_width(70),
        // language=Move
        "
        module 0x1::m {
            fun main() {
                outer_call(first_argument, second_argument, long_call(inner_a, inner_b), fourth_argument, fifth_argument);
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    outer_call(
                        first_argument,
                        second_argument,
                        long_call(inner_a, inner_b),
                        fourth_argument,
                        fifth_argument
                    );
                }
            }
        "#]],
    );
}

#[test]
fn test_call_args_short_stays_one_line() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            fun main() {
                f(a, b);
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    f(a, b);
                }
            }
        "#]],
    );
}

#[test]
fn test_fn_params_break_long_line_1_param() {
    check_fmt(
        CstFormatConfig::default().with_max_width(60),
        // language=Move
        "
        module 0x1::m {
            public entry fun my_long_function_name(framework: &signer) {
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                public entry fun my_long_function_name(
                    framework: &signer
                ) {}
            }
        "#]],
    );
}

#[test]
fn test_fn_params_break_long_line_2_params() {
    check_fmt(
        CstFormatConfig::default().with_max_width(60),
        // language=Move
        "
        module 0x1::m {
            public entry fun my_long_function_name(framework: &signer, amount: u64) {
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                public entry fun my_long_function_name(
                    framework: &signer,
                    amount: u64
                ) {}
            }
        "#]],
    );
}

#[test]
fn test_fn_params_break_dense_header() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        script {
            public entry fun test_register_twice_should_not_fail(framework: &signer) {
            }
        }
        ",
        // language=Move
        expect![[r#"
            script {
                public entry fun test_register_twice_should_not_fail(framework: &signer) {}
            }
        "#]],
    );
}

#[test]
fn test_fn_params_short_stays_one_line() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            fun main(x: u64) {
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main(x: u64) {}
            }
        "#]],
    );
}

#[test]
fn test_fun_ret_type_continuation_line() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        script {
            public(friend) fun create_framework_reserved_account(addr: address): (signer, SignerCapability) {
            }
        }
        ",
        // language=Move
        expect![[r#"
            script {
                public(friend) fun create_framework_reserved_account(
                    addr: address
                ): (signer, SignerCapability) {}
            }
        "#]],
    );
}

#[test]
fn test_fun_ret_type_short_stays_one_line() {
    // Line before colon is short (< 32), so no break even with a long return type.
    check_fmt(
        CstFormatConfig::default().with_max_width(60),
        // language=Move
        "
        module 0x1::m {
            fun main(): (u64, u64, u64, u64) {
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main(): (u64, u64, u64, u64) {}
            }
        "#]],
    );
}

#[test]
fn test_fun_ret_type_break_does_not_split_params() {
    // Return type makes the line too long, but params alone fit — don't split params.
    check_fmt(
        CstFormatConfig::default().with_max_width(80),
        // language=Move
        "
        module 0x1::m {
            public(friend) fun create_account(addr: address): (signer, SignerCapability) {
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                public(friend) fun create_account(
                    addr: address
                ): (signer, SignerCapability) {}
            }
        "#]],
    );
}

#[test]
fn test_fn_params_not_split_after_ret_type_break() {
    // After the return type breaks to a continuation line, the first line fits —
    // don't split params just because the header density exceeds the limit.
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        script {
            public(friend) fun create_framework_reserved_account(addr: address): (signer, SignerCapability) {
            }
        }
        ",
        // language=Move
        expect![[r#"
            script {
                public(friend) fun create_framework_reserved_account(
                    addr: address
                ): (signer, SignerCapability) {}
            }
        "#]],
    );
}

#[test]
fn test_multiple_funs_in_module_required_linebreaks() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            fun a() {} fun b() {}
        }
        ",
        // language=Move
        expect![[r#"
        module 0x1::m {
            fun a() {}

            fun b() {}
        }
        "#]],
    );
}

#[test]
fn test_fun_ret_type_already_on_continuation_line() {
    // Input already has `: RetType` on a continuation line — preserve the layout,
    // don't split params and keep the colon at continuation indent.
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        script {
            public(friend) fun create_framework_reserved_account(addr: address):(signer, SignerCapability) {

            }
        }
        ",
        // language=Move
        expect![[r#"
            script {
                public(friend) fun create_framework_reserved_account(
                    addr: address
                ): (signer, SignerCapability) {}
            }
        "#]],
    );
}

#[test]
fn test_let_binding_break_after_eq() {
    check_fmt(
        CstFormatConfig::default().with_max_width(60),
        // language=Move
        "
        module 0x1::m {
            fun main() {
                let result = some_long_function_name(first_arg, second_arg);
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    let result =
                        some_long_function_name(first_arg, second_arg);
                }
            }
        "#]],
    );
}

#[test]
fn test_spacing_around_operators_and_punctuation() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            fun main() {
                let a=1+2  ;
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    let a = 1 + 2;
                }
            }
        "#]],
    );
}

#[test]
fn test_deref_star_no_extra_space() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            fun main() {
                let s = s + *e1 / *e2;
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    let s = s + *e1 / *e2;
                }
            }
        "#]],
    );
}

#[test]
fn test_let_binding_short_stays_one_line() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            fun main() {
                let x = 1;
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    let x = 1;
                }
            }
        "#]],
    );
}

#[test]
fn test_call_args_collapse_to_one_line() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            fun main() {
                initialize_and_register(framework,
                        1, true);
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    initialize_and_register(framework, 1, true);
                }
            }
        "#]],
    );
}

#[test]
fn test_trailing_comma_removed() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            fun main() {
                my_function(a, b, c,);
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    my_function(a, b, c);
                }
            }
        "#]],
    );
}

#[test]
fn test_indent_normalization() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
                fun main() {
                        let x = 1;
                }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    let x = 1;
                }
            }
        "#]],
    );
}

#[test]
fn test_indent_normalization_script() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        script {
                fun main() {
                            let x = 1;
                }
        }
        ",
        // language=Move
        expect![[r#"
            script {
                fun main() {
                    let x = 1;
                }
            }
        "#]],
    );
}

#[test]
fn test_attribute_before_module() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        #[test_only]
        module 0x1::m {
            fun main() {
            }
        }
        ",
        // language=Move
        expect![[r#"
            #[test_only]
            module 0x1::m {
                fun main() {}
            }
        "#]],
    );
}

#[test]
fn test_comment_indentation() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
                /// doc comment on function
            fun main() {
                    // inline comment
                        let x = 1;
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                /// doc comment on function
                fun main() {
                    // inline comment
                    let x = 1;
                }
            }
        "#]],
    );
}

#[test]
fn test_blank_lines_collapsed() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            fun main() {
                let x = 1;


                let y = 2;



                let z = 3;
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    let x = 1;

                    let y = 2;

                    let z = 3;
                }
            }
        "#]],
    );
}

#[test]
fn test_bin_expr_chain_long_breaks() {
    // 3+ operands AND compact text >= 64 chars → breaks
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            fun main() {
                let x = first_condition || second_condition || third_condition || fourth_condition || fifth_condition;
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    let x = first_condition
                        || second_condition
                        || third_condition
                        || fourth_condition
                        || fifth_condition;
                }
            }
        "#]],
    );
}

#[test]
fn test_bin_expr_chain_two_operands_breaks_when_long() {
    // Long two-operand binary expressions can break.
    check_fmt(
        CstFormatConfig::default().with_max_width(60),
        // language=Move
        "
        module 0x1::m {
            fun main() {
                let x = some_very_long_first_condition || some_very_long_second_condition;
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    let x = some_very_long_first_condition
                        || some_very_long_second_condition;
                }
            }
        "#]],
    );
}

#[test]
fn test_bin_expr_chain_short_text_stays_one_line() {
    // 3+ operands but compact text < 64 chars → no break
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            fun main() {
                let x = a || b || c || d;
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    let x = a || b || c || d;
                }
            }
        "#]],
    );
}

#[test]
fn test_bin_expr_chain_nested_in_call_not_broken() {
    // Inner chain inside a call arg is separate — only the outer chain breaks.
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            fun main() {
                let x = first_condition || some_function(second_condition || third_condition) || fourth_condition || fifth_condition;
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    let x = first_condition
                        || some_function(second_condition || third_condition)
                        || fourth_condition
                        || fifth_condition;
                }
            }
        "#]],
    );
}

#[test]
fn test_bin_expr_chain_and_breaks() {
    // && chains work the same way
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            fun main() {
                let x = first_condition && second_condition && third_condition && fourth_condition && fifth_condition;
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    let x = first_condition
                        && second_condition
                        && third_condition
                        && fourth_condition
                        && fifth_condition;
                }
            }
        "#]],
    );
}

#[test]
fn test_trailing_newline_added() {
    let config = CstFormatConfig::default();
    // language=Move
    let input = "module 0x1::m {}";
    let actual = format_content(input, config).unwrap();
    assert!(
        actual.ends_with('\n'),
        "expected trailing newline, got: {actual:?}"
    );
}

#[test]
fn test_leading_whitespace_removed() {
    let config = CstFormatConfig::default();
    // language=Move
    let input = "\n\n        module 0x1::m {}";
    let actual = format_content(input, config).unwrap();

    assert_eq!(actual, "module 0x1::m {\n}\n");
}

#[test]
fn test_fun_params_not_split_with_misindented_comment() {
    // Doc comment preceding the function shouldn't cause params to be split.
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        script {
            /// create the account for system reserved addresses
            public(friend) fun create_framework_reserved_account(addr: address)
                : (signer, SignerCapability) {
            }
        }
        ",
        // language=Move
        expect![[r#"
            script {
                /// create the account for system reserved addresses
                public(friend) fun create_framework_reserved_account(
                    addr: address
                ): (signer, SignerCapability) {}
            }
        "#]],
    );
}

#[test]
fn test_fun_params_not_split_with_misindented_attr() {
    // Same as above but with an attribute (correctly indented).
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        script {
            #[test]
            public(friend) fun create_framework_reserved_account(addr: address)
                : (signer, SignerCapability) {
            }
        }
        ",
        // language=Move
        expect![[r#"
            script {
                #[test]
                public(friend) fun create_framework_reserved_account(
                    addr: address
                ): (signer, SignerCapability) {}
            }
        "#]],
    );
}

#[test]
fn test_address_literal_not_split_across_lines() {
    // `@0x5` should not be split into `@` on one line and `0x5` on the next.
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            fun main() {
            let x = addr == @
                    0x5;
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    let x = addr == @0x5;
                }
            }
        "#]],
    );
}

#[test]
fn test_bin_expr_trailing_op_moved_to_next_line() {
    // Trailing-operator style (`a ||\n    b`) should be reformatted to
    // leading-operator style (`a\n        || b`).
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            fun main() {
                assert!(
                    addr == @0x1 ||
                        addr == @0x2 ||
                        addr == @0x3 ||
                        addr == @0x4 ||
                        addr == @0x5 ||
                        addr == @0x6,
                    1
                );
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    assert!(
                        addr == @0x1
                            || addr == @0x2
                            || addr == @0x3
                            || addr == @0x4
                            || addr == @0x5
                            || addr == @0x6,
                        1
                    );
                }
            }
        "#]],
    );
}

#[test]
fn test_blank_line_before_closing_brace_removed() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        script {
            fun main() {
            }

        }
        ",
        // language=Move
        expect![[r#"
            script {
                fun main() {}
            }
        "#]],
    );
}

#[test]
fn test_blank_line_added_between_module_items() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            fun foo() {}
            fun bar() {}
            fun baz() {}
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun foo() {}

                fun bar() {}

                fun baz() {}
            }
        "#]],
    );
}

#[test]
fn test_fields_in_structs() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            struct S {val: u8}
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                struct S {
                    val: u8
                }
            }
        "#]],
    );
}

#[test]
fn test_blank_line_added_between_spec_items() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        spec 0x1::m {
            spec native fun spec_len<K, V>(t: Table<K, V>): num;
            spec native fun spec_contains<K, V>(t: Table<K, V>, k: K): bool;
            spec native fun spec_get<K, V>(t: Table<K, V>, k: K): V;
        }
        ",
        // language=Move
        expect![[r#"
            spec 0x1::m {
                spec native fun spec_len<K, V>(t: Table<K, V>): num;

                spec native fun spec_contains<K, V>(t: Table<K, V>, k: K): bool;

                spec native fun spec_get<K, V>(t: Table<K, V>, k: K): V;
            }
        "#]],
    );
}

#[test]
fn test_consecutive_use_stmts_no_blank_line() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            use std::vector;
            use std::string;
            use std::option;
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                use std::vector;
                use std::string;
                use std::option;
            }
        "#]],
    );
}

#[test]
fn test_consecutive_consts_no_blank_line() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            const A: u64 = 1;
            const B: u64 = 2;
            const C: u64 = 3;
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                const A: u64 = 1;
                const B: u64 = 2;
                const C: u64 = 3;
            }
        "#]],
    );
}

#[test]
fn test_consecutive_comments_no_blank_line() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            // comment 1
            // comment 2
            /// doc comment
            fun foo() {}
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                // comment 1
                // comment 2
                /// doc comment
                fun foo() {}
            }
        "#]],
    );
}

#[test]
fn test_block_comment_no_blank_line() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            /* block comment */
            fun foo() {
                /* block comment */
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                /* block comment */
                fun foo() {
                    /* block comment */
                }
            }
        "#]],
    );
}

#[test]
fn test_use_block_then_consts_block() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            use std::vector;
            use std::string;
            const A: u64 = 1;
            const B: u64 = 2;
            fun foo() {}
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                use std::vector;
                use std::string;

                const A: u64 = 1;
                const B: u64 = 2;

                fun foo() {}
            }
        "#]],
    );
}

#[test]
fn test_consecutive_friends_no_blank_line() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            friend 0x1::a;
            friend 0x1::b;
            friend 0x1::c;

            fun foo() {}
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                friend 0x1::a;
                friend 0x1::b;
                friend 0x1::c;

                fun foo() {}
            }
        "#]],
    );
}

#[test]
fn test_fn_params_split_with_trailing_comma() {
    // Param list already multiline with a trailing comma — the whitespace
    // token before `)` is shared with the last comma's following ws,
    // so re-indenting must not produce two conflicting edits on the same token.
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            public entry fun rotate_authentication_key(
                account: &signer,
                from_scheme: u8,
                from_public_key_bytes: vector<u8>,
                to_scheme: u8,
                to_public_key_bytes: vector<u8>,
                cap_rotate_key: vector<u8>,
                cap_update_table: vector<u8>,
            ) {
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                public entry fun rotate_authentication_key(
                    account: &signer,
                    from_scheme: u8,
                    from_public_key_bytes: vector<u8>,
                    to_scheme: u8,
                    to_public_key_bytes: vector<u8>,
                    cap_rotate_key: vector<u8>,
                    cap_update_table: vector<u8>
                ) {}
            }
        "#]],
    );
}

#[test]
fn test_module_doc_comment_not_indented() {
    // Doc comments before `module` are at column 0 and should stay there.
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
/// Module doc comment line 1.
/// Module doc comment line 2.
module 0x1::m {
    fun main() {
    }
}
        ",
        // language=Move
        expect![[r#"
            /// Module doc comment line 1.
            /// Module doc comment line 2.
            module 0x1::m {
                fun main() {}
            }
        "#]],
    );
}

#[test]
fn test_macro_trailing_comma_removed() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            fun main() {
                assert!(condition, ERROR_CODE,);
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    assert!(condition, ERROR_CODE);
                }
            }
        "#]],
    );
}

#[test]
fn test_macro_args_break_long_line() {
    check_fmt(
        CstFormatConfig::default().with_max_width(60),
        // language=Move
        "
        module 0x1::m {
            fun main() {
                assert!(some_long_condition_expression, error::permission_denied(ENOT_AUTHORIZED));
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    assert!(
                        some_long_condition_expression,
                        error::permission_denied(ENOT_AUTHORIZED)
                    );
                }
            }
        "#]],
    );
}

#[test]
fn test_macro_bin_expr_chain_breaks() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            fun main() {
                assert!(addr == @0x1 || addr == @0x2 || addr == @0x3 || addr == @0x4 || addr == @0x5 || addr == @0x6, ENOT_AUTHORIZED);
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    assert!(
                        addr == @0x1
                            || addr == @0x2
                            || addr == @0x3
                            || addr == @0x4
                            || addr == @0x5
                            || addr == @0x6,
                        ENOT_AUTHORIZED
                    );
                }
            }
        "#]],
    );
}

#[test]
fn test_struct_trailing_comma_removed() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            struct Aggregator has store {
                handle: address,
                key: address,
                limit: u128,
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                struct Aggregator has store {
                    handle: address,
                    key: address,
                    limit: u128
                }
            }
        "#]],
    );
}

#[test]
fn test_spec_ensures_implies_should_break_after_implies() {
    check_fmt(
        CstFormatConfig::default().with_max_width(100),
        // language=Move
        "
        module 0x1::m {
            spec shift_left_for_verification_only {
                aborts_if false;
                ensures amount < bitvector.length ==> forall i in bitvector.length - amount..bitvector.length: !bitvector.bit_field[i];
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                spec shift_left_for_verification_only {
                    aborts_if false;
                    ensures amount < bitvector.length
                        ==> forall i in bitvector.length - amount..bitvector.length: !bitvector.bit_field[i];
                }
            }
        "#]],
    );
}

#[test]
fn test_pragma_continuation_lines_indented_started_on_one_line() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        spec aptos_std::table_with_length {
            spec TableWithLength {
                pragma intrinsic = map, map_new = new, map_destroy_empty = destroy_empty, map_len = length, map_has_key = contains;
            }
        }
        ",
        // language=Move
        expect![[r#"
            spec aptos_std::table_with_length {
                spec TableWithLength {
                    pragma intrinsic = map,
                        map_new = new,
                        map_destroy_empty = destroy_empty,
                        map_len = length,
                        map_has_key = contains;
                }
            }
        "#]],
    );
}

#[test]
fn test_pragma_continuation_lines_indented() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        spec aptos_std::table_with_length {
            spec TableWithLength {
                pragma intrinsic = map,
                    map_new = new,
                    map_destroy_empty = destroy_empty,
                    map_len = length,
                    map_has_key = contains;
            }
        }
        ",
        // language=Move
        expect![[r#"
            spec aptos_std::table_with_length {
                spec TableWithLength {
                    pragma intrinsic = map,
                        map_new = new,
                        map_destroy_empty = destroy_empty,
                        map_len = length,
                        map_has_key = contains;
                }
            }
        "#]],
    );
}

#[test]
fn test_spec_native_fun_ret_type_stays_one_line() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        spec aptos_std::table_with_length {
            spec native fun spec_set<K, V>(t: TableWithLength<K, V>, k: K, v: V): TableWithLength<K, V>;
            spec native fun spec_remove<K, V>(t: TableWithLength<K, V>, k: K): TableWithLength<K, V>;
        }
        ",
        // language=Move
        expect![[r#"
            spec aptos_std::table_with_length {
                spec native fun spec_set<K, V>(
                    t: TableWithLength<K, V>,
                    k: K,
                    v: V
                ): TableWithLength<K, V>;

                spec native fun spec_remove<K, V>(
                    t: TableWithLength<K, V>,
                    k: K
                ): TableWithLength<K, V>;
            }
        "#]],
    );
}

#[test]
fn test_empty_struct() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            struct S {}
        }
        ",
        // language=Move
        expect![[r#"
        module 0x1::m {
            struct S {}
        }
        "#]],
    );
}

#[test]
fn test_empty_struct_lit() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            fun main() {
                test_natives_with_type<Droppable>(Droppable{});
            }
        }
        ",
        // language=Move
        expect![[r#"
        module 0x1::m {
            fun main() {
                test_natives_with_type<Droppable>(Droppable {});
            }
        }
        "#]],
    );
}

#[test]
fn test_non_empty_struct_lit() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            fun main() {
                test_natives_with_type<Droppable>(Droppable{val: u8}, Droppable{val: u8});
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    test_natives_with_type<Droppable>(Droppable { val: u8 }, Droppable { val: u8 });
                }
            }
        "#]],
    );
}

#[test]
fn test_non_empty_struct_lit_with_enough_fields_to_break_on_next_line() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            fun main() {
                test_natives_with_type<Droppable>(Droppable{val: u8, val: u8, val: u8, val: u8, val: u8});
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    test_natives_with_type<Droppable>(
                        Droppable { val: u8, val: u8, val: u8, val: u8, val: u8 }
                    );
                }
            }
        "#]],
    );
}

#[test]
fn test_non_empty_struct_lit_with_a_lot_of_fields() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
module 0x1::m {
    fun main() {
        test_natives_with_type<Droppable>(
            Droppable{val: u8, val: u8, val: u8, val: u8, val: u8, val: u8, val: u8, val: u8, val: u8}
        );
    }
}
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    test_natives_with_type<Droppable>(
                        Droppable {
                            val: u8,
                            val: u8,
                            val: u8,
                            val: u8,
                            val: u8,
                            val: u8,
                            val: u8,
                            val: u8,
                            val: u8
                        }
                    );
                }
            }
        "#]],
    );
}

#[test]
fn test_fn_params_not_broken_by_long_body() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            fun main(a: u8) {
                let very_long_variable_name = some_very_long_function_name(first_argument, second_argument);
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main(a: u8) {
                    let very_long_variable_name =
                        some_very_long_function_name(first_argument, second_argument);
                }
            }
        "#]],
    );
}

#[test]
fn test_bin_expr_arithmetic_chain_breaks() {
    // When `let a = expr + expr + expr` is too long, break before each operator
    // with continuation indent, and collapse `=\n` to `= `.
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            fun main() {
                let result = first_long_value + second_long_value + third_long_value + fourth_long_value + fifth_long_value;
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    let result = first_long_value
                        + second_long_value
                        + third_long_value
                        + fourth_long_value
                        + fifth_long_value;
                }
            }
        "#]],
    );
}

#[test]
fn test_bin_expr_arithmetic_no_chain_break_keeps_eq_break() {
    // Bin expr after `=` fits on its own line — chain not broken, `=\n` stays.
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            fun main() {
                let very_long_variable_name_here = some_long_value + another_long_value + third_val;
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    let very_long_variable_name_here =
                        some_long_value + another_long_value + third_val;
                }
            }
        "#]],
    );
}

#[test]
fn test_bin_expr_stmt_do_not_indent_first_one() {
    check_fmt(
        CstFormatConfig::default().with_max_width(10),
        // language=Move
        "
        module 0x1::m {
            fun main() {
                111111111111 + 22222222222;
            }
        }
        ",
        // language=Move
        expect![[r#"
        module 0x1::m {
            fun main() {
                111111111111
                    + 22222222222;
            }
        }
        "#]],
    );
}

#[test]
fn test_line_break_accounts_for_indent() {
    // The whole line exceeds max_width=60, so `=` gets a SpacesOrLineBreak.
    // After breaking at `=`, the call is on its own line at continuation
    // indent (12 chars). 12 + 49 = 61 > 60, so args should also break.
    // Without on-the-fly indentation, line_len_at would see no indent after
    // the `=` break (0 + 49 = 49 <= 60) and incorrectly keep args on one line.
    check_fmt(
        CstFormatConfig::default().with_max_width(60),
        // language=Move
        "
        module 0x1::m {
            fun main() {
                let some_long_variable_name = some_func(first_long_argument, second_long_arggg);
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    let some_long_variable_name =
                        some_func(
                            first_long_argument,
                            second_long_arggg
                        );
                }
            }
        "#]],
    );
}

#[test]
fn test_spec_pragma_binary_string() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        r#"
        module 0x1::m {
            spec module {
                pragma bv=b"0";
            }
        }
        "#,
        // language=Move
        expect![[r#"
            module 0x1::m {
                spec module {
                    pragma bv = b"0";
                }
            }
        "#]],
    );
}

#[test]
fn test_doc_comment_before_module() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        r#"
        /// Module Comment
        module 0x1::m {
        }
        "#,
        // language=Move
        expect![[r#"
            /// Module Comment
            module 0x1::m {
            }
        "#]],
    );
}

#[test]
fn test_indented_module_in_the_middle() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        r#"
            module 0x1::m {
        }
            module 0x1::m2 {
        }
        "#,
        // language=Move
        expect![[r#"
        module 0x1::m {
        }
        module 0x1::m2 {
        }
        "#]],
    );
}

#[test]
fn test_doc_comment_before_module_spec() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        r#"
        /// Comment
        spec 0x1::m {}
        "#,
        // language=Move
        expect![[r#"
            /// Comment
            spec 0x1::m {
            }
        "#]],
    );
}

#[test]
fn test_spec_ensures_implies_breaks_before_arithmetic_operands() {
    check_fmt(
        CstFormatConfig::default().with_max_width(22),
        // language=Move
        "
        module 0x1::m {
            spec module {
                ensures 1 + 2 ==> 2 + 3;
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                spec module {
                    ensures 1 + 2
                        ==> 2 + 3;
                }
            }
        "#]],
    );
}

#[test]
fn test_spec_apply_diff() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
spec apply_diff(features: &mut vector<u8>, enable: vector<u64>, disable: vector<u64>) {
        aborts_if [abstract] false; // TODO(#12011)
        ensures [abstract] forall i in disable: !spec_contains(features, i);
        // TODO(#12011)
        // TODO(#12011)
            // TODO(#12011)
        // TODO(#12011)
        ensures [abstract] forall i in enable: !vector::spec_contains(disable, i) ==> spec_contains(features, i);
            // TODO(#12011)
        pragma opaque;
    }
            }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                spec apply_diff(
                    features: &mut vector<u8>,
                    enable: vector<u64>,
                    disable: vector<u64>
                ) {
                    aborts_if [abstract] false; // TODO(#12011)
                    ensures [abstract] forall i in disable: !spec_contains(features, i);
                    // TODO(#12011)
                    // TODO(#12011)
                    // TODO(#12011)
                    // TODO(#12011)
                    ensures [abstract]
                        forall i in enable: !vector::spec_contains(disable, i)
                            ==> spec_contains(features, i);
                    // TODO(#12011)
                    pragma opaque;
                }
            }
        "#]],
    );
}

#[test]
fn test_spec_requires_property_breaks_before_predicate_expr() {
    check_fmt(
        CstFormatConfig::default().with_max_width(50),
        // language=Move
        "
        module 0x1::m {
            spec module {
                requires [abstract] some_really_long_condition_name;
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                spec module {
                    requires [abstract]
                        some_really_long_condition_name;
                }
            }
        "#]],
    );
}

#[test]
fn test_spec_fun_spec_contains() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
    spec fun spec_contains(features: vector<u8>, feature: u64): bool {
        ((int2bv((((1 as u8) << ((feature % (8 as u64)) as u64)) as u8)) as u8) & features[feature/8] as u8) > (0 as u8)
            && (feature / 8) < len(features)
    }        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                spec fun spec_contains(features: vector<u8>, feature: u64): bool {
                    (
                        (
                            int2bv((((1 as u8) << ((feature % (8 as u64)) as u64)) as u8)) as u8
                        ) & features[feature / 8] as u8
                    ) > (0 as u8)
                        && (feature / 8) < len(features)
                }
            }
        "#]],
    );
}

#[test]
fn test_vector_trim() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
        #[test]
    fun test_trim() {
        {
            let v = V::empty<u64>();
            assert!(&V::trim(&mut v, 0) == &vector[], 0);
        };
    }
    }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                #[test]
                fun test_trim() {
                    {
                        let v = V::empty<u64>();
                        assert!(&V::trim(&mut v, 0) == &vector[], 0);
                    };
                }
            }
        "#]],
    );
}

#[test]
fn test_assignment_indent() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
        #[test]
    fun test_trim() {
        (NotDroppable {}, NotDroppable {}) = test_natives_with_type<NotDroppable>(
            NotDroppable {},
            NotDroppable {}
        );
            }
    }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                #[test]
                fun test_trim() {
                    (NotDroppable {}, NotDroppable {}) =
                        test_natives_with_type<NotDroppable>(NotDroppable {}, NotDroppable {});
                }
            }
        "#]],
    );
}

#[test]
fn test_short_assert() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
        #[test]
    fun short_assert() {
assert!(  s   ==   6 , 0
, )
            }
    }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                #[test]
                fun short_assert() {
                    assert!(s == 6, 0)
                }
            }
        "#]],
    );
}

#[test]
fn test_closure_with_one_line() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
    fun short_assert() {
        v(a, b, |x| 1 + 1);
    }
}
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun short_assert() {
                    v(a, b, |x| 1 + 1);
                }
            }
        "#]],
    );
}

#[test]
fn test_multiple_statements_in_block() {
    check_fmt(
        CstFormatConfig::default().with_max_width(70),
        // language=Move
        r#"
        module 0x1::m {
            fun main() {
                let a = { 1 + 1; 1 + 2; 1 + 3; 1 + 4 };
            }
        }
    "#,
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    let a = {
                        1 + 1;
                        1 + 2;
                        1 + 3;
                        1 + 4
                    };
                }
            }
        "#]],
    )
}

#[test]
fn test_multiple_statements_in_closure_block() {
    check_fmt(
        CstFormatConfig::default().with_max_width(70),
        // language=Move
        r#"
        module 0x1::m {
            fun main() {
                let a = || { 1 + 1; 1 + 2; 1 + 3; 1 + 4 };
            }
        }
    "#,
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    let a = || {
                        1 + 1;
                        1 + 2;
                        1 + 3;
                        1 + 4
                    };
                }
            }
        "#]],
    )
}

#[test]
fn test_multiple_statements_in_closure_block_in_call() {
    check_fmt(
        CstFormatConfig::default().with_max_width(70),
        // language=Move
        r#"
        module 0x1::m {
            fun main() {
                v(|| { 1 + 1; 1 + 2; 1 + 3; 1 + 4 });
            }
        }
    "#,
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    v(|| {
                        1 + 1;
                        1 + 2;
                        1 + 3;
                        1 + 4
                    });
                }
            }
        "#]],
    )
}

#[test]
fn test_multiple_call_expr_indent() {
    check_fmt(
        CstFormatConfig::default().with_max_width(70),
        // language=Move
        r#"
        module 0x1::m {
            fun main() {
                v(11111111111111111, 2222222222222, 33333333333, 44444444, 555555555)
            }
        }
    "#,
        // language=Move
        expect![[r#"
        module 0x1::m {
            fun main() {
                v(
                    11111111111111111,
                    2222222222222,
                    33333333333,
                    44444444,
                    555555555
                )
            }
        }
    "#]],
    )
}

#[test]
fn test_closure_with_two_lines() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
    fun short_assert() {
        v(a, b, |x| { 1 + 1; 1 + 2 });
    }
}
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun short_assert() {
                    v(a, b, |x| {
                        1 + 1;
                        1 + 2
                    });
                }
            }
        "#]],
    );
}

#[test]
fn test_comment_in_block() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m { // hello
    fun short_assert() { // hello
        let a = { // hello
            1     // hello
        }; // hello
    }
}
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                // hello
                fun short_assert() { // hello
                    let a = { // hello
                        1 // hello
                    }; // hello
                }
            }
        "#]],
    );
}

#[test]
fn test_inline_spec_block() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
    fun short_assert() {
        spec { ensures 1 == 1; };
    }
}
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun short_assert() {
                    spec {
                        ensures 1 == 1;
                    };
                }
            }
        "#]],
    );
}

#[test]
fn test_inline_spec_block_in_while_loop_with_block() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
    fun short_assert() {
        while ({spec {
            invariant counter <= length;
            invariant len(bit_field) == counter;
        };
            (counter < length)}) {};
    }
}
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun short_assert() {
                    while ({
                        spec {
                            invariant counter <= length;
                            invariant len(bit_field) == counter;
                        };
                        (counter < length)
                    }) {};
                }
            }
        "#]],
    );
}

#[test]
fn test_remove_trailing_comma_in_struct_lit() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
    fun short_assert() {
        BitVector { length, bit_field, }
    }
}
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun short_assert() {
                    BitVector { length, bit_field }
                }
            }
        "#]],
    );
}

#[test]
fn test_use_groups() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
    use DiemFramework::AccountLimits:: { Self, AccountLimitMutationCapability };
}
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                use DiemFramework::AccountLimits::{Self, AccountLimitMutationCapability};
            }
        "#]],
    );
}

#[test]
fn test_aborts_if_with_errors() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
        spec module { aborts_if exists<AccountFreezing::FreezingBit>(@TreasuryCompliance) with errors::ALREADY_PUBLISHED;
        }
}
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                spec module {
                    aborts_if exists<AccountFreezing::FreezingBit>(@TreasuryCompliance)
                        with errors::ALREADY_PUBLISHED;
                }
            }
        "#]],
    );
}

#[test]
fn test_include_implies_fits() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
        spec module {         include dual_attestation
        ==> DualAttestation::AssertPaymentOkAbortsIf<Token> { value: amount };
        }
}
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                spec module {
                    include dual_attestation
                        ==> DualAttestation::AssertPaymentOkAbortsIf<Token> { value: amount };
                }
            }
        "#]],
    );
}

#[test]
fn test_include_implies_does_not_fit() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
        spec module {         include dual_attestation
        ==> DualAttestation::AssertPaymentOkAbortsIfAssertPaymentOkAbortsIf<Token> { value: amount };
        }
}
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                spec module {
                    include dual_attestation
                        ==> DualAttestation::AssertPaymentOkAbortsIfAssertPaymentOkAbortsIf<Token> {
                                value: amount
                            };
                }
            }
        "#]],
    );
}

#[test]
fn test_long_struct_lit_block_indent() {
    check_fmt(
        CstFormatConfig::default().with_max_width(65),
        // language=Move
        "
        module 0x1::m {
            fun main() {
                let a =
                    SomeVeryLongStructNameThatStillDoesNotFit { value };
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    let a =
                        SomeVeryLongStructNameThatStillDoesNotFit { value };
                }
            }
        "#]],
    );
}

#[test]
fn test_long_struct_lit_block_indent_does_not_fit_with_field() {
    check_fmt(
        CstFormatConfig::default().with_max_width(60),
        // language=Move
        "
        module 0x1::m {
            fun main() {
                let a = SomeVeryLongStructNameThatStillDoesNotFit { value };
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    let a = SomeVeryLongStructNameThatStillDoesNotFit {
                        value
                    };
                }
            }
        "#]],
    );
}

#[test]
fn test_long_struct_lit_block_indent_where_whole_name_does_not_fit() {
    check_fmt(
        CstFormatConfig::default().with_max_width(40),
        // language=Move
        "
        module 0x1::m {
            fun main() {
                let a = SomeVeryLongStructNameThatStillDoesNotFit{
                    value
                };
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    let a =
                        SomeVeryLongStructNameThatStillDoesNotFit {
                            value
                        };
                }
            }
        "#]],
    );
}

#[test]
fn test_const_with_wrong_spaces() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            const    ENOT_ADMIN :   u64  =  0 ;
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                const ENOT_ADMIN: u64 = 0;
            }
        "#]],
    );
}

#[test]
fn test_spaces_after_keywords() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module     0x1::m    {
            friend    0x1::m   ;
            fun     main()    {}
            struct    S    { }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m    {
                friend 0x1::m;

                fun main() {}

                struct S {}
            }
        "#]],
    );
}

#[test]
fn test_references() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            fun main(a: & u16, b: & mut   u64) {
                let a = &   1;
                let a = & mut   1;
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main(a: &u16, b: &mut u64) {
                    let a = &1;
                    let a = &mut 1;
                }
            }
        "#]],
    );
}

#[test]
fn test_mut_global() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            fun main() {
                let tokens = &mut borrow_global_mut<TokenDataCollection<TokenType>>(origin).tokens();
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    let tokens =
                        &mut borrow_global_mut<TokenDataCollection<TokenType>>(origin).tokens();
                }
            }
        "#]],
    );
}

#[test]
fn test_call_expr_with_call_expr_inside() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            fun main() {
                call(origin, origin, origin, origin, origin, origin, origin, origin, option::extract(item_opt))
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    call(
                        origin,
                        origin,
                        origin,
                        origin,
                        origin,
                        origin,
                        origin,
                        origin,
                        option::extract(item_opt)
                    )
                }
            }
        "#]],
    );
}

#[test]
fn test_struct_lit_with_call_expr_inside_tail_expr() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            fun main() {
TokenDataWrapper { origin: owner_addr, index1, metadata: option::extract(
            item_opt
        ) }
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    TokenDataWrapper {
                        origin: owner_addr,
                        index1,
                        metadata: option::extract(item_opt)
                    }
                }
            }
        "#]],
    );
}

#[test]
fn test_struct_lit_with_call_expr_inside_tail_expr_struct_lit_under_width() {
    check_fmt(
        CstFormatConfig::default().with_max_width(90),
        // language=Move
        "
        module 0x1::m {
            fun main() {
                TokenDataWrapper { origin: owner_addr, index, metadata: option::extract(item_opt) }
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    TokenDataWrapper {
                        origin: owner_addr,
                        index,
                        metadata: option::extract(item_opt)
                    }
                }
            }
        "#]],
    );
}

#[test]
fn test_struct_lit_with_call_expr_inside_stmt() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            fun main() {
TokenDataWrapper { origin: owner_addr, index1, metadata: option::extract(
            item_opt
        ) };
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    TokenDataWrapper {
                        origin: owner_addr,
                        index1,
                        metadata: option::extract(item_opt)
                    };
                }
            }
        "#]],
    );
}

#[test]
fn test_fun_returns_tuple_under_max_width() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            fun main() {
            (token,
            Token { id, balance: amount })
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    (token, Token { id, balance: amount })
                }
            }
        "#]],
    );
}

#[test]
fn test_empty_module() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {

        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
            }
        "#]],
    );
}

#[test]
fn test_spec_include_fits_into_one_line() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            spec main {
        include PeerToPeer<Currency> { payer_signer: payer };
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                spec main {
                    include PeerToPeer<Currency> { payer_signer: payer };
                }
            }
        "#]],
    );
}
#[test]
fn test_spec_include_does_not_fit_because_of_comment_show_still_be_one_line() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            spec main {
        include DiemAccount::TransactionChecks { sender: payer_signer }; // properties checked by the prologue.
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                spec main {
                    include DiemAccount::TransactionChecks { sender: payer_signer }; // properties checked by the prologue.
                }
            }
        "#]],
    );
}

#[test]
fn test_spec_include_does_not_fit_because_of_block_comment_1() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            spec main {
        include DiemAccount::TransactionChecks /* properties checked by the prologue. */ { sender: payer_signer };
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                spec main {
                    include DiemAccount::TransactionChecks /* properties checked by the prologue. */
                        { sender: payer_signer };
                }
            }
        "#]],
    );
}

#[test]
fn test_spec_include_does_not_fit_because_of_block_comment_2() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            spec main {
        include DiemAccount::TransactionChecks  { /* properties checked by the prologue. */sender: payer_signer };
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                spec main {
                    include DiemAccount::TransactionChecks { /* properties checked by the prologue. */
                        sender: payer_signer
                    };
                }
            }
        "#]],
    );
}

#[test]
fn test_implies_then_equals_two_indents() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            spec main {
ensures payer_addr != payee
            ==> DiemAccount::balance<Currency>(payer_addr)
            == old(DiemAccount::balance<Currency>(payer_addr)) - amount;            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                spec main {
                    ensures payer_addr != payee
                        ==> DiemAccount::balance<Currency>(payer_addr)
                            == old(DiemAccount::balance<Currency>(payer_addr)) - amount;
                }
            }
        "#]],
    );
}

#[test]
fn test_const_line_break_with_indent() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            const MAX_U8: u8 = 115792089237316195423570985008687907853269984665640564039457584007913129639935;
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                const MAX_U8: u8 =
                    115792089237316195423570985008687907853269984665640564039457584007913129639935;
            }
        "#]],
    );
}

#[test]
fn test_function_modifiers() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            public   public( script ) public( friend )  entry  friend  package   fun main() {}
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                public public(script) public(friend) entry friend package fun main() {}
            }
        "#]],
    );
}

#[test]
fn test_block_after_block() {
    check_fmt(
        CstFormatConfig::default(),
        // language=Move
        "
        module 0x1::m {
            fun main() {
                { let a = 1;  };  { let a = 2; }
            }
        }
        ",
        // language=Move
        expect![[r#"
            module 0x1::m {
                fun main() {
                    {
                        let a = 1;
                    };
                    {
                        let a = 2;
                    }
                }
            }
        "#]],
    );
}
