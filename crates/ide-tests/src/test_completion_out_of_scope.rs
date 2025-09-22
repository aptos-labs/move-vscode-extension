use crate::ide_test_utils::completion_utils::{
    check_completions_with_config, do_single_completion_with_config,
};
use expect_test::{Expect, expect};
use ide_completion::config::CompletionConfig;
use ide_db::AllowSnippets;

fn check_out_of_scope_completions(source: &str, expected: Expect) {
    check_completions_with_config(
        CompletionConfig {
            allow_snippets: AllowSnippets::new(true),
            enable_imports_on_the_fly: true,
        },
        source,
        expected,
    )
}

fn do_out_of_scope_completion(before: &str, after: Expect) {
    do_single_completion_with_config(
        CompletionConfig {
            allow_snippets: AllowSnippets::new(true),
            enable_imports_on_the_fly: true,
        },
        before,
        after,
    );
}

#[test]
fn test_module_completion() {
    check_out_of_scope_completions(
        // language=Move
        r#"
module 0x1::string {
    public fun call() {}
}
module 0x1::m {
    fun main() {
        str/*caret*/
    }
}
    "#,
        expect![[r#"
            [
                "string",
            ]"#]],
    );
}

#[test]
fn test_module_completion_with_import_present() {
    check_out_of_scope_completions(
        // language=Move
        r#"
module 0x1::string {
    public fun call() {}
}
module 0x1::m {
    use 0x1::string;
    fun main() {
        str/*caret*/
    }
}
    "#,
        expect![[r#"
            [
                "string",
            ]"#]],
    );
}

#[test]
fn test_do_module_completion() {
    // language=Move
    do_out_of_scope_completion(
        r#"
module 0x1::string {
    public fun call() {}
}
module 0x1::m {
    fun main() {
        str/*caret*/
    }
}
    "#,
        expect![[r#"
            module 0x1::string {
                public fun call() {}
            }
            module 0x1::m {
                use 0x1::string;

                fun main() {
                    string/*caret*/
                }
            }
        "#]],
    );
}

#[test]
fn test_do_struct_completion() {
    // language=Move
    do_out_of_scope_completion(
        r#"
module 0x1::string {
    struct String {}
    public fun call() {}
}
module 0x1::m {
    fun main(s: Str/*caret*/) {
    }
}
    "#,
        expect![[r#"
            module 0x1::string {
                struct String {}
                public fun call() {}
            }
            module 0x1::m {
                use 0x1::string::String;

                fun main(s: String/*caret*/) {
                }
            }
        "#]],
    );
}

#[test]
fn test_do_function_completion() {
    // language=Move
    do_out_of_scope_completion(
        r#"
module 0x1::string {
    public fun call() {}
}
module 0x1::m {
    fun main() {
        ca/*caret*/
    }
}
    "#,
        expect![[r#"
            module 0x1::string {
                public fun call() {}
            }
            module 0x1::m {
                use 0x1::string::call;

                fun main() {
                    call()/*caret*/
                }
            }
        "#]],
    );
}

#[test]
fn test_do_function_completion_with_test_only_in_test_scope() {
    // language=Move
    do_out_of_scope_completion(
        r#"
module 0x1::string {
    public fun call() {}
}
module 0x1::m {
    #[test]
    fun test_main() {
        ca/*caret*/
    }
}
    "#,
        expect![[r#"
            module 0x1::string {
                public fun call() {}
            }
            module 0x1::m {
                #[test_only]
                use 0x1::string::call;

                #[test]
                fun test_main() {
                    call()/*caret*/
                }
            }
        "#]],
    );
}
