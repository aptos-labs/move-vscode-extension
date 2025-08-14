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
