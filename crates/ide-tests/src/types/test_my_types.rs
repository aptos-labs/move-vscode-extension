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
