use ide::{assert_eq_text, Analysis};

fn check_highlighting_for_text(source: &str, target: &str) {
    let (analysis, file_id) = Analysis::from_single_file(source.to_owned());
    let highlights = analysis.highlight_as_html_no_style(file_id).unwrap();

    assert_eq_text!(highlights.trim(), target.trim());
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
        r#"
<span class="keyword">module</span> <span class="numeric_literal">0x1</span>::m {
    <span class="keyword">const</span> <span class="constant">ERR</span>: <span class="builtin_type">u8</span> = <span class="numeric_literal">1</span>;
}
    "#,
    );
}

#[test]
fn test_highlight_type_param() {
    check_highlighting_for_text(
        // language=Move
        r#"
module 0x1::m {
    native fun main<Element>(a: Element);
}
    "#,
        // language=HTML
        r#"
<span class="keyword">module</span> <span class="numeric_literal">0x1</span>::m {
    <span class="keyword">native</span> <span class="keyword">fun</span> <span class="function">main</span>&lt;<span class="type_param">Element</span>&gt;(<span class="variable">a</span>: <span class="type_param">Element</span>);
}
    "#,
    );
}
