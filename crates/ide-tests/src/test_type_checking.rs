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

#[test]
fn test_vector_push_back() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            native public fun push_back<Element>(v: &mut vector<Element>, e: Element);

            fun m<E: drop>(v: &mut vector<E>, x: E): u8 {
                push_back(v, x)
              //^^^^^^^^^^^^^^^ err: Incompatible type '()', expected 'u8'
            }
        }
    "#]]);
}

#[test]
fn test_if_condition_should_be_boolean() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            fun m() {
                if (1) 1;
                  //^ err: Incompatible type 'integer', expected 'bool'
            }
        }
    "#]]);
}

#[test]
fn test_incompatible_types_in_if_branches() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            fun m() {
                if (true) {1} else {true};
                                  //^^^^ err: Incompatible type 'bool', expected 'integer'
            }
        }
    "#]]);
}

#[test]
fn test_no_type_error_with_explicit_generic_in_move_to() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct Option<Element: store> has store {
                element: Element
            }
            public fun some<SomeElement: store>(e: SomeElement): Option<SomeElement> {
                Option { element: e }
            }
            struct Vault<VaultContent: store> has key {
                content: Option<VaultContent>
            }
            public fun new<Content: store>(owner: &signer,  content: Content) {
                move_to<Vault<Content>>(
                    owner,
                    Vault { content: some(content) }
                )
            }
        }
    "#]]);
}

#[test]
fn test_type_check_incompatible_constraints() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct C {}
            struct D {}
            fun new<Content>(a: Content, b: Content): Content { a }
            fun m() {
                new(C {}, D {});
                        //^^^^ err: Incompatible type '0x1::M::D', expected '0x1::M::C'
            }
        }
    "#]]);
}

#[test]
fn test_error_if_resolved_type_requires_a_reference() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            fun index_of<Element>(v: &vector<Element>, e: &Element): (bool, u64) {
                (false, 0)
            }
            fun m() {
                let ids: vector<u64>;
                index_of(&ids, 1u64);
                             //^^^^ err: Incompatible type 'u64', expected '&u64'
            }
        }
    "#]]);
}

#[test]
fn test_return_generic_tuple_from_nested_callable() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct MintCapability<phantom CoinType> has key, store {}
            struct BurnCapability<phantom CoinType> has key, store {}

            public fun register_native_currency<FCoinType>(): (MintCapability<FCoinType>, BurnCapability<FCoinType>) {
                register_currency<FCoinType>()
            }
            public fun register_currency<FCoinType>(): (MintCapability<FCoinType>, BurnCapability<FCoinType>) {
                return (MintCapability<FCoinType>{}, BurnCapability<FCoinType>{})
            }
        }
    "#]]);
}

#[test]
fn test_emit_event_requires_mutable_reference() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct EventHandle<phantom T: drop + store> has store {
                counter: u64,
                guid: vector<u8>,
            }
            struct Account has key {
                handle: EventHandle<Event>
            }
            struct Event has store, drop {}
            fun emit_event<T: drop + store>(handler_ref: &mut EventHandle<T>, msg: T) {}
            fun m<Type: store + drop>() acquires Account {
                emit_event(borrow_global_mut<Account>(@0x1).handle, Event {});
                         //^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ err: Incompatible type '0x1::M::EventHandle<0x1::M::Event>', expected '&mut 0x1::M::EventHandle<0x1::M::Event>'
            }
        }
    "#]]);
}

#[test]
fn test_invalid_type_for_field_in_struct_literal() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct Deal { val: u8 }
            fun main() {
                Deal { val: false };
                          //^^^^^ err: Incompatible type 'bool', expected 'u8'
            }
        }
    "#]]);
}

#[test]
fn test_valid_type_for_field() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct Deal { val: u8 }
            fun main() {
                Deal { val: 10 };
                Deal { val: 10u8 };
            }
        }
    "#]]);
}

