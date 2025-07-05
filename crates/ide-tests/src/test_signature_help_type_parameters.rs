use crate::test_signature_help_value_parameters::check_signature_info;
use expect_test::expect;

#[test]
fn test_struct_as_type() {
    // language=Move
    let source = r#"
        module 0x1::m {
            struct S<T> {
                field: T
            }

            fun main(val: S</*caret*/>) {}
        }
            "#;

    check_signature_info(
        source,
        expect![[r#"
        >>T
        //^
    "#]],
    );
}

#[test]
fn test_struct_as_type_with_bounds() {
    // language=Move
    let source = r#"
        module 0x1::m {
            struct S<T: copy> {
                field: T
            }

            fun main(val: S</*caret*/>) {}
        }
            "#;

    check_signature_info(
        source,
        expect![[r#"
        >>T: copy
        //^^^^^^^
    "#]],
    );
}

#[test]
fn test_struct_as_struct_lit() {
    // language=Move
    let source = r#"
        module 0x1::m {
            struct S<T: copy> {
                field: T
            }

            fun main() {
                let a = S</*caret*/> {}
            }
        }
            "#;

    check_signature_info(
        source,
        expect![[r#"
        >>T: copy
        //^^^^^^^
    "#]],
    );
}

#[test]
fn test_generic_fun_no_arguments() {
    // language=Move
    let source = r#"
        module 0x1::m {
            fun call() {}

            fun main() {
                call</*caret*/>()
            }
        }
            "#;

    check_signature_info(source, expect!["<no arguments>"]);
}

#[test]
fn test_generic_fun() {
    // language=Move
    let source = r#"
        module 0x1::m {
            fun call<R: store>() {}

            fun main() {
                call</*caret*/>()
            }
        }
            "#;

    check_signature_info(
        source,
        expect![[r#"
        >>R: store
        //^^^^^^^^
    "#]],
    );
}

#[test]
fn test_aliased_generic_fun() {
    // language=Move
    let source = r#"
        module 0x1::mod {
            public fun call<R: store>() {}
        }
        module 0x1::m {
            use 0x1::mod::call as mycall;
            fun main() {
                mycall</*caret*/>()
            }
        }
            "#;

    check_signature_info(
        source,
        expect![[r#"
        >>R: store
        //^^^^^^^^
    "#]],
    );
}

#[test]
fn test_generic_fun_index_0() {
    // language=Move
    let source = r#"
        module 0x1::m {
            fun call<R: store, S: copy>() {}

            fun main() {
                call<u8/*caret*/, u8>()
            }
        }
            "#;

    check_signature_info(
        source,
        expect![[r#"
        >>R: store, S: copy
        //^^^^^^^^
    "#]],
    );
}

#[test]
fn test_generic_fun_index_1() {
    // language=Move
    let source = r#"
        module 0x1::m {
            fun call<R: store, S: copy>() {}

            fun main() {
                call<u8, u8/*caret*/>()
            }
        }
            "#;

    check_signature_info(
        source,
        expect![[r#"
        >>R: store, S: copy
                  //^^^^^^^
    "#]],
    );
}
