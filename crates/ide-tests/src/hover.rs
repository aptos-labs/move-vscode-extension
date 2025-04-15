use crate::init_tracing_for_test;
use expect_test::expect;
use ide::Analysis;
use ide::test_utils::get_marked_position_offset;
use syntax::files::FilePosition;

pub(crate) fn check_hover(source: &str, expect: expect_test::Expect) {
    init_tracing_for_test();

    let ref_offset = get_marked_position_offset(&source, "//^");

    let (analysis, file_id) = Analysis::from_single_file(source.to_string());
    let position = FilePosition {
        file_id,
        offset: ref_offset,
    };

    let hover_result = analysis
        .hover(position)
        .unwrap()
        .map(|range_info| range_info.info);
    let hover_result = hover_result.expect("None is returned from the generator");

    let doc_string = hover_result.doc_string;
    expect.assert_eq(&doc_string);
}

#[test]
fn test_hover_for_struct() {
    check_hover(
        // language=Move
        r#"
module 0x1::m {
    /// struct docs
    struct S has key { val: u8 }
    fun main() {
        S { val: u8 };
      //^
    }
}
    "#,
        // language=Markdown
        expect![[r#"
            ```move
            0x1::m

            struct S has key { }
            ```
            ---
            struct docs
        "#]],
    )
}

#[test]
fn test_hover_for_enum() {
    check_hover(
        // language=Move
        r#"
module 0x1::m {
    /// enum docs
    enum S has key { One, Two }
    fun main() {
        S::One;
      //^
    }
}
    "#,
        // language=Markdown
        expect![[r#"
            ```move
            0x1::m

            enum S has key { }
            ```
            ---
            enum docs
        "#]],
    )
}

#[test]
fn test_hover_for_function() {
    check_hover(
        // language=Move
        r#"
module 0x1::m {
    /// function docs
    fun main() {
        main();
        //^
    }
}
    "#,
        // language=Markdown
        expect![[r#"
            ```move
            0x1::m

            fun main()
            ```
            ---
            function docs
        "#]],
    )
}

#[test]
fn test_hover_for_struct_field() {
    check_hover(
        // language=Move
        r#"
module 0x1::m {
    /// struct docs
    struct S has key {
        /// my field
        val: u8
    }
    fun main() {
        S { val: u8 };
           //^
    }
}
    "#,
        // language=Markdown
        expect![[r#"
            ```move
            0x1::m::S

            field val: u8
            ```
            ---
            my field
        "#]],
    )
}

#[test]
fn test_hover_for_enum_variant() {
    check_hover(
        // language=Move
        r#"
module 0x1::m {
    /// struct docs
    enum S has key {
        /// my enum variant
        One,
        Two,
    }
    fun main() {
        S::One;
           //^
    }
}
    "#,
        // language=Markdown
        expect![[r#"
            ```move
            0x1::m::S

            variant One
            ```
            ---
            my enum variant
        "#]],
    )
}

#[test]
fn test_hover_for_variable_with_type() {
    check_hover(
        // language=Move
        r#"
module 0x1::m {
    fun main() {
        let my_var: u8 = 1;
        my_var;
        //^
    }
}
    "#,
        // language=Markdown
        expect![[r#"
            ```move

            variable my_var
            ```
            ---

        "#]],
    )
}

#[test]
fn test_hover_for_function_parameter_with_type() {
    check_hover(
        // language=Move
        r#"
module 0x1::m {
    fun main(my_param: u8) {
        my_param;
        //^
    }
}
    "#,
        // language=Markdown
        expect![[r#"
            ```move

            parameter my_param
            ```
            ---

        "#]],
    )
}

#[test]
fn test_hover_for_module_with_inner_comment() {
    check_hover(
        // language=Move
        r#"
/// module docs
module 0x1::m {
          //^
    /// inner string
}
    "#,
        // language=Markdown
        expect![[r#"
            ```move
            0x1

            module m
            ```
            ---
            module docs
        "#]],
    )
}
