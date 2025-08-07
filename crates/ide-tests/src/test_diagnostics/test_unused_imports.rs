use crate::ide_test_utils::diagnostics::check_diagnostics;
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
