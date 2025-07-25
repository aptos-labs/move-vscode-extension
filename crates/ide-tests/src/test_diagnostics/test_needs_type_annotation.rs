use crate::ide_test_utils::diagnostics::check_diagnostics;
use expect_test::expect;

#[test]
fn test_type_parameter_can_be_inferred_from_mut_vector_ref() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun swap<T>(v: &mut vector<T>) {
                swap(v);
            }
        }
    "#]]);
}

#[test]
fn test_no_error_if_name_is_unresolved_but_type_is_inferrable() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            struct Option<Element> has copy, drop, store {
                vec: vector<Element>
            }
            native fun is_none<Element>(t: &Option<Element>): bool;
            fun main() {
                is_none(unknown_name);
                      //^^^^^^^^^^^^ err: Unresolved reference `unknown_name`: cannot resolve
            }
        }
    "#]]);
}

#[test]
fn test_no_error_if_return_type_is_unresolved_but_type_is_inferrable() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            struct Option<Element> has copy, drop, store {
                vec: vector<Element>
            }
            native fun none<Element>(): Option<Element>;
            fun main() {
                none() == unknown_name;
                        //^^^^^^^^^^^^ err: Unresolved reference `unknown_name`: cannot resolve
            }
        }
    "#]]);
}

#[test]
fn test_no_error_for_spec_struct_field_item_passed() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            struct Option<Element> has copy, drop, store {
                vec: vector<Element>
            }
            native fun is_none<Element>(t: &Option<Element>): bool;
            struct S { aggregator: Option<u8> }
            spec S {
                is_none(aggregator);
            }
        }
    "#]]);
}

#[test]
fn test_no_error_if_schema_params_are_inferrable() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct Token<Type> {}
            spec schema MySchema<Type> {
                token: Token<Type>;
            }
            fun call() {}
            spec call {
                include MySchema { token: Token<u8> {} };
            }
        }
    "#]]);
}

#[test]
fn test_infer_from_return_type() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            struct Coin<CoinType> { val: u8 }
            struct S<X> { coins: Coin<X> }
            struct BTC {}
            fun coin_zero<ZeroCoinType>(): Coin<ZeroCoinType> { Coin<ZeroCoinType> { val: 0 } }
            fun call<CallCoinType>() {
                S<CallCoinType> { coins: coin_zero() };
            }
        }
    "#]]);
}

#[test]
fn test_uninferrable_type_params() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            fun call<R>() {}
            fun m() {
                call();
              //^^^^ err: Could not infer this type. Try adding a type annotation
            }
        }
    "#]]);
}

#[test]
fn test_no_error_if_type_error() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun call<R>(_a: u8, _b: &R) {}
            fun main() {
                call(1, false);
                      //^^^^^ err: Incompatible type 'bool', expected '&R'
            }
        }
    "#]]);
}

#[test]
fn test_no_error_if_fun_has_missing_value_params() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun call<R>(_a: u8, _b: &R) {}
            fun main() {
                call();
                   //^ err: This function takes 2 parameters, but 0 parameters were supplied
            }
        }
    "#]]);
}

#[test]
fn test_no_error_missing_parameters_but_they_do_not_affect_inference() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun call<R>(_a: u8) {}
            fun main() {
                call();
                   //^ err: This function takes 1 parameters, but 0 parameters were supplied
            }
        }
    "#]]);
}

#[test]
fn test_method_type_args_inferrable() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            struct S<T> { field: T }
            fun receiver<T, U>(self: &S<T>, param: U): U {
                param
            }
            fun main(s: S<u8>) {
                let _a = s.receiver(1);
            }
        }
    "#]]);
}

#[test]
fn test_method_type_args_not_inferrable() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            struct S { field: u8 }
            native fun receiver<Z>(self: &S, _param: u8): Z;
            fun main(s: S) {
                let _a = s.receiver(1);
                         //^^^^^^^^ err: Could not infer this type. Try adding a type annotation
            }
        }
    "#]]);
}

#[test]
fn test_no_error_if_inferrable_from_params() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            native public fun to_bytes<MoveValue>(v: &MoveValue): vector<u8>;
            fun main(account_address: address) {}
            spec main(account_address: address) {
                let a = to_bytes(account_address);
            }
        }
    "#]]);
}
