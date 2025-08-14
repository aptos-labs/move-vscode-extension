use crate::ide_test_utils::diagnostics::{check_diagnostics, check_diagnostics_and_fix};
use expect_test::expect;

#[test]
fn test_no_error() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct MyItem {}
            struct MyItem2 {}
            public fun call() {}
        }
        module 0x1::M2 {
            use 0x1::M::MyItem;
            use 0x1::M::MyItem2;
            use 0x1::M::call;
            fun main(_arg: MyItem2) {
                let _a: MyItem;
                call();
            }
        }
    "#]]);
}

#[test]
fn test_error_unused_item_import() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct MyItem {}
            struct MyItem2 {}
            public fun call() {}
        }
        module 0x1::M2 {
            use 0x1::M::MyItem;
          //^^^^^^^^^^^^^^^^^^^ warn: Unused use item
            fun main() {}
        }
    "#]]);
}

#[test]
fn test_error_unused_module_import() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct MyItem {}
            struct MyItem2 {}
            public fun call() {}
        }
        module 0x1::M2 {
            use 0x1::M;
          //^^^^^^^^^^^ warn: Unused use item
            fun main() {}
        }
    "#]]);
}

#[test]
fn test_error_unused_import_in_use_group() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct MyItem {}
            struct MyItem2 {}
            public fun call() {}
        }
        module 0x1::M2 {
            use 0x1::M::{MyItem, MyItem2};
                               //^^^^^^^ warn: Unused use item
            fun main(_a: MyItem) {}
        }
    "#]]);
}

#[test]
fn test_no_error_if_module_is_imported_and_use_with_fq() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            public fun call() {}
        }
        module 0x1::M2 {
            use 0x1::M;
            fun main() {
                M::call();
            }
        }
    "#]]);
}

#[test]
fn test_no_error_on_self() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct S {}
            public fun call() {}
        }
        module 0x1::Main {
            use 0x1::M::{Self, S};

            fun main(_a: S) {
                M::call();
            }
        }
    "#]]);
}

#[test]
fn test_duplicate_self_import() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct S {}
            public fun call() {}
        }
        module 0x1::Main {
            use 0x1::M::{Self, Self, S};
                             //^^^^ warn: Unused use item

            fun main(_a: S) {
                M::call();
            }
        }
    "#]]);
}

#[test]
fn test_unused_import_with_unresolved_specks() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::Main {
            use 0x1::M1;
          //^^^^^^^^^^^^ warn: Unused use item
                   //^^ err: Unresolved reference `M1`: cannot resolve
            use 0x1::M1::call;
          //^^^^^^^^^^^^^^^^^^ warn: Unused use item
                   //^^ err: Unresolved reference `M1`: cannot resolve
        }
    "#]]);
}

#[test]
fn test_no_error_if_unresolved_but_used() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::Main {
            use 0x1::M;
                   //^ err: Unresolved reference `M`: cannot resolve
            use 0x1::M::call;
                   //^ err: Unresolved reference `M`: cannot resolve
            fun call() {
                M::call();
            }
            fun main() {
                call();
            }
        }
    "#]]);
}

#[test]
fn test_duplicate_import() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            public fun call() {}
        }
        module 0x1::M2 {
            use 0x1::M::call;
            use 0x1::M::call;
          //^^^^^^^^^^^^^^^^^ warn: Unused use item

            fun main() {
                call();
              //^^^^ err: Unresolved reference `call`: resolved to multiple elements
            }
        }
    "#]]);
}

#[test]
fn test_duplicate_import_with_item_group() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct S {}
            public fun call() {}
        }
        module 0x1::M2 {
            use 0x1::M::{S, call};
            use 0x1::M::call;
          //^^^^^^^^^^^^^^^^^ warn: Unused use item

            fun main(_s: S) {
                call();
              //^^^^ err: Unresolved reference `call`: resolved to multiple elements
            }
        }
    "#]]);
}

#[test]
fn test_unused_self_import() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::Coin {
            struct Coin {}
            public fun get_coin(): Coin { Coin { } }
        }
        module 0x1::Main {
            use 0x1::Coin;
            use 0x1::Coin::Self;
          //^^^^^^^^^^^^^^^^^^^^ warn: Unused use item

            fun call(): Coin::Coin {
                Coin::get_coin()
            }
        }
    "#]]);
}

