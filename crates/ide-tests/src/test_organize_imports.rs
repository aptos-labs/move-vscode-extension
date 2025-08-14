use crate::ide_test_utils::diagnostics::apply_assist;
use expect_test::{Expect, expect};
use test_utils::fixtures;
use test_utils::tracing::init_tracing_for_test;

fn check_organize_imports(source: &str, expect: Expect) {
    init_tracing_for_test();

    let source = stdx::trim_indent(source);
    let (analysis, file_id) = fixtures::from_single_file(source.to_string());

    let organize_assist = analysis
        .organize_imports(file_id)
        .unwrap()
        .expect("no assist found");
    let after = apply_assist(&organize_assist, &source);

    expect.assert_eq(&after);
}

#[test]
fn test_remove_unused_type_import() {
    // language=Move
    check_organize_imports(
        r#"
    module 0x1::m {
        struct MyStruct {}
        public fun call() {}
    }
    module 0x1::main {
        use 0x1::m::MyStruct;
        use 0x1::m::call;

        fun main() {
            let a = call();
        }
    }
    "#,
        expect![[r#"
        module 0x1::m {
            struct MyStruct {}
            public fun call() {}
        }
        module 0x1::main {
            use 0x1::m::call;

            fun main() {
                let a = call();
            }
        }
    "#]],
    )
}

#[test]
fn test_remove_unused_import_from_group_in_the_middle() {
    // language=Move
    check_organize_imports(
        r#"
        module 0x1::M {
            struct MyStruct {}
            public fun call() {}
            public fun aaa() {}
        }
        script {
            use 0x1::M::{aaa, MyStruct, call};

            fun main() {
                let a = call();
                let a = aaa();
            }
        }
    "#,
        expect![[r#"
        module 0x1::M {
            struct MyStruct {}
            public fun call() {}
            public fun aaa() {}
        }
        script {
            use 0x1::M::{aaa, call};

            fun main() {
                let a = call();
                let a = aaa();
            }
        }
    "#]],
    )
}

#[test]
fn test_remove_unused_import_from_group_in_the_beginning() {
    // language=Move
    check_organize_imports(
        r#"
        module 0x1::M {
            struct Bbb {}
            public fun call() {}
            public fun aaa() {}
        }
        script {
            use 0x1::M::{aaa, Bbb, call};

            fun main() {
                let a: Bbb = call();
            }
        }
    "#,
        expect![[r#"
        module 0x1::M {
            struct Bbb {}
            public fun call() {}
            public fun aaa() {}
        }
        script {
            use 0x1::M::{Bbb, call};

            fun main() {
                let a: Bbb = call();
            }
        }
    "#]],
    )
}

#[test]
fn test_remove_unused_import_from_group_in_the_end() {
    // language=Move
    check_organize_imports(
        r#"
        module 0x1::M {
            struct Bbb {}
            public fun call() {}
            public fun aaa() {}
        }
        script {
            use 0x1::M::{aaa, Bbb, call};

            fun main() {
                let a: Bbb = aaa();
            }
        }
    "#,
        expect![[r#"
        module 0x1::M {
            struct Bbb {}
            public fun call() {}
            public fun aaa() {}
        }
        script {
            use 0x1::M::{aaa, Bbb};

            fun main() {
                let a: Bbb = aaa();
            }
        }
    "#]],
    )
}

#[test]
fn test_remove_redundant_group_curly_braces() {
    // language=Move
    check_organize_imports(
        r#"
        module 0x1::M {
            struct MyStruct {}
            public fun call() {}
        }
        script {
            use 0x1::M::{call};

            fun main() {
                let a = call();
            }
        }
    "#,
        expect![[r#"
            module 0x1::M {
                struct MyStruct {}
                public fun call() {}
            }
            script {
                use 0x1::M::call;

                fun main() {
                    let a = call();
                }
            }
        "#]],
    )
}

#[test]
fn test_remove_redundant_group_curly_braces_with_self() {
    // language=Move
    check_organize_imports(
        r#"
        module 0x1::M {
            struct MyStruct {}
            public fun call() {}
        }
        script {
            use 0x1::M::{Self};

            fun main() {
                let a = M::call();
            }
        }
    "#,
        expect![[r#"
            module 0x1::M {
                struct MyStruct {}
                public fun call() {}
            }
            script {
                use 0x1::M;

                fun main() {
                    let a = M::call();
                }
            }
        "#]],
    )
}
