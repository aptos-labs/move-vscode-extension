// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use expect_test::{Expect, expect};
use test_utils::fixtures;

fn check_highlighting_for_text(source: &str, expect: Expect) {
    let (analysis, file_id) = fixtures::from_single_file(source.to_owned());
    let html_highlights = analysis.highlight_as_html_no_style(file_id).unwrap();
    expect.assert_eq(html_highlights.trim());
}

#[test]
fn test_highlight_items() {
    check_highlighting_for_text(
        // language=Move
        r#"
module 0x1::m {
    const ERR: u8 = 1;
    const ERR_1: u8 = 1;

    fun main() {
        ERR;
        ERR_1;
        assert!();
    }
}
    "#,
        // language=HTML
        expect![[r#"
            <keyword>module</keyword> <numeric_literal>0x1</numeric_literal><operator>::</operator><module>m</module> <brace>{</brace>
                <keyword>const</keyword> <constant>ERR</constant><colon>:</colon> <builtin_type>u8</builtin_type> <operator>=</operator> <numeric_literal>1</numeric_literal><semicolon>;</semicolon>
                <keyword>const</keyword> <constant>ERR_1</constant><colon>:</colon> <builtin_type>u8</builtin_type> <operator>=</operator> <numeric_literal>1</numeric_literal><semicolon>;</semicolon>

                <keyword>fun</keyword> <function>main</function><parenthesis>(</parenthesis><parenthesis>)</parenthesis> <brace>{</brace>
                    <constant>ERR</constant><semicolon>;</semicolon>
                    <constant>ERR_1</constant><semicolon>;</semicolon>
                    <assert>assert</assert><macro_bang>!</macro_bang><parenthesis>(</parenthesis><parenthesis>)</parenthesis><semicolon>;</semicolon>
                <brace>}</brace>
            <brace>}</brace>"#]],
    );
}

#[test]
fn test_highlight_type_param() {
    check_highlighting_for_text(
        // language=Move
        r#"
module 0x1::m {
    native fun main<Element>(
        a: Element
    );
}
    "#,
        // language=HTML
        expect![[r#"
            <keyword>module</keyword> <numeric_literal>0x1</numeric_literal><operator>::</operator><module>m</module> <brace>{</brace>
                <keyword>native</keyword> <keyword>fun</keyword> <function>main</function><angle>&lt;</angle><type_param>Element</type_param><angle>&gt;</angle><parenthesis>(</parenthesis>
                    <variable>a</variable><colon>:</colon> <type_param>Element</type_param>
                <parenthesis>)</parenthesis><semicolon>;</semicolon>
            <brace>}</brace>"#]],
    );
}

#[test]
fn test_highlight_module_spec() {
    check_highlighting_for_text(
        // language=Move
        r#"
module aptos_framework::m {
    fun main() {
        main();
    }
}
spec aptos_framework::m {
    spec fun main(): u8 {
        main(); 1
    }
}
    "#,
        // language=HTML
        expect![[r#"
            <keyword>module</keyword> aptos_framework<operator>::</operator><module>m</module> <brace>{</brace>
                <keyword>fun</keyword> <function>main</function><parenthesis>(</parenthesis><parenthesis>)</parenthesis> <brace>{</brace>
                    <function>main</function><parenthesis>(</parenthesis><parenthesis>)</parenthesis><semicolon>;</semicolon>
                <brace>}</brace>
            <brace>}</brace>
            <keyword>spec</keyword> <unresolved_reference>aptos_framework</unresolved_reference><operator>::</operator><module>m</module> <brace>{</brace>
                <keyword>spec</keyword> <keyword>fun</keyword> <function>main</function><parenthesis>(</parenthesis><parenthesis>)</parenthesis><colon>:</colon> <builtin_type>u8</builtin_type> <brace>{</brace>
                    <function>main</function><parenthesis>(</parenthesis><parenthesis>)</parenthesis><semicolon>;</semicolon> <numeric_literal>1</numeric_literal>
                <brace>}</brace>
            <brace>}</brace>"#]],
    );
}

#[test]
fn test_highlight_literals() {
    check_highlighting_for_text(
        // language=Move
        r#"
module aptos_framework::m {
    fun main() {
        1;
        @0x1;
        true;
        false;
        x"f1f1f1f1";
        b"f1f1f1f1";
        vector[1, 2, 3];
    }
}
    "#,
        // language=HTML
        expect![[r#"
            <keyword>module</keyword> aptos_framework<operator>::</operator><module>m</module> <brace>{</brace>
                <keyword>fun</keyword> <function>main</function><parenthesis>(</parenthesis><parenthesis>)</parenthesis> <brace>{</brace>
                    <numeric_literal>1</numeric_literal><semicolon>;</semicolon>
                    <operator>@</operator><numeric_literal>0x1</numeric_literal><semicolon>;</semicolon>
                    <bool_literal>true</bool_literal><semicolon>;</semicolon>
                    <bool_literal>false</bool_literal><semicolon>;</semicolon>
                    <string_literal>x"f1f1f1f1"</string_literal><semicolon>;</semicolon>
                    <string_literal>b"f1f1f1f1"</string_literal><semicolon>;</semicolon>
                    <vector>vector</vector><bracket>[</bracket><numeric_literal>1</numeric_literal><comma>,</comma> <numeric_literal>2</numeric_literal><comma>,</comma> <numeric_literal>3</numeric_literal><bracket>]</bracket><semicolon>;</semicolon>
                <brace>}</brace>
            <brace>}</brace>"#]],
    );
}

#[test]
fn test_highlight_operators() {
    check_highlighting_for_text(
        // language=Move
        r#"
module aptos_framework::m {
    fun main() {
        1 <= 1;
        1 != 1;
        1 == 1;
        1 >= 1;
        1 >> 1;
        1 << 1;
        1 ==> 1;
        1 <==> 1;
        match (x) { 1 => 2 }
    }
}
    "#,
        // language=HTML
        expect![[r#"
            <keyword>module</keyword> aptos_framework<operator>::</operator><module>m</module> <brace>{</brace>
                <keyword>fun</keyword> <function>main</function><parenthesis>(</parenthesis><parenthesis>)</parenthesis> <brace>{</brace>
                    <numeric_literal>1</numeric_literal> <comparison>&lt;=</comparison> <numeric_literal>1</numeric_literal><semicolon>;</semicolon>
                    <numeric_literal>1</numeric_literal> <comparison>!=</comparison> <numeric_literal>1</numeric_literal><semicolon>;</semicolon>
                    <numeric_literal>1</numeric_literal> <comparison>==</comparison> <numeric_literal>1</numeric_literal><semicolon>;</semicolon>
                    <numeric_literal>1</numeric_literal> <comparison>&gt;=</comparison> <numeric_literal>1</numeric_literal><semicolon>;</semicolon>
                    <numeric_literal>1</numeric_literal> <bitwise>&gt;&gt;</bitwise> <numeric_literal>1</numeric_literal><semicolon>;</semicolon>
                    <numeric_literal>1</numeric_literal> <bitwise>&lt;&lt;</bitwise> <numeric_literal>1</numeric_literal><semicolon>;</semicolon>
                    <numeric_literal>1</numeric_literal> <logical>==&gt;</logical> <numeric_literal>1</numeric_literal><semicolon>;</semicolon>
                    <numeric_literal>1</numeric_literal> <logical>&lt;==&gt;</logical> <numeric_literal>1</numeric_literal><semicolon>;</semicolon>
                    <keyword>match</keyword> <parenthesis>(</parenthesis><unresolved_reference>x</unresolved_reference><parenthesis>)</parenthesis> <brace>{</brace> <numeric_literal>1</numeric_literal> <operator>=&gt;</operator> <numeric_literal>2</numeric_literal> <brace>}</brace>
                <brace>}</brace>
            <brace>}</brace>"#]],
    );
}