#[test]
fn test_unused_self_in_group() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::Coin {
            struct Coin {}
            public fun get_coin(): Coin { Coin {} }
        }
        module 0x1::Main {
            use 0x1::Coin;
            use 0x1::Coin::{Self, Coin};
                          //^^^^ warn: Unused use item

            fun call(): Coin {
                Coin::get_coin()
            }
        }
    "#]]);
}

#[test]
fn test_incomplete_alias_considered_absent_for_module() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::coin {
            struct Coin {}
            public fun get_coin(): Coin { Coin {} }
        }
        module 0x1::Main {
            use 0x1::coin::{Self as, Self, Coin};
                                   //^^^^ warn: Unused use item

            fun call(): Coin {
                coin::get_coin()
            }
        }
    "#]]);
}

#[test]
fn test_incomplete_alias_considered_absent_for_item() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::coin {
            struct Coin {}
            public fun get_coin(): Coin { Coin {} }
        }
        module 0x1::Main {
            use 0x1::coin::{Coin as, Coin};
                                   //^^^^ warn: Unused use item

            fun call(): Coin {
                      //^^^^ err: Unresolved reference `Coin`: resolved to multiple elements
            }
        }
    "#]]);
}

#[test]
fn test_no_error_for_self_with_alias_and_no_alias() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::coin {
            struct Coin {}
            public fun get_coin(): Coin { Coin {} }
        }
        module 0x1::Main {
            use 0x1::coin::{Self as my_coin, Self, Coin};

            fun call(): Coin {
                coin::get_coin();
                my_coin::get_coin()
            }
        }
    "#]]);
}

#[test]
fn test_unused_alias_if_another_exists() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::Coin {
            struct Coin {}
        }
        module 0x1::Main {
            use 0x1::Coin::{Coin as MyCoin, Coin as MyCoin};
                                          //^^^^^^^^^^^^^^ warn: Unused use item

            fun call(_c: MyCoin) {}
                       //^^^^^^ err: Unresolved reference `MyCoin`: resolved to multiple elements
        }
    "#]]);
}

#[test]
fn test_empty_item_group() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::Coin {
            struct C {}
        }
        module 0x1::Main {
            use 0x1::Coin::{};
          //^^^^^^^^^^^^^^^^^^ warn: Unused use item
        }
    "#]]);
}

#[test]
fn test_all_items_in_group_unused() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::Coin {
            struct C {}
            struct D {}
        }
        module 0x1::Main {
            use 0x1::Coin::{C, D};
          //^^^^^^^^^^^^^^^^^^^^^^ warn: Unused use item
        }
    "#]]);
}

#[test]
fn test_unused_import_in_module_spec() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::Coin {}
        module 0x1::Main {
        }
        spec 0x1::Main {
            use 0x1::Coin::{};
          //^^^^^^^^^^^^^^^^^^ warn: Unused use item
        }
    "#]]);
}

#[test]
fn test_unused_signer_import() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::signer {}
        module 0x1::main {
            use std::signer;
          //^^^^^^^^^^^^^^^^ warn: Unused use item
            fun call(_a: signer) {}
        }
    "#]]);
}

#[test]
fn test_unused_signer_import_with_self() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::signer {}
        module 0x1::main {
            use std::signer::Self;
          //^^^^^^^^^^^^^^^^^^^^^^ warn: Unused use item
            fun call(_a: signer) {}
        }
    "#]]);
}

