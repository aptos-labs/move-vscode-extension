// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::types::check_expr_type;

// language=Move
#[test]
fn test_type_of_inner_field() {
    check_expr_type(
        r#"
module 0x1::m {
    struct Inner { field: u8 }
    enum Outer { One { inner: Inner } }

    public fun non_exhaustive(o: &Outer) {
        match (o) {
            One { inner } => inner
                              //^ &0x1::m::Inner
        }
    }
}
"#,
    )
}

// language=Move
#[test]
fn test_type_of_deep_inner_field() {
    check_expr_type(
        r#"
module 0x1::m {
    struct Inner { field: u8 }
    enum Outer { One { inner: Inner } }

    public fun non_exhaustive(o: &Outer) {
        match (o) {
            One { inner: Inner { field: myfield } } => myfield
                                                      //^ &u8
        }
    }
}
"#,
    )
}

#[test]
fn test_resolve_builtin_function_in_module_spec() {
    // language=Move
    check_expr_type(
        r#"
spec std::m {
    spec module {
        (len(vector[1, 2])) == 2;
      //^ num
    }
}
    "#,
    );
}

#[test]
fn test_infer_type_of_lambda_parameter() {
    // language=Move
    check_expr_type(
        r#"
module std::vector {
    public inline fun for_each_ref<Element>(self: &vector<Element>, f: |&Element|)  {}
}
module std::m {
    fun main() {
        vector[vector[true]].for_each_ref(|elem| { elem })
                                                   //^ &vector<bool>
    }
}
    "#,
    );
}

#[test]
fn test_tuple_enum_field_type_of_reference() {
    // language=Move
    check_expr_type(
        r#"
module std::m {
    enum StoredPermission has store, copy, drop {
        Unlimited,
        Capacity(u256),
    }
    fun consume_capacity(perm: &mut StoredPermission, threshold: u256): bool {
        match (perm) {
            StoredPermission::Capacity(current_capacity) => {
                current_capacity;
                //^ &mut u256
            }
            StoredPermission::Unlimited => true
        }
    }
}
    "#,
    );
}

#[test]
fn test_type_for_uninitialized_variable_that_inferred_later() {
    // language=Move
    check_expr_type(
        r#"
        module 0x1::m {
            public native fun borrow_mut<Element>(self: Element): &mut Element;
            fun main() {
                let a;
                a = borrow_mut(1u8);
                a;
              //^ &mut u8
            }
        }
    "#,
    );
}

#[test]
fn test_field_for_uninitialized_variable_that_inferred_later() {
    // language=Move
    check_expr_type(
        r#"
        module 0x1::m {
            struct S { val: u16 }
            public native fun borrow_mut<Element>(self: Element): &mut Element;
            fun main() {
                let a;
                a = borrow_mut(S { val: 1 });
                a.val;
                 //^ u16
            }
        }
    "#,
    );
}

#[test]
fn test_infer_include_if_else() {
    // language=Move
    check_expr_type(
        r#"
        module 0x1::m {
            struct XUS {}
            spec schema AddCurrencyAbortsIf<CoinType> {
                dd_addr: address;
            }
            spec schema S<CoinType> {
                    let dd_addr = @0x1;
                    let add_all_currencies = true;
                    include if (add_all_currencies) AddCurrencyAbortsIf<XUS>{dd_addr: dd_addr}
                                                                                     //^ address
                            else AddCurrencyAbortsIf<CoinType>{dd_addr: dd_addr};
            }
        }
    "#,
    );
}

#[test]
fn test_paren_pat() {
    // language=Move
    check_expr_type(
        r#"
        module 0x1::main {
            fun main() {
                let ((((a)))) = 1;
                a;
              //^ integer
            }
        }
    "#,
    )
}

#[test]
fn test_type_of_range_function_integer() {
    // language=Move
    check_expr_type(
        r#"
        module 0x1::main {
            spec module {
                let my_range = range(vector[1, 2]);
                my_range;
               //^ range<num>
            }
        }
    "#,
    )
}

#[test]
fn test_type_of_range_function_bool() {
    // language=Move
    check_expr_type(
        r#"
        module 0x1::main {
            spec module {
                let my_range = range(vector[true]);
                my_range;
               //^ range<bool>
            }
        }
    "#,
    )
}

#[test]
fn test_choose_index_type() {
    // language=Move
    check_expr_type(
        r#"
        module 0x1::main {
            spec module {
                let idx = choose min i in range(vector[0, 1, 2, 3]);
                idx;
               //^ num
            }
        }
    "#,
    )
}

#[test]
fn test_choose_index_type_in_where() {
    // language=Move
    check_expr_type(
        r#"
        module 0x1::main {
            spec module {
                choose min i in range(vector[0, 1, 2, 3]) where i == 0;
                                                              //^ num
            }
        }
    "#,
    )
}
