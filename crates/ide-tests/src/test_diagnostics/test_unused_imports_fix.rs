// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::ide_test_utils::diagnostics::check_diagnostics_and_fix;
use expect_test::expect;

#[test]
fn test_error_unused_item_import_with_fix() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
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
        "#]],
        expect![[r#"
            module 0x1::M {
                struct MyItem {}
                struct MyItem2 {}
                public fun call() {}
            }
            module 0x1::M2 {
                fun main() {}
            }
        "#]],
    );
}

#[test]
fn test_error_unused_module_import_with_fix() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
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
        "#]],
        expect![[r#"
        module 0x1::M {
            struct MyItem {}
            struct MyItem2 {}
            public fun call() {}
        }
        module 0x1::M2 {
            fun main() {}
        }
    "#]],
    );
}

#[test]
fn test_error_unused_import_in_use_group_with_fix() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
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
        "#]],
        expect![[r#"
        module 0x1::M {
            struct MyItem {}
            struct MyItem2 {}
            public fun call() {}
        }
        module 0x1::M2 {
            use 0x1::M::MyItem;
            fun main(_a: MyItem) {}
        }
    "#]],
    );
}

#[test]
fn test_error_unused_import_in_use_group_and_self_with_fix() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::M {
                struct MyItem {}
                struct MyItem2 {}
                public fun call() {}
            }
            module 0x1::M2 {
                use 0x1::M::{Self, MyItem2};
                                 //^^^^^^^ warn: Unused use item
                fun main(_a: M::MyItem) {}
            }
        "#]],
        expect![[r#"
        module 0x1::M {
            struct MyItem {}
            struct MyItem2 {}
            public fun call() {}
        }
        module 0x1::M2 {
            use 0x1::M;
            fun main(_a: M::MyItem) {}
        }
    "#]],
    );
}

#[test]
fn test_highlight_use_stmt_with_too_broad_scope_with_fix() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::pool {
                public fun create_pool() {}
            }
            module 0x1::main {
                use 0x1::pool;
              //^^^^^^^^^^^^^^ warn: Use item is used only in test scope and should be declared as #[test_only]

                #[test]
                fun main() {
                    pool::create_pool();
                }
            }
        "#]],
        expect![[r#"
        module 0x1::pool {
            public fun create_pool() {}
        }
        module 0x1::main {
            #[test_only]
            use 0x1::pool;

            #[test]
            fun main() {
                pool::create_pool();
            }
        }
    "#]],
    );
}

#[test]
fn test_use_speck_in_group_scope_too_broad_extract_with_attribute() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::pool {
                public fun create_pool() {}
            }
            module 0x1::main {
                use 0x1::pool::{Self, create_pool};
                                    //^^^^^^^^^^^ warn: Use item is used only in test scope and should be declared as #[test_only]

                fun main() {
                    pool::create_pool();
                }
                #[test]
                fun test_main() {
                    create_pool();
                }
            }
        "#]],
        expect![[r#"
            module 0x1::pool {
                public fun create_pool() {}
            }
            module 0x1::main {
                use 0x1::pool;
                #[test_only]
                use 0x1::pool::create_pool;

                fun main() {
                    pool::create_pool();
                }
                #[test]
                fun test_main() {
                    create_pool();
                }
            }
        "#]],
    );
}
