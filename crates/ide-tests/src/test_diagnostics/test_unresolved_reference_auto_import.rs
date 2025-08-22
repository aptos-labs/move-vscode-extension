use crate::ide_test_utils::diagnostics::{
    check_diagnostics, check_diagnostics_and_fix, check_diagnostics_on_tmpfs,
    check_diagnostics_on_tmpfs_and_fix,
};
use expect_test::expect;
use test_utils::fixtures;
use test_utils::fixtures::test_state::{named, named_with_deps};

#[test]
fn test_import_unresolved_type() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module std::string {
                struct String { val: u8 }
            }
            module 0x1::main {
                fun main(_s: String) {}
                           //^^^^^^ err: Unresolved reference `String`: cannot resolve
            }
        "#]],
        expect![[r#"
            module std::string {
                struct String { val: u8 }
            }
            module 0x1::main {
                use std::string::String;

                fun main(_s: String) {}
            }
        "#]],
    );
}

#[test]
fn test_import_function() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::M {
                public fun call() {}
            }
            script {
                fun main() {
                    call();
                  //^^^^ err: Unresolved reference `call`: cannot resolve
                }
            }
        "#]],
        expect![[r#"
            module 0x1::M {
                public fun call() {}
            }
            script {
                use 0x1::M::call;

                fun main() {
                    call();
                }
            }
        "#]],
    );
}

#[test]
fn test_import_module() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::Signer {
                public fun address_of() {}
            }
            script {
                fun main() {
                    Signer::address_of();
                  //^^^^^^ err: Unresolved reference `Signer`: cannot resolve
                }
            }
        "#]],
        expect![[r#"
            module 0x1::Signer {
                public fun address_of() {}
            }
            script {
                use 0x1::Signer;

                fun main() {
                    Signer::address_of();
                }
            }
        "#]],
    );
}

#[test]
fn test_no_newline_if_already_exists() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::Signer {
                public fun address_of() {}
            }
            script {

                fun main() {
                    Signer::address_of();
                  //^^^^^^ err: Unresolved reference `Signer`: cannot resolve
                }
            }
        "#]],
        expect![[r#"
            module 0x1::Signer {
                public fun address_of() {}
            }
            script {
                use 0x1::Signer;

                fun main() {
                    Signer::address_of();
                }
            }
        "#]],
    );
}

#[test]
fn test_function_with_struct() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::Module {
                public fun call() {}
            }
            module 0x1::Main {
                struct BTC {}

                fun m() {
                    Module::call();
                  //^^^^^^ err: Unresolved reference `Module`: cannot resolve
                }
            }
        "#]],
        expect![[r#"
            module 0x1::Module {
                public fun call() {}
            }
            module 0x1::Main {
                use 0x1::Module;

                struct BTC {}

                fun m() {
                    Module::call();
                }
            }
        "#]],
    );
}

#[test]
fn test_import_module_from_dependency() {
    let test_state = fixtures::from_multiple_files_on_tmpfs(vec![
        named_with_deps(
            "MyApp",
            // language=TOML
            r#"
[dependencies]
MyDep = { local = "../MyDep" }
"#,
            // language=Move
            r#"
//- main.move
module 0x1::main {
    fun main() {
        let _ = vector::empty();/*caret*/
    }
}
"#,
        ),
        named(
            "MyDep",
            // language=Move
            r#"
//- vector.move
module std::vector {
    public fun empty<T>(): vector<T> { vector[] }
}
    "#,
        ),
    ]);
    // language=Move
    check_diagnostics_on_tmpfs_and_fix(
        test_state,
        expect![[r#"
        module 0x1::main {
            fun main() {
                let _ = vector::empty();/*caret*/
                      //^^^^^^ err: Unresolved reference `vector`: cannot resolve
            }
        }
    "#]],
        expect![[r#"
        module 0x1::main {
            use std::vector;

            fun main() {
                let _ = vector::empty();/*caret*/
            }
    "#]],
    );
}
