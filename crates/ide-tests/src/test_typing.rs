use ide_db::text_edit::TextEdit;
use syntax::files::FilePosition;
use test_utils::{assert_eq_text, fixtures, get_and_replace_caret, get_and_replace_caret_2};

fn do_type_char(char_typed: char, before: &str) -> Option<String> {
    let (mut before, offset) = get_and_replace_caret_2(before, "/*caret*/");

    let edit = TextEdit::insert(offset, char_typed.to_string());
    edit.apply(&mut before);

    let (analysis, file_id) = fixtures::from_single_file(before.clone());
    let pos = FilePosition { file_id, offset };
    let sc = analysis.on_char_typed(pos, char_typed).unwrap()?;

    let text_edit = sc.source_file_edits.get(&file_id)?;
    text_edit.apply(&mut before);

    Some(before.to_string())
}

fn type_char(char_typed: char, before: &str, after: &str) {
    let actual =
        do_type_char(char_typed, before).unwrap_or_else(|| panic!("typing `{char_typed}` did nothing"));

    assert_eq_text!(after, &actual);
}

fn type_char_noop(char_typed: char, before: &str) {
    let file_change = do_type_char(char_typed, before);
    assert_eq!(file_change, None)
}

// language=Move
#[test]
fn test_generic_right_angle_bracket_for_let_type() {
    type_char(
        '<',
        r#"
module 0x1::m {
    fun main() {
        let a: vector/*caret*/
    }
}
    "#,
        r#"
module 0x1::m {
    fun main() {
        let a: vector<>
    }
}
    "#,
    );
}

// language=Move
#[test]
fn test_generic_right_angle_bracket_for_param_type() {
    type_char(
        '<',
        r#"
module 0x1::m {
    fun main(a: vector/*caret*/) {
    }
}
    "#,
        r#"
module 0x1::m {
    fun main(a: vector<>) {
    }
}
    "#,
    );
}

// language=Move
#[test]
fn test_generic_right_angle_bracket_already_exists() {
    type_char_noop(
        '<',
        r#"
module 0x1::m {
    fun main() {
        let a: vector/*caret*/>
    }
}
    "#,
    );
}

// language=Move
#[test]
fn test_do_not_autoinsert_angle_bracket_in_expr() {
    type_char_noop(
        '<',
        r#"
module 0x1::m {
    fun main() {
        len /*caret*/
    }
}
    "#,
    );
}
