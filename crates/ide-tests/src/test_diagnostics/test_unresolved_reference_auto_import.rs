use crate::ide_test_utils::diagnostics::{
    check_diagnostics_and_fix, check_diagnostics_and_fix_with_id, check_diagnostics_no_fix,
    check_diagnostics_on_tmpfs_and_fix,
};
use expect_test::{Expect, expect};
use ide_db::assists::AssistId;
use test_utils::fixtures;
use test_utils::fixtures::test_state::{named, named_with_deps};

fn check_diagnostics_apply_import_fix(before: Expect, after: Expect) {
    check_diagnostics_and_fix_with_id(AssistId::quick_fix("add-import"), before, after);
}

fn check_diagnostics_no_import_fix(before: Expect) {
    check_diagnostics_no_fix(AssistId::quick_fix("add-import"), before);
}

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
            }
        "#]],
    );
}

#[test]
fn test_merge_new_auto_import_with_the_existing_group_1() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::M {
                struct S {}
                public fun call() {}
            }
            module 0x1::Main {
                use 0x1::M::S;

                fun main(_s: S) {
                    call();
                  //^^^^ err: Unresolved reference `call`: cannot resolve
                }
            }
        "#]],
        expect![[r#"
            module 0x1::M {
                struct S {}
                public fun call() {}
            }
            module 0x1::Main {
                use 0x1::M::{S, call};

                fun main(_s: S) {
                    call();
                }
            }
        "#]],
    );
}

#[test]
fn test_merge_new_auto_import_with_the_existing_group_2() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::M {
                struct S {}
                public fun call() {}
            }
            module 0x1::Main {
                use 0x1::M::{S};

                fun main(_s: S) {
                    call();
                  //^^^^ err: Unresolved reference `call`: cannot resolve
                }
            }
        "#]],
        expect![[r#"
            module 0x1::M {
                struct S {}
                public fun call() {}
            }
            module 0x1::Main {
                use 0x1::M::{S, call};

                fun main(_s: S) {
                    call();
                }
            }
        "#]],
    );
}

#[test]
fn test_merge_new_auto_import_with_the_existing_group_3() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::M {
                struct S {}
                public fun call() {}
            }
            module 0x1::Main {
                use 0x1::M;

                fun main(_s: M::S) {
                    call();
                  //^^^^ err: Unresolved reference `call`: cannot resolve
                }
            }
        "#]],
        expect![[r#"
            module 0x1::M {
                struct S {}
                public fun call() {}
            }
            module 0x1::Main {
                use 0x1::M::{Self, call};

                fun main(_s: M::S) {
                    call();
                }
            }
        "#]],
    );
}

#[test]
fn test_merge_new_auto_import_with_the_existing_group_with_alias() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::M {
                struct S {}
                public fun call() {}
            }
            module 0x1::Main {
                use 0x1::M::S as MyS;

                fun main(_s: MyS) {
                    call();
                  //^^^^ err: Unresolved reference `call`: cannot resolve
                }
            }
        "#]],
        expect![[r#"
            module 0x1::M {
                struct S {}
                public fun call() {}
            }
            module 0x1::Main {
                use 0x1::M::{S as MyS, call};

                fun main(_s: MyS) {
                    call();
                }
            }
        "#]],
    );
}

#[test]
fn test_no_struct_in_module_context() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::Token {
                struct Token {}
                struct MintCapability {}
                public fun call() {}
            }
            module 0x1::Main {
                fun main(_a: Token::MintCapability) {}
                           //^^^^^ err: Unresolved reference `Token`: cannot resolve
            }
        "#]],
        expect![[r#"
            module 0x1::Token {
                struct Token {}
                struct MintCapability {}
                public fun call() {}
            }
            module 0x1::Main {
                use 0x1::Token;

                fun main(_a: Token::MintCapability) {}
            }
        "#]],
    );
}

#[test]
fn test_struct_with_the_same_name_as_module() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::Token {
                struct Token {}
                public fun call() {}
            }
            module 0x1::Main {
                use 0x1::Token;

                fun main(_a: Token) {
                           //^^^^^ err: Unresolved reference `Token`: cannot resolve
                    Token::call();
                }
            }
        "#]],
        expect![[r#"
            module 0x1::Token {
                struct Token {}
                public fun call() {}
            }
            module 0x1::Main {
                use 0x1::Token::{Self, Token};

                fun main(_a: Token) {
                    Token::call();
                }
            }
        "#]],
    );
}