#[test]
fn test_unused_vector_import() {
    // language=Move
    check_diagnostics(expect![[r#"
        module std::vector {}
        module 0x1::main {
            use std::vector;
          //^^^^^^^^^^^^^^^^ warn: Unused use item
            fun call(_a: vector<u8>) {}
        }
    "#]]);
}

#[test]
fn test_unused_module_import_type_with_same_name_as_used_item() {
    // language=Move
    check_diagnostics(expect![[r#"
        module std::coin {
            struct coin {}
        }
        module 0x1::main {
            use std::coin;
          //^^^^^^^^^^^^^^ warn: Unused use item
            use std::coin::coin;

            fun call(_coin: coin) {}
        }
    "#]]);
}

#[test]
fn test_no_unused_import_for_type() {
    // language=Move
    check_diagnostics(expect![[r#"
        module std::coin {
            struct coin {}
        }
        module 0x1::main {
            use std::coin::coin;

            fun call(_coin: coin) {}
        }
    "#]]);
}

#[test]
fn test_no_unused_import_for_type_with_same_name_as_module_and_self() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::Coin {
            struct Coin {}
            public fun get_coin(): Coin { Coin { } }
        }
        module 0x1::Main {
            use 0x1::Coin::{Self, Coin};

            fun call(): Coin {
                Coin::get_coin()
            }
        }
    "#]]);
}

#[test]
fn test_unused_main_import_in_presence_of_test_only_usage() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::string {
            public fun call() {}
        }
        module 0x1::main {
            use 0x1::string::call;
          //^^^^^^^^^^^^^^^^^^^^^^ warn: Unused use item
            #[test_only]
            use 0x1::string::call;

            #[test_only]
            fun main() {
                call();
              //^^^^ err: Unresolved reference `call`: resolved to multiple elements
            }
        }
    "#]]);
}

#[test]
fn test_unused_main_import_in_presence_of_test_usage() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::string {
            public fun call() {}
        }
        module 0x1::main {
            use 0x1::string::call;
          //^^^^^^^^^^^^^^^^^^^^^^ warn: Unused use item
            #[test_only]
            use 0x1::string::call;

            #[test]
            fun main() {
                call();
              //^^^^ err: Unresolved reference `call`: resolved to multiple elements
            }
        }
    "#]]);
}

#[test]
fn test_unused_main_import_in_presence_of_unresolved_test_only_usage() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::string {
            public fun call() {}
        }
        module 0x1::main {
            use 0x1::string::call;
          //^^^^^^^^^^^^^^^^^^^^^^ warn: Unused use item
            #[test_only]
            fun main() {
                call();
            }
        }
    "#]]);
}

#[test]
fn test_no_error_with_used_alias() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::string {
            public fun call() {}
        }
        module 0x1::main {
            use 0x1::string::call as mycall;

            fun main() {
                mycall();
            }
        }
    "#]]);
}

#[test]
fn test_no_error_with_used_module_alias() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::string {
            public fun call() {}
        }
        module 0x1::main {
            use 0x1::string::call as mycall;

            fun main() {
                mycall();
            }
        }
    "#]]);
}

#[test]
fn test_error_with_unused_alias() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::string {
            public fun call() {}
        }
        module 0x1::main {
            use 0x1::string::call as mycall;
          //^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ warn: Unused use item
            fun main() {
            }
        }
    "#]]);
}

#[test]
fn test_error_with_unused_module_alias() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::string {
            public fun call() {}
        }
        module 0x1::main {
            use 0x1::string as mystring;
          //^^^^^^^^^^^^^^^^^^^^^^^^^^^^ warn: Unused use item
            fun main() {
            }
        }
    "#]]);
}

#[test]
fn test_error_with_self_module_alias_used_in_type_position() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::string {
            public fun call() {}
        }
        module 0x1::main {
            use 0x1::string::Self as mystring;
          //^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ warn: Unused use item
            fun main(_s: mystring) {
                       //^^^^^^^^ err: Unresolved reference `mystring`: cannot resolve
            }
        }
    "#]]);
}

#[test]
fn test_no_error_with_self_module_alias_in_group() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::string {
            public fun call() {}
        }
        module 0x1::main {
            use 0x1::string::{Self as mystring};

            fun main() {
                mystring::call();
            }
        }
    "#]]);
}

#[test]
fn test_no_unused_import_for_function_return_type() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::string {
            struct String {}
        }
        module 0x1::main {
            use 0x1::string;
            public native fun type_name<T>(): string::String;
        }
    "#]]);
}

#[test]
fn test_unused_top_import_with_local_present() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::string {
            public fun call() {}
        }
        module 0x1::main {
            use 0x1::string;
          //^^^^^^^^^^^^^^^^ warn: Unused use item
            fun main() {
                use 0x1::string;
                string::call();
            }
        }
    "#]]);
}