#[test]
fn test_no_need_for_explicit_type_parameter_if_inferrable_from_context() {
    // language=Move
    check_diagnostics(expect![[r#"
    module 0x1::M {
        struct Option<Element> has copy, drop, store {}
        public fun none<NoneElement>(): Option<NoneElement> {
            Option {}
        }
        struct S { field: Option<address> }
        fun m(): S {
            S { field: none() }
        }
    }
    "#]]);
}

#[test]
fn test_no_need_for_vector_empty_generic() {
    // language=Move
    check_diagnostics(expect![[r#"
    module 0x1::M {
        /// Create an empty vector.
        native public fun empty<Element>(): vector<Element>;
        struct CapState<phantom Feature> has key {
            delegates: vector<address>
        }
        fun m() {
            CapState { delegates: empty() };
        }
    }
    "#]]);
}

#[test]
fn test_type_error_in_struct_literal_field_shorthand() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct S { a: u8 }
            fun m() {
                let a = true;
                S { a };
                  //^ err: Incompatible type 'bool', expected 'u8'
            }
        }
    "#]]);
}

#[test]
fn test_do_not_crash_type_checking_invalid_number_of_type_params_or_call_params() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct S<R: key> { val: R }
            fun call(a: u8) {}
            fun m() {
                let s = S<u8, u8>{};
                call(1, 2, 3);
            }
        }
    "#]]);
}

#[test]
fn test_explicit_unit_return() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            fun m(): () {}
        }
    "#]]);
}

#[test]
fn test_if_else_with_reference_no_error_if_coercable() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct S has drop {}
            fun m() {
                let s = S {};
                let _ = if (true) &s else &mut s;
                let _ = if (true) &mut s else &s;
            }
        }
    "#]]);
}

#[test]
fn test_incorrect_type_address_passed_where_signer_is_expected_in_spec() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            fun send(account: &signer) {}

            spec send {
                send(@0x1);
                   //^^^^ err: Incompatible type 'address', expected '&signer'
            }
        }
    "#]]);
}

#[test]
fn test_signer_compatibility_in_spec() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            fun address_of(account: &signer): address { @0x1 }
            fun send(account: &signer) {}
            spec send {
                address_of(account);
            }
        }
    "#]]);
}

#[test]
fn test_vector_u8_is_compatible_with_vector_num_in_spec() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct S {
                val: vector<u8>
            }
            spec module {
                S { val: b"" };
            }
        }
    "#]]);
}

#[test]
fn test_ref_equality_for_generics_in_spec_call_expr() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct Token<TokenT> {}
            fun call<TokenT>(ref: &Token<TokenT>) {
                let token = Token<TokenT> {};
                spec {
                    call(token);
                }
            }
        }
    "#]]);
}

#[test]
fn test_invalid_argument_to_plus_expr() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            fun add(a: bool, b: bool) {
                a
              //^ err: Invalid argument to +: expected integer type, but found bool
                + b
                //^ err: Invalid argument to +: expected integer type, but found bool
            }
        }
    "#]]);
}

#[test]
fn test_invalid_argument_to_plus_expr_for_type_parameter() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            fun add<T>(a: T, b: T) {
                a
              //^ err: Invalid argument to +: expected integer type, but found T
                + b;
                //^ err: Invalid argument to +: expected integer type, but found T
            }
        }
    "#]]);
}

#[test]
fn test_no_error_if_return_nested_in_if_and_while() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            fun main(): u8 {
                let i = 0;
                while (true) {
                    if (true) return i
                };
                i
            }
        }
    "#]]);
}

#[test]
fn test_no_error_empty_return() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            fun main() {
                if (true) return
                return
            }
        }
    "#]]);
}

#[test]
fn test_no_error_return_tuple_from_if_else() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            fun main(): (u8, u8) {
                if (true) {
                    return (1, 1)
                } else {
                    return (2, 2)
                }
            }
        }
    "#]]);
}

#[test]
fn test_no_error_return_tuple_from_nested_if_else() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            fun main(): (u8, u8) {
                if (true) {
                    if (true) {
                        return (1, 1)
                    } else {
                        return (2, 2)
                    }
                } else {
                    return (3, 3)
                }
            }
        }
    "#]]);
}

#[test]
fn test_error_add_bool_in_assignment_expr() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            fun main() {
                let a = 1u64;
                let b = false;
                a = a + b;
                      //^ err: Invalid argument to +: expected integer type, but found bool
              //^^^^^^^^^ weak: Can be replaced with compound assignment
            }
        }
    "#]]);
}

#[test]
fn test_error_add_bool_in_compound_assignment_expr() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            fun main() {
                let a = 1u64;
                let b = false;
                a += b;
                   //^ err: Invalid argument to +: expected integer type, but found bool
            }
        }
    "#]]);
}

#[test]
fn test_error_invalid_assignment_type() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            fun main() {
                let a = 1u64;
                a = false;
                  //^^^^^ err: Incompatible type 'bool', expected 'u64'
            }
        }
    "#]]);
}
