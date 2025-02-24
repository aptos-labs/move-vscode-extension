use ide::test_utils::completion::{
    check_completion_exact, check_completions_contains, check_completions_with_prefix_exact,
    check_no_completions, do_single_completion,
};

#[rustfmt::skip]
#[test]
fn test_module_item_list_completion() {
    check_completion_exact(
        // language=Move
        r#"
module 0x1::m {
    fu/*caret*/
}
    "#,
        vec![
            "fun", "struct", "const", "enum", "use", "spec", "friend",
            "public", "entry", "native", "inline",
        ],
    );
}

#[test]
fn test_top_level_completion_items() {
    check_completion_exact(
        // language=Move
        r#"
mod/*caret*/
    "#,
        vec!["module", "script", "spec"],
    );
}

#[test]
fn test_top_level_module_completion() {
    do_single_completion(
        // language=Move
        r#"
mod/*caret*/
    "#,
        // language=Move
        r#"
module $0
    "#,
    );
}

#[test]
fn test_expr_start_completion() {
    check_completions_contains(
        // language=Move
        r#"
module 0x1::m {
    fun main() {
        i/*caret*/
    }
}
    "#,
        vec!["if", "while", "let", "loop", "match", "for", "true", "false"],
    );
}

#[test]
fn test_no_completions_on_completed_let_keyword() {
    check_no_completions(
        // language=Move
        r#"
module 0x1::m {
    fun main() {
        let/*caret*/
    }
}
    "#,
    );
}

#[test]
fn test_complete_function_item() {
    check_completions_with_prefix_exact(
        // language=Move
        r#"
module 0x1::m {
    fun call() {}
    fun main() {
        ca/*caret*/
    }
}
    "#,
        vec!["call()"],
    );
}

#[test]
fn test_complete_function_item_inserts_parens_zero_params() {
    do_single_completion(
        // language=Move
        r#"
module 0x1::m {
    fun call() {}
    fun main() {
        ca/*caret*/
    }
}
    "#,
        // language=Move
        r#"
module 0x1::m {
    fun call() {}
    fun main() {
        call()$0
    }
}
    "#,
    );
}

#[test]
fn test_complete_function_item_inserts_parens_one_param() {
    do_single_completion(
        // language=Move
        r#"
module 0x1::m {
    fun call(a: u8) {}
    fun main() {
        ca/*caret*/
    }
}
    "#,
        // language=Move
        r#"
module 0x1::m {
    fun call(a: u8) {}
    fun main() {
        call($0)
    }
}
    "#,
    );
}

#[test]
fn test_no_keyword_completion_after_colon_colon_in_expr() {
    check_no_completions(
        // language=Move
        r#"
module 0x1::m {
    fun main() {
        Option::/*caret*/
    }
}
    "#,
    );
}
