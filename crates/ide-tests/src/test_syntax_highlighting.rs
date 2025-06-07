use expect_test::{Expect, expect};
use test_utils::fixtures;

fn check_highlighting_for_text(source: &str, expect: Expect) {
    let (analysis, file_id) = fixtures::from_single_file(source.to_owned());
    let html_highlights = analysis.highlight_as_html_no_style(file_id).unwrap();
    expect.assert_eq(html_highlights.trim());
}

#[test]
fn test_highlight_const_with_builtin_type() {
    check_highlighting_for_text(
        // language=Move
        r#"
module 0x1::m {
    const ERR: u8 = 1;
}
    "#,
        // language=HTML
        expect![[r#"
            <span class="keyword">module</span> <span class="numeric_literal">0x1</span>::<span class="module">m</span> {
                <span class="keyword">const</span> <span class="constant">ERR</span>: <span class="builtin_type">u8</span> = <span class="numeric_literal">1</span>;
            }"#]],
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
            <span class="keyword">module</span> <span class="numeric_literal">0x1</span>::<span class="module">m</span> {
                <span class="keyword">native</span> <span class="keyword">fun</span> <span class="function">main</span>&lt;<span class="type_param">Element</span>&gt;(
                    <span class="variable">a</span>: <span class="type_param">Element</span>
                );
            }"#]],
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
            <span class="keyword">module</span> aptos_framework::<span class="module">m</span> {
                <span class="keyword">fun</span> <span class="function">main</span>() {
                    <span class="function">main</span>();
                }
            }
            <span class="keyword">spec</span> <span class="unresolved_reference">aptos_framework</span>::<span class="module">m</span> {
                <span class="keyword">spec</span> <span class="keyword">fun</span> <span class="function">main</span>(): <span class="builtin_type">u8</span> {
                    <span class="function">main</span>(); <span class="numeric_literal">1</span>
                }
            }"#]],
    );
}
