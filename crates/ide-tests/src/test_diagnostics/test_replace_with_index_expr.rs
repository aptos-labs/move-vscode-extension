// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::ide_test_utils::diagnostics::{check_diagnostics, check_diagnostics_and_fix};
use expect_test::expect;

#[test]
fn test_no_error_if_argument_types_are_incorrect() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::vector {
            native public fun borrow<Element>(v: &vector<Element>, i: u64): &Element;
        }
        module 0x1::m {
            use 0x1::vector;

            fun main() {
                let v = vector[1, 2];
                *vector::borrow(0, &v);
                              //^ err: Incompatible type 'integer', expected '&vector<Element>'
                                 //^^ err: Incompatible type '&vector<integer>', expected 'u64'
                *vector::borrow(v, 0);
                              //^ err: Incompatible type 'vector<integer>', expected '&vector<Element>'
            }
        }
    "#]]);
}

#[test]
fn test_no_error_vector_address_is_0x2() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x2::vector {
            native public fun borrow<Element>(v: &vector<Element>, i: u64): &Element;
        }
        module 0x1::m {
            use 0x2::vector;

            fun main() {
                let v = vector[1, 2];
                *vector::borrow(&v, 0);
            }
        }
    "#]]);
}

#[test]
fn test_no_error_if_item_is_not_copy() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::vector {
            native public fun borrow<Element>(v: &vector<Element>, i: u64): &Element;
        }
        module 0x1::m {
            use 0x1::vector;

            struct S { field: u8 }

            fun main() {
                let v = vector[S { field: 10 }];
                *vector::borrow(&v, 0);
            }
        }
    "#]]);
}

#[test]
fn test_replace_vector_borrow_deref() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::vector {
                native public fun borrow<Element>(v: &vector<Element>, i: u64): &Element;
            }
            module 0x1::m {
                use 0x1::vector;

                fun main() {
                    let v = vector[1, 2];
                    let vv = &v;
                    *vector::borrow(vv, 0);
                  //^^^^^^^^^^^^^^^^^^^^^^ weak: Can be replaced with index expr
                }
            }
        "#]],
        expect![[r#"
            module 0x1::vector {
                native public fun borrow<Element>(v: &vector<Element>, i: u64): &Element;
            }
            module 0x1::m {
                use 0x1::vector;

                fun main() {
                    let v = vector[1, 2];
                    let vv = &v;
                    vv[0];
                }
            }
        "#]],
    );
}

#[test]
fn test_replace_vector_borrow_deref_with_direct_reference() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::vector {
                native public fun borrow<Element>(v: &vector<Element>, i: u64): &Element;
            }
            module 0x1::m {
                use 0x1::vector;

                fun main() {
                    let v = vector[1, 2];
                    *vector::borrow(&v, 0);
                  //^^^^^^^^^^^^^^^^^^^^^^ weak: Can be replaced with index expr
                }
            }
        "#]],
        expect![[r#"
            module 0x1::vector {
                native public fun borrow<Element>(v: &vector<Element>, i: u64): &Element;
            }
            module 0x1::m {
                use 0x1::vector;

                fun main() {
                    let v = vector[1, 2];
                    v[0];
                }
            }
        "#]],
    );
}

#[test]
fn test_replace_vector_borrow_deref_with_direct_mut_reference() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::vector {
                native public fun borrow<Element>(v: &vector<Element>, i: u64): &Element;
            }
            module 0x1::m {
                use 0x1::vector;

                fun main() {
                    let v = vector[1, 2];
                    *vector::borrow(&mut v, 0);
                  //^^^^^^^^^^^^^^^^^^^^^^^^^^ weak: Can be replaced with index expr
                }
            }
        "#]],
        expect![[r#"
            module 0x1::vector {
                native public fun borrow<Element>(v: &vector<Element>, i: u64): &Element;
            }
            module 0x1::m {
                use 0x1::vector;

                fun main() {
                    let v = vector[1, 2];
                    v[0];
                }
            }
        "#]],
    );
}

#[test]
fn test_replace_vector_borrow_deref_with_dot_expr() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::vector {
                native public fun borrow<Element>(v: &vector<Element>, i: u64): &Element;
            }
            module 0x1::m {
                use 0x1::vector;

                struct S has copy { field: u8 }

                fun main() {
                    let v = vector[S { field: 0 }];
                    (*vector::borrow(&v, 0)).field;
                   //^^^^^^^^^^^^^^^^^^^^^^ weak: Can be replaced with index expr
                }
            }
        "#]],
        expect![[r#"
            module 0x1::vector {
                native public fun borrow<Element>(v: &vector<Element>, i: u64): &Element;
            }
            module 0x1::m {
                use 0x1::vector;

                struct S has copy { field: u8 }

                fun main() {
                    let v = vector[S { field: 0 }];
                    (v[0]).field;
                }
            }
        "#]],
    );
}

#[test]
fn test_replace_vector_borrow_deref_from_method() {
    // language=Move
    check_diagnostics_and_fix(
        expect![[r#"
            module 0x1::vector {
                native public fun borrow<Element>(self: &vector<Element>, i: u64): &Element;
            }
            module 0x1::m {
                fun main() {
                    let v = vector[1];
                    *v.borrow(0);
                  //^^^^^^^^^^^^ weak: Can be replaced with index expr
                }
            }
        "#]],
        expect![[r#"
            module 0x1::vector {
                native public fun borrow<Element>(self: &vector<Element>, i: u64): &Element;
            }
            module 0x1::m {
                fun main() {
                    let v = vector[1];
                    v[0];
                }
            }
        "#]],
    );
}
