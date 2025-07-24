use crate::test_inlay_type_hints::check_inlay_hints;
use expect_test::expect;

#[test]
fn test_inlay_parameter_hints_for_literals_on_fun() {
    // language=Move
    check_inlay_hints(expect![[r#"
        module 0x1::m {
            fun call(min_size: u8, mid_size: u8, max_size: u8, limit: u8) { min_size + max_size + limit }
            fun max_size(): u8 { 1 }
            fun main() {
                let limit: u8 = 1;
                call(
                    1,
                  //^ min_size
                    1 + 2,
                  //^^^^^ mid_size
                    max_size(),
                    limit
                );
            }
        }
    "#]]);
}

#[test]
fn test_no_hints_for_tuple_structs_and_assert() {
    // language=Move
    check_inlay_hints(expect![[r#"
        module 0x1::m {
            struct S(u8, u8);
            enum T { One(u8, u8) }
            fun main() {
                S(1, 1);
                T::One(1, 1);
                assert!(true, 1);
            }
        }
    "#]]);
}

#[test]
fn test_inlay_parameter_hints_for_literals_on_method() {
    // language=Move
    check_inlay_hints(expect![[r#"
        module 0x1::m {
            struct S { val: u8 }
            fun receiver(
                self: &S,
                min_size: u8,
                mid_size: u8,
                max_size: u8,
                limit: u8
            ) {
                min_size + max_size + limit
            }
            fun max_size(): u8 { 1 }
            fun main(s: &S) {
                let limit: u8 = 1;
                s.receiver(
                    1,
                  //^ min_size
                    1 + 2,
                  //^^^^^ mid_size
                    max_size(),
                    limit
                );
            }
        }
    "#]]);
}

#[test]
fn test_inlay_parameter_hints_for_literals_on_lambda() {
    // language=Move
    check_inlay_hints(expect![[r#"
        module 0x1::m {
            fun main() {
                let lambda = |a: u8, b: u8| a + b;
                  //^^^^^^ |u8, u8| -> u8
                lambda(
                    1,
                  //^ a
                    1
                  //^ b
                );
            }
        }
    "#]]);
}

#[test]
fn test_no_inlay_parameter_hint_if_only_one_parameter() {
    // language=Move
    check_inlay_hints(expect![[r#"
        module 0x1::m {
            fun call(a: u8) {}
            fun main() {
                call(1);
            }
        }
    "#]]);
}

#[test]
fn test_no_inlay_hint_if_variable_has_the_same_name_as_parameter() {
    // language=Move
    check_inlay_hints(expect![[r#"
        module 0x1::m {
            fun call(limit: u8, max_size: u8) {}
            fun main() {
                let limit = 1;
                  //^^^^^ u8
                let vector_len = 1;
                  //^^^^^^^^^^ u8
                call(limit, vector_len);
                          //^^^^^^^^^^ max_size
            }
        }
    "#]]);
}

#[test]
fn test_no_inlay_hint_if_variable_has_the_same_name_param_with_underscore() {
    // language=Move
    check_inlay_hints(expect![[r#"
        module 0x1::m {
            fun call(_limit: u8, max_size: u8) {}
            fun main() {
                let limit = 1;
                  //^^^^^ u8
                let vector_len = 1;
                  //^^^^^^^^^^ u8
                call(limit, vector_len);
                          //^^^^^^^^^^ max_size
            }
        }
    "#]]);
}

#[test]
fn test_no_inlay_hints_for_builtins() {
    // language=Move
    check_inlay_hints(expect![[r#"
        module 0x1::m {
            fun main(s: &signer) {
                move_to(s, vector[1]);
            }
        }
    "#]]);
}
