use crate::resolve::assert_resolves_to_multiple_targets;
use syntax::files::FilePosition;
use test_utils::{fixtures, get_marked_position_offset};

#[track_caller]
fn check_goto_spec(source: &str) {
    let offset = get_marked_position_offset(source, "//^");
    let (analysis, file_id) = fixtures::from_single_file(source);
    let pos = FilePosition { file_id, offset };
    let nav_items = analysis
        .goto_specification(pos)
        .unwrap()
        .expect("missing specifications")
        .info;

    assert_resolves_to_multiple_targets(&analysis, nav_items, (file_id, source.to_string()));
}

#[test]
fn test_goto_fun_spec() {
    // language=Move
    check_goto_spec(
        r#"
module std::m {
    fun main() {}
       //^
}
spec std::m {
    spec main {
        //X
        ensures 1 == 1;
    }
}
    "#,
    );
}