#[test]
fn test_unused_local_import() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::string {
            public fun call() {}
        }
        module 0x1::main {
            fun main() {
                use 0x1::string;
              //^^^^^^^^^^^^^^^^ warn: Unused use item
            }
        }
    "#]]);
}

#[test]
fn test_no_unused_import_if_used_in_two_local_places() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::string {
            public fun call() {}
        }
        module 0x1::main {
            use 0x1::string;
            fun a() {
                string::call();
            }
            fun b() {
                use 0x1::string;
                string::call();
            }
        }
    "#]]);
}

#[test]
fn test_unused_import_if_imported_locally_test_only() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::string {
            public fun call() {}
        }
        module 0x1::main {
            use 0x1::string;
          //^^^^^^^^^^^^^^^^ warn: Unused use item
            #[test_only]
            fun main() {
                use 0x1::string;
                string::call();
            }
        }
    "#]]);
}

#[test]
fn test_no_unused_import_used_both_in_main_and_test_scopes_expr() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::string {
            struct String {}
            public fun call() {}
        }
        module 0x1::main {
            use 0x1::string;
            fun d() {
                string::call();
            }
            #[test_only]
            fun main() {
                string::call();
            }
        }
    "#]]);
}

#[test]
fn test_no_unused_import_used_both_in_main_and_test_scopes_type() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::string {
            struct String {}
            public fun call() {}
        }
        module 0x1::main {
            use 0x1::string;
            struct S { val: string::String }
            #[test_only]
            fun main() {
                string::call();
            }
        }
    "#]]);
}

#[test]
fn test_no_unused_import_with_dot_expr_with_same_name_as_module() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            #[view]
            public fun call(): bool { true }
        }
        module 0x1::main {
            use 0x1::m;
            public entry fun main() {
                if (m::call()) m::call();
            }
        }
        spec 0x1::main {
            spec main {
                let m = 1;
                m.addr = 1;
                //^^^^ err: Unresolved reference `addr`: cannot resolve
            }
        }
    "#]]);
}

#[test]
fn test_unused_test_only_import() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::string {
            public fun call() {}
        }
        module 0x1::main {
            use 0x1::string::call;
            #[test_only]
            use 0x1::string::call;

            fun main() {
                call();
            }
            #[test]
            fun test_main() {
                call();
              //^^^^ err: Unresolved reference `call`: resolved to multiple elements
            }
        }
    "#]]);
}

#[test]
fn test_error_if_main_import_used_only_in_spec_fun() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::string {
            public fun id(): u128 { 1 }
        }
        module 0x1::m {
            use 0x1::string;
          //^^^^^^^^^^^^^^^^ warn: Unused use item
            spec fun call(): u128 {
                string::id()
            }
        }
    "#]]);
}

#[test]
fn test_import_duplicate_inside_spec() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::string {
            public fun id(): u128 { 1 }
        }
        module 0x1::m {
            use 0x1::string;
          //^^^^^^^^^^^^^^^^ warn: Unused use item
        }
        spec 0x1::m {
            use 0x1::string;
            spec module {
                string::id();
            }
        }
    "#]]);
}

#[test]
fn test_no_error_if_verify_only_and_used_inside_spec_fun() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::string {
            public fun id(): u128 { 1 }
        }
        module 0x1::m {
            #[verify_only]
            use 0x1::string;
            spec fun main(): u128 { string::id() }
         }
    "#]]);
}

#[test]
fn test_no_error_if_verify_only_and_used_inside_module_spec() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::string {
            public fun id(): u128 { 1 }
        }
        module 0x1::m {
            #[verify_only]
            use 0x1::string;
        }
        spec 0x1::m {
            spec module {
                string::id();
            }
        }
    "#]]);
}

#[test]
fn test_error_if_only_used_inside_spec() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::string {
            public fun id(): u128 { 1 }
        }
        module 0x1::m {
            use 0x1::string;
          //^^^^^^^^^^^^^^^^ warn: Unused use item
        }
        spec 0x1::m {
            spec module {
                string::id();
            }
        }
    "#]]);
}
