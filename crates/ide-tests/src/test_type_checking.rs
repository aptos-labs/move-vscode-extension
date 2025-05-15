use crate::test_utils::diagnostics::check_diagnostics;
use expect_test::expect;

#[test]
fn test_incorrect_type_address_passed_when_signer_is_expected() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun send(account: &signer) {}
            fun main(addr: address) {
                send(addr);
                   //^^^^ err: Incompatible type 'address', expected '&signer'
            }
        }
    "#]]);
}

#[test]
fn test_incorrect_type_u8_passed_where_signer_is_expected() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun send(account: &signer) {}
            fun main(addr: u8) {
                send(addr)
                   //^^^^ err: Incompatible type 'u8', expected '&signer'
            }
        }
    "#]]);
}

#[test]
fn test_no_errors_if_the_same_type() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun send(account: &signer) {}
            fun main(acc: &signer) {
                send(acc)
            }
        }
    "#]]);
}

#[test]
fn test_mutable_reference_compatible_with_immutable_one() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            struct Option<Element> {
                vec: vector<Element>
            }
            fun is_none<Element>(t: &Option<Element>): bool {
                true
            }
            fun main<Element>(opt: &mut Option<Element>) {
                is_none(opt);
            }
        }
    "#]]);
}

#[test]
fn test_same_struct_but_different_generic_types() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct Option<Element> {}
            fun is_none<Elem>(t: Option<u64>): bool {
                true
            }
            fun main() {
                let opt = Option<u8> {};
                is_none(opt);
                      //^^^ err: Incompatible type '0x1::M::Option<u8>', expected '0x1::M::Option<u64>'
            }
        }
    "#]]);
}

#[test]
fn test_different_generic_types_for_references() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct Option<Element> {}
            fun is_none<Elem>(t: &Option<u64>): bool {
                true
            }
            fun main() {
                let opt = &Option<u8> {};
                is_none(opt);
                      //^^^ err: Incompatible type '&0x1::M::Option<u8>', expected '&0x1::M::Option<u64>'
            }
        }
    "#]]);
}

#[test]
fn test_immutable_reference_not_compatible_with_mutable_reference() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct Option<OptElement> {
                vec: vector<OptElement>
            }
            fun is_none<NoneElement>(t: &mut Option<NoneElement>): bool {
                true
            }
            fun main<Element>(opt: &Option<Element>) {
                is_none(opt);
                      //^^^ err: Incompatible type '&0x1::M::Option<Element>', expected '&mut 0x1::M::Option<Element>'
            }
        } 
    "#]]);
}

#[test]
fn test_incorrect_type_of_argument_with_struct_literal() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct A {}
            struct B {}

            fun use_a(a: A) {}
            fun main() {
                use_a(B {});
                    //^^^^ err: Incompatible type '0x1::M::B', expected '0x1::M::A'
            }
        }
    "#]]);
}

#[test]
fn test_incorrect_type_of_argument_with_call_expr() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct A {}
            struct B {}

            fun use_a(a: A) {}
            fun get_b(): B { B {} }

            fun main() {
                use_a(get_b())
                    //^^^^^^^ err: Incompatible type '0x1::M::B', expected '0x1::M::A'
            }
        }
    "#]]);
}

#[test]
fn test_incorrect_type_of_argument_with_call_expr_from_different_module() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::Other {
            struct B {}
            public fun get_b(): B { B {} }
        }
        module 0x1::M {
            use 0x1::Other::get_b;

            struct A {}
            fun use_a(a: A) {}

            fun main() {
                use_a(get_b())
                    //^^^^^^^ err: Incompatible type '0x1::Other::B', expected '0x1::M::A'
            }
        }
    "#]]);
}

#[test]
fn test_bytearray_is_vector_u8() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            fun send(a: vector<u8>) {}
            fun main() {
                let a = b"deadbeef";
                send(a)
            }
        }
    "#]]);
}

#[test]
fn test_no_error_for_compatible_generic_with_explicit_parameter() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct Diem<CoinType> has store { val: u64 }
            struct Balance<Token> has key {
                coin: Diem<Token>
            }

            fun value<CoinType: store>(coin: &Diem<CoinType>) {}

            fun main<Token: store>() {
                let balance: Balance<Token>;
                let coin = &balance.coin;
                value<Token>(coin)
            }
        }
    "#]]);
}

#[test]
fn test_no_error_for_compatible_generic_with_inferred_parameter() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct Diem<CoinType> has store { val: u64 }
            struct Balance<Token> has key {
                coin: Diem<Token>
            }

            fun value<CoinType: store>(coin: &Diem<CoinType>) {}

            fun main<Token: store>() {
                let balance: Balance<Token>;
                let coin = &balance.coin;
                value(coin)
            }
        }
    "#]]);
}

#[test]
fn test_no_return_type_but_returns_integer() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            fun call() {
                return 1;
                     //^ err: Incompatible type 'integer', expected '()'
            }
        }
    "#]]);
}

#[test]
fn test_no_return_type_but_returns_integer_with_expression() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            fun call() {
                return 1
                     //^ err: Incompatible type 'integer', expected '()'
            }
        }
    "#]]);
}

#[test]
fn test_if_statement_returns_unit() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            fun m() {
                if (true) {1} else {2};
            }
        }
    "#]]);
}

#[test]
fn test_block_expr_returns_unit() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            fun m() {
                {1};
            }
        }
    "#]]);
}

#[test]
fn test_error_on_code_block_if_empty_block_and_return_type() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            fun call(): u8 {}
                          //^ err: Incompatible type '()', expected 'u8'
        }
    "#]]);
}
