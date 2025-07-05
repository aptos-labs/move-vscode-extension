use crate::ide_test_utils::completion_utils::check_completions;
use expect_test::expect;

#[test]
fn test_exact_name_match_does_before_everything_else() {
    check_completions(
        // language=Move
        r#"
module std::main {
    fun call(exact_match: u8) {}
    fun main() {
        let exact_match: u8 = 1;
        let exact_match_with_suffix: u8 = 2;
        call(exa/*caret*/);
    }
}
    "#,
        expect![[r#"
            [
                "exact_match",
                "exact_match_with_suffix",
            ]"#]],
    );
}