#[test]
fn test_unresolved_function_on_module_should_not_have_a_fix() {
    // language=Move
    check_diagnostics_no_import_fix(expect![[r#"
        module 0x1::Coin {
            public fun initialize() {}
        }
        module 0x1::AnotherCoin {}
        module 0x1::Main {
            use 0x1::AnotherCoin;

            fun call() {
                AnotherCoin::initialize();
                           //^^^^^^^^^^ err: Unresolved reference `initialize`: cannot resolve
            }
        }
    "#]]);
}

#[test]
fn test_test_only_function_inside_test_only_module() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::Minter {
                #[test_only]
                public fun get_weekly_emission(): u64 { 0 }
            }
            #[test_only]
            module 0x1::MinterTests {
                #[test]
                fun test_a() {
                    get_weekly_emission();
                  //^^^^^^^^^^^^^^^^^^^ err: Unresolved reference `get_weekly_emission`: cannot resolve
                }
            }
        "#]],
        expect![[r#"
            module 0x1::Minter {
                #[test_only]
                public fun get_weekly_emission(): u64 { 0 }
            }
            #[test_only]
            module 0x1::MinterTests {
                use 0x1::Minter::get_weekly_emission;

                #[test]
                fun test_a() {
                    get_weekly_emission();
                }
            }
        "#]],
    );
}

#[test]
fn test_test_only_function_in_test_with_test_only_import() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::Minter {
                #[test_only]
                public fun get_weekly_emission(): u64 { 0 }
            }
            module 0x1::MinterTests {
                #[test]
                fun my_fun() {
                    get_weekly_emission();
                  //^^^^^^^^^^^^^^^^^^^ err: Unresolved reference `get_weekly_emission`: cannot resolve
                }
            }
        "#]],
        expect![[r#"
            module 0x1::Minter {
                #[test_only]
                public fun get_weekly_emission(): u64 { 0 }
            }
            module 0x1::MinterTests {
                #[test_only]
                use 0x1::Minter::get_weekly_emission;

                #[test]
                fun my_fun() {
                    get_weekly_emission();
                }
            }
        "#]],
    );
}

#[test]
fn test_add_non_test_only_import_with_test_only_existing() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::minter {
                public fun mint() {}
            }
            module 0x1::main {
                #[test_only]
                use 0x1::minter::mint;

                public fun main() {
                    mint();
                  //^^^^ err: Unresolved reference `mint`: cannot resolve (note: item defined as `#[test_only]` and cannot be used here)
                }

                #[test_only]
                public fun main_test() {
                    mint();
                }
            }
        "#]],
        expect![[r#"
            module 0x1::minter {
                public fun mint() {}
            }
            module 0x1::main {
                #[test_only]
                use 0x1::minter::mint;
                use 0x1::minter::mint;

                public fun main() {
                    mint();
                }

                #[test_only]
                public fun main_test() {
                    mint();
                }
            }
        "#]],
    );
}

#[test]
fn test_add_non_test_only_import_with_test_only_group_existing() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::minter {
                struct S {}
                public fun mint() {}
            }
            module 0x1::main {
                #[test_only]
                use 0x1::minter::{Self, mint};

                public fun main() {
                    mint();
                  //^^^^ err: Unresolved reference `mint`: cannot resolve (note: item defined as `#[test_only]` and cannot be used here)
                }

                #[test_only]
                public fun main_test(_s: minter::S) {
                    mint();
                }
            }
        "#]],
        expect![[r#"
            module 0x1::minter {
                struct S {}
                public fun mint() {}
            }
            module 0x1::main {
                #[test_only]
                use 0x1::minter::{Self, mint};
                use 0x1::minter::mint;

                public fun main() {
                    mint();
                }

                #[test_only]
                public fun main_test(_s: minter::S) {
                    mint();
                }
            }
        "#]],
    );
}

#[test]
fn test_auto_import_test_function() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::m1 {
                #[test]
                public fun test_a() {}
            }
            module 0x1::m2 {
                #[test_only]
                fun main() {
                    test_a();
                  //^^^^^^ err: Unresolved reference `test_a`: cannot resolve
                }
            }
        "#]],
        expect![[r#"
            module 0x1::m1 {
                #[test]
                public fun test_a() {}
            }
            module 0x1::m2 {
                #[test_only]
                use 0x1::m1::test_a;

                #[test_only]
                fun main() {
                    test_a();
                }
            }
        "#]],
    );
}

#[test]
fn test_add_import_into_existing_empty_group() {
    // language=Move
    check_diagnostics_apply_import_fix(
        expect![[r#"
            module 0x1::Token {
                struct Token {}
                struct MintCapability {}
                public fun call() {}
            }
            module 0x1::Main {
                use 0x1::Token::{};
              //^^^^^^^^^^^^^^^^^^^ warn: Unused use item
                fun main() {
                    call();
                  //^^^^ err: Unresolved reference `call`: cannot resolve
                }
            }
        "#]],
        expect![[r#"
            module 0x1::Token {
                struct Token {}
                struct MintCapability {}
                public fun call() {}
            }
            module 0x1::Main {
                use 0x1::Token::{call};
                fun main() {
                    call();
                }
            }
        "#]],
    );
}

#[test]
fn test_add_import_into_existing_empty_group_with_verify_only_stmt_present() {
    // language=Move
    check_diagnostics_apply_import_fix(
        expect![[r#"
            module 0x1::a {
                struct String {}
                public fun test_call() {}
            }
            module 0x1::m {
                struct S {}
            }
            module 0x1::main {
                use 0x1::a::String;
                #[verify_only]
                use 0x1::a::test_call;

                fun main(_a: String, _b: S) {
                                       //^ err: Unresolved reference `S`: cannot resolve
                }

                #[verify_only]
                fun test() {
                    test_call();
                }
            }
        "#]],
        expect![[r#"
            module 0x1::a {
                struct String {}
                public fun test_call() {}
            }
            module 0x1::m {
                struct S {}
            }
            module 0x1::main {
                use 0x1::a::String;
                #[verify_only]
                use 0x1::a::test_call;
                use 0x1::m::S;

                fun main(_a: String, _b: S) {
                }

                #[verify_only]
                fun test() {
                    test_call();
                }
            }
        "#]],
    );
}

#[test]
fn test_add_import_into_existing_empty_group_with_verify_only_stmt_present_for_the_same_module() {
    // language=Move
    check_diagnostics_apply_import_fix(
        expect![[r#"
            module 0x1::a {
                struct S {}
                public fun test_call() {}
            }
            module 0x1::main {
                #[verify_only]
                use 0x1::a::test_call;

                fun main(_b: S) {
                           //^ err: Unresolved reference `S`: cannot resolve
                }

                #[verify_only]
                fun test() {
                    test_call();
                }
            }
        "#]],
        expect![[r#"
            module 0x1::a {
                struct S {}
                public fun test_call() {}
            }
            module 0x1::main {
                #[verify_only]
                use 0x1::a::test_call;
                use 0x1::a::S;

                fun main(_b: S) {
                }

                #[verify_only]
                fun test() {
                    test_call();
                }
            }
        "#]],
    );
}

#[test]
fn test_auto_import_in_spec() {
    // language=Move
    check_diagnostics_apply_import_fix(
        expect![[r#"
            module 0x1::bcs {
                native public fun to_bytes<MoveValue>(v: &MoveValue): vector<u8>;
            }
            module 0x1::m {
            }
            spec 0x1::m {
                spec module {
                    to_bytes();
                  //^^^^^^^^ err: Unresolved reference `to_bytes`: cannot resolve
                }
            }
        "#]],
        expect![[r#"
            module 0x1::bcs {
                native public fun to_bytes<MoveValue>(v: &MoveValue): vector<u8>;
            }
            module 0x1::m {
            }
            spec 0x1::m {
                use 0x1::bcs::to_bytes;

                spec module {
                    to_bytes();
                }
            }
        "#]],
    );
}

#[test]
fn test_auto_import_test_scope() {
    // language=Move
    check_diagnostics_apply_import_fix(
        expect![[r#"
            module 0x1::bcs {
                native public fun to_bytes<MoveValue>(v: &MoveValue): vector<u8>;
            }
            module 0x1::m {
                #[test]
                fun test_main() {
                    to_bytes();
                  //^^^^^^^^ err: Unresolved reference `to_bytes`: cannot resolve
                }
            }
        "#]],
        expect![[r#"
            module 0x1::bcs {
                native public fun to_bytes<MoveValue>(v: &MoveValue): vector<u8>;
            }
            module 0x1::m {
                #[test_only]
                use 0x1::bcs::to_bytes;

                #[test]
                fun test_main() {
                    to_bytes();
                }
            }
        "#]],
    );
}
