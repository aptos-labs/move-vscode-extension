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
              //^ err: Invalid argument to '+': expected integer type, but found 'bool'
                + b
                //^ err: Invalid argument to '+': expected integer type, but found 'bool'
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
              //^ err: Invalid argument to '+': expected integer type, but found 'T'
                + b;
                //^ err: Invalid argument to '+': expected integer type, but found 'T'
            }
        }
    "#]]);
}

#[test]
fn test_cannot_add_bool_and_u64() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            fun main() {
                false + 1u64;
              //^^^^^ err: Invalid argument to '+': expected integer type, but found 'bool'
            }
        }
    "#]]);
}

#[test]
fn test_cannot_add_u8_and_u64() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            fun main() {
                1u8 + 1u64;
              //^^^^^^^^^^ err: Incompatible arguments to '+': 'u8' and 'u64'
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
                      //^ err: Invalid argument to '+': expected integer type, but found 'bool'
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
                   //^ err: Invalid argument to '+': expected integer type, but found 'bool'
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

#[test]
fn test_tuple_unpacking_with_three_elements_when_two_are_specified() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            fun tuple(): (u8, u8, u8) { (1, 1, 1) }
            fun main() {
                let (a, b) = tuple();
                  //^^^^^^ err: Invalid unpacking. Expected tuple binding of length 3: (_, _, _)
            }
        }
    "#]]);
}

#[test]
fn test_invalid_tuple_unpacking_with_nested_error() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct S { val: u8 }
            fun tuple(): (u8, u8, u8) { (1, 1, 1) }
            fun main() {
                let (S { val }, b) = tuple();
                   //^^^^^^^^^ err: Assigned expr of type 'u8' cannot be unpacked with struct pattern
            }
        }
    "#]]);
}

#[test]
fn test_tuple_unpacking_into_struct_when_tuple_pat_is_expected() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct S { val: u8 }
            fun tuple(): (u8, u8, u8) { (1, 1, 1) }
            fun main() {
                let S { val } = tuple();
                  //^^^^^^^^^ err: Invalid unpacking. Expected tuple binding of length 3: (_, _, _)
            }
        }
    "#]]);
}

#[test]
fn test_unpacking_struct_into_variable() {
    // language=Move
    check_diagnostics(expect![[r#"
    module 0x1::M {
        struct S { val: u8 }
        fun s(): S { S { val: 10 } }
        fun main() {
            let s = s();
        }
    }
    "#]]);
}

#[test]
fn test_error_unpacking_struct_into_tuple() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct S { val: u8 }
            fun s(): S { S { val: 10 } }
            fun main() {
                let (a, b) = s();
                  //^^^^^^ err: Invalid unpacking. Expected struct binding of type '0x1::M::S'
            }
        }
    "#]]);
}

#[test]
fn test_error_unpacking_struct_into_tuple_when_single_var_is_expected() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct S { val: u8 }
            fun s(): u8 { 1 }
            fun main() {
                let (a, b) = s();
                  //^^^^^^ err: Assigned expr of type 'u8' cannot be unpacked with tuple pattern
            }
        }
    "#]]);
}

#[test]
fn test_error_parameter_type_with_return_type_inferred() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            fun identity<T>(a: T): T { a }
            fun main() {
                let a: u8 = identity(1u64);
                                   //^^^^ err: Incompatible type 'u64', expected 'u8'
            }
        }
    "#]]);
}

#[test]
fn test_all_integers_are_nums_in_spec_block() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            spec fun spec_pow(y: u64, x: u64): u64 {
                if (x == 0) {
                    1
                } else {
                    y * spec_pow(y, x - 1)
                }
            }

            /// Returns 10^degree.
            public fun pow_10(degree: u8): u64 {
                let res = 1;
                let i = 0;
                while ({
                    spec {
                        invariant res == spec_pow(10, i);
                        invariant 0 <= i && i <= degree;
                    };
                    i < degree
                }) {
                    res *= 10;
                    i += 1;
                };
                res
            }
        }
    "#]]);
}

#[test]
fn test_no_error_unpacking_struct_from_move_from() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            struct Container has key { val: u8 }
            fun main() {
                let Container { val } = move_from(@0x1);
            }
        }
    "#]]);
}

#[test]
fn test_vector_lit_with_explicit_type_and_type_error() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            fun main() {
                vector<u8>[1u64];
                         //^^^^ err: Incompatible type 'u64', expected 'u8'
            }
        }
    "#]]);
}

#[test]
fn test_vector_lit_with_implicit_type_and_type_error() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            fun main() {
                vector[1u8, 1u64];
                          //^^^^ err: Incompatible type 'u64', expected 'u8'
            }
        }
    "#]]);
}

#[test]
fn test_call_expr_with_incomplete_arguments_and_explicit_type() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            fun call<T>(a: T, b: T): T {
                b
            }
            fun main() {
                call<u8>(1u64);
                       //^^^^ err: Incompatible type 'u64', expected 'u8'
            }
        }
    "#]]);
}

#[test]
fn test_call_expr_with_incomplete_arguments_and_implicit_type() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            fun call<T>(a: T, b: T, c: T): T {
                b
            }
            fun main() {
                call(1u8, 1u64);
                        //^^^^ err: Incompatible type 'u64', expected 'u8'
            }
        }
    "#]]);
}

#[test]
fn test_option_none_is_compatible_with_any_option() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::option {
            struct Option<Element: copy + drop + store> has copy, drop, store {
                vec: vector<Element>
            }
            public fun none<Element: copy + drop + store>(): Option<Element> {
                Option { vec: vector[] }
            }
        }
        module 0x1::main {
            use 0x1::option;
            struct IterableValue<K: copy + store + drop> has store {
                prev: option::Option<K>,
                next: option::Option<K>,
            }
            public fun new() {
                IterableValue { prev: option::none(), next: option::none() };
            }
        }
    "#]]);
}

#[test]
fn test_nested_boxes() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            struct Box<T> has copy, drop, store { x: T }
            fun box1<T>(x: T): Box<Box<T>> {
                Box { x: Box { x } }
            }
        }
    "#]]);
}

#[test]
fn test_deeply_nested_structure_type_is_unknown_due_to_memory_issues() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            struct Box<T> has copy, drop, store { x: T }
            struct Box3<T> has copy, drop, store { x: Box<Box<T>> }
            struct Box7<T> has copy, drop, store { x: Box3<Box3<T>> }
            struct Box15<T> has copy, drop, store { x: Box7<Box7<T>> }
            struct Box31<T> has copy, drop, store { x: Box15<Box15<T>> }
            struct Box63<T> has copy, drop, store { x: Box31<Box31<T>> }

            fun box3<T>(x: T): Box3<T> {
                Box3 { x: Box { x: Box { x } } }
            }

            fun box7<T>(x: T): Box7<T> {
                Box7 { x: box3(box3(x)) }
            }

            fun box15<T>(x: T): Box15<T> {
                Box15 { x: box7(box7(x)) }
            }

            fun box31<T>(x: T): Box31<T> {
                Box31 { x: box15(box15(x)) }
            }

            fun box63<T>(x: T): Box63<T> {
                Box63 { x: box31(box31(x)) }
            }

            fun main() {
                let a: Box63<u8>;
                a;
            }
        }
    "#]]);
}

#[test]
fn test_no_invalid_unpacking_error_for_unresolved_name_in_tuple() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            fun main() {
                let (a, b) = call();
                           //^^^^ err: Unresolved reference `call`: cannot resolve
            }
        }
    "#]]);
}

#[test]
fn test_no_invalid_unpacking_error_for_unresolved_name_in_struct() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            struct S { val: u8 }
            fun main() {
                let S { val } = call();
                              //^^^^ err: Unresolved reference `call`: cannot resolve
            }
        }
    "#]]);
}

#[test]
fn test_loop_never_returns_and_not_a_type_error() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            fun main(): u64 {
                let a = 1;
                if (a == 1) return a;
                loop {}
            }
        }
    "#]]);
}

#[test]
fn test_integer_arguments_of_the_same_type_support_ordering() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            fun main(a: u64, b: u64) {
                let c = 1;
                a < b;
                a > b;
                a >= b;
                a <= b;
                a < c;
                b < c;
            }
        }
    "#]]);
}

#[test]
fn test_cannot_order_references() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            fun main(a: &u64, b: &u64) {
                a < b;
              //^ err: Invalid argument to '<': expected integer type, but found '&u64'
                  //^ err: Invalid argument to '<': expected integer type, but found '&u64'
            }
        }
    "#]]);
}

#[test]
fn test_cannot_order_bools() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            fun main(a: bool, b: bool) {
                a < b;
              //^ err: Invalid argument to '<': expected integer type, but found 'bool'
                  //^ err: Invalid argument to '<': expected integer type, but found 'bool'
            }
        }
    "#]]);
}

#[test]
fn test_cannot_order_type_parameters() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            fun main<T>(a: T, b: T) {
                a < b;
              //^ err: Invalid argument to '<': expected integer type, but found 'T'
                  //^ err: Invalid argument to '<': expected integer type, but found 'T'
            }
        }
    "#]]);
}

#[test]
fn test_eq_is_supported_for_same_type_arguments() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            struct S { val: u8 }
            fun main<T>(a: T, b: T) {
                1 == 1;
                1u8 == 1u8;
                1u64 == 1u64;
                false == false;
                S { val: 10 } == S { val: 20 };
                a == b;
            }
        }
    "#]]);
}

#[test]
fn test_not_eq_is_supported_for_same_type_arguments() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            struct S { val: u8 }
            fun main<T>(a: T, b: T) {
                1 != 1;
                1u8 != 1u8;
                1u64 != 1u64;
                false != false;
                S { val: 10 } != S { val: 20 };
                a != b;
            }
        }
    "#]]);
}

#[test]
fn test_cannot_eq_different_types() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            struct S { val: u64 }
            fun main() {
                1 == false;
              //^^^^^^^^^^ err: Incompatible arguments to '==': 'integer' and 'bool'
                S { val: 10 } == false;
              //^^^^^^^^^^^^^^^^^^^^^^ err: Incompatible arguments to '==': '0x1::main::S' and 'bool'
            }
        }
    "#]]);
}

#[test]
fn test_cannot_eq_different_integers() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            fun main() {
                1u8 == 1u64;
              //^^^^^^^^^^^ err: Incompatible arguments to '==': 'u8' and 'u64'
            }
        }
    "#]]);
}

#[test]
fn test_cannot_not_eq_different_integers() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            fun main() {
                1u8 != 1u64;
              //^^^^^^^^^^^ err: Incompatible arguments to '!=': 'u8' and 'u64'
            }
        }
    "#]]);
}

#[test]
fn test_logic_ops_allow_bools() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            fun main() {
                true && true;
                false || false;
            }
        }
    "#]]);
}

#[test]
fn test_logic_ops_invalid_argument_type() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            fun main() {
                1u8 && 1u64
              //^^^ err: Incompatible type 'u8', expected 'bool'
                     //^^^^ err: Incompatible type 'u64', expected 'bool'
            }
        }
    "#]]);
}

#[test]
fn test_if_else_with_different_generic_parameters() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            struct G<X, Y> {}
            fun main<X, Y>() {
                if (true) {
                    G<X, Y> {}
                } else {
                    G<Y, X> {}
                  //^^^^^^^^^^ err: Incompatible type '0x1::main::G<Y, X>', expected '0x1::main::G<X, Y>'
                };
            }
        }
    "#]]);
}

#[test]
fn test_type_cannot_contain_itself() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            struct S { val: S }
                          //^ err: Circular reference of type 'S'
        }
    "#]]);
}

#[test]
fn test_type_cannot_contain_itself_in_vector() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::main {
            struct S { val: vector<S> }
                                 //^ err: Circular reference of type 'S'
        }
    "#]]);
}

#[test]
fn test_recursive_structs() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x42::M0 {
            struct Foo { f: Foo }
                          //^^^ err: Circular reference of type 'Foo'

            struct Cup<T> { f: T }
            struct Bar { f: Cup<Bar> }
                              //^^^ err: Circular reference of type 'Bar'

            struct X { y: vector<Y> }
            struct Y { x: vector<X> }

        }

        module 0x42::M1 {
            use 0x42::M0;

            struct Foo { f: M0::Cup<Foo> }
                                  //^^^ err: Circular reference of type 'Foo'

            struct A { b: B }
            struct B { c: C }
            struct C { d: vector<D> }
            struct D { x: M0::Cup<M0::Cup<M0::Cup<A>>> }
        }
    "#]]);
}

#[test]
fn test_no_error_for_table_borrow_mut_of_unknown_type() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::table {
            /// Type of tables
            struct Table<phantom K: copy + drop, V: store> has store {
                inner: V
            }
            public fun borrow_mut<K: copy + drop, V: store>(table: &mut Table<K, V>, key: K): &mut V {
                &mut table.inner
            }
        }
        module 0x1::pool {
            use 0x1::table;
            struct Pool {
                shares: Unknown
                      //^^^^^^^ err: Unresolved reference `Unknown`: cannot resolve
            }
            fun call(pool: &mut Pool) {
                let value = table::borrow_mut(&mut pool.shares, @0x1);
                let unref = *value;
                1u128 - unref;
            }
        }
    "#]]);
}

#[test]
fn test_no_error_for_nested_struct_literal_and_explicit_type() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::M {
            struct Option<Element> { element: Element }
            struct S { id: Option<u64> }

            fun m() {
                S { id: Option { element: 1u64 } };
            }
        }
    "#]]);
}

#[test]
fn test_if_else_incompatible_expected_type() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun main() {
                let a = 1u8;
                a = if (true) 1u16 else 1u32;
                            //^^^^ err: Incompatible type 'u16', expected 'u8'
                                      //^^^^ err: Incompatible type 'u32', expected 'u8'
                a = if (true) 1u8 else 1u32;
                                     //^^^^ err: Incompatible type 'u32', expected 'u8'
                a = if (true) 1u16 else 1u8;
                            //^^^^ err: Incompatible type 'u16', expected 'u8'
            }
        }
    "#]]);
}

#[test]
fn test_if_else_incompatible_expected_type_both_incompatible_but_compat_to_each_other() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun main() {
                let a = 1;
                a = if (true) false else true;
                  //^^^^^^^^^^^^^^^^^^^^^^^^^ err: Incompatible type 'bool', expected 'integer'
            }
        }
    "#]]);
}

#[test]
fn test_if_else_with_expected_type_of_mut_ref_then_incompat() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            struct S { val: u8 }
            fun main() {
                let mut_s = &mut S { val: 2 };
                let s = S { val: 1 };
                mut_s = if (true) &mut s else &s;
                                            //^^ err: Incompatible type '&0x1::m::S', expected '&mut 0x1::m::S'
            }
        }
    "#]]);
}

#[test]
fn test_if_else_with_expected_type_of_mut_ref_else_incompat() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            struct S { val: u8 }
            fun main() {
                let mut_s = &mut S { val: 2 };
                let s = S { val: 1 };
                mut_s = if (true) &s else &mut s;
                                //^^ err: Incompatible type '&0x1::m::S', expected '&mut 0x1::m::S'
            }
        }
    "#]]);
}

#[test]
fn test_if_else_with_expected_type_of_ref_but_incompat_no_error() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            struct S { val: u8 }
            fun main() {
                let mut_s = &S { val: 2 };
                let s = S { val: 1 };
                mut_s = if (true) &s else &mut s;
            }
        }
    "#]]);
}

#[test]
fn test_if_branch_returns_unit() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun main() {
                let a = 1;
                a = if (true) {} else 1;
                             //^ err: Incompatible type '()', expected 'integer'
            }
        }
    "#]]);
}

#[test]
fn test_else_branch_returns_unit() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun main() {
                let a = 1;
                a = if (true) {} else 1;
                             //^ err: Incompatible type '()', expected 'integer'
            }
        }
    "#]]);
}

#[test]
fn test_if_else_uninitialized_integer_with_bin_expr() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun main() {
                let lt;
                if (true) {
                    lt = 1;
                } else {
                    lt = 2;
                };
                lt - 1;
            }
        }
    "#]]);
}

#[test]
fn test_no_invalid_unpacking_for_full_struct_pat() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            struct S<phantom CoinType> { amount: u8 }
            fun call<CallCoinType>(s: S<CallCoinType>) {
                let S { amount: my_amount } = s;
            }
        }
    "#]]);
}

#[test]
fn test_no_invalid_unpacking_for_shorthand_struct_pat() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            struct S<phantom CoinType> { amount: u8 }
            fun call<CallCoinType>(s: S<CallCoinType>) {
                let S { amount } = s;
            }
        }
    "#]]);
}

#[test]
fn test_no_invalid_unpacking_variable_in_parens() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun call() {
                let (a) = 1;
            }
        }
    "#]]);
}

#[test]
fn test_check_type_of_assigning_value_in_tuple_assignment() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            struct Coin<CoinType> { val: u8 }
            fun coin_zero<CoinType>(): Coin<CoinType> { Coin { val: 0 } }
            fun call<CallCoinType>() {
                let a = 0;
                (a, _) = (coin_zero<CallCoinType>(), 2);
                        //^^^^^^^^^^^^^^^^^^^^^^^^^ err: Incompatible type '0x1::m::Coin<CallCoinType>', expected 'integer'
            }
        }
    "#]]);
}

#[test]
fn test_deref_type_error() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            struct S {}
            fun main() {
                let s = S {};
                let mut_s = &mut s;
                let b: bool = *mut_s;
                            //^^^^^^ err: Incompatible type '0x1::m::S', expected 'bool'
            }
        }
    "#]]);
}

#[test]
fn test_shift_left_with_u64() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun main() {
                let a = 1u64;
                a << 1;
            }
        }
    "#]]);
}

#[test]
fn test_abort_expr_requires_integer() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun main() {
                abort 1;
                abort 1u8;
                abort 1u64;
                abort false;
                    //^^^^^ err: Incompatible type 'bool', expected 'integer'
            }
        }
    "#]]);
}

#[test]
fn test_aborts_if_requires_bool() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun call() {}
            spec call {
                aborts_if 1 with 1;
                        //^ err: Incompatible type 'num', expected 'bool'
            }
        }
    "#]]);
}

#[test]
fn test_aborts_if_with_requires_integer() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun call() {}
            spec call {
                aborts_if true with 1;
                aborts_if true with 1u8;
                aborts_if true with 1u64;
                aborts_if true with false;
                                  //^^^^^ err: Incompatible type 'bool', expected 'num'
            }
        }
    "#]]);
}

#[test]
fn test_aborts_with_requires_integer() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun call() {}
            spec call {
                aborts_with false;
                          //^^^^^ err: Incompatible type 'bool', expected 'num'
            }
        }
    "#]]);
}

#[test]
fn test_type_check_function_param_in_func_spec() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun call(val: bool) {}
            spec call {
                val + 1;
              //^^^ err: Invalid argument to '+': expected integer type, but found 'bool'
            }
        }
    "#]]);
}

#[test]
fn test_type_check_function_result_in_func_spec() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun call(): bool { true }
            spec call {
                result + 1;
              //^^^^^^ err: Invalid argument to '+': expected integer type, but found 'bool'
            }
        }
    "#]]);
}

// todo
#[test]
fn test_type_check_imply_expr_in_include() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            spec schema Schema {}
            spec module {
                include 1 ==> Schema {};
            }
        }
    "#]]);
}

#[test]
fn test_spec_vector_slice() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            spec module {
                let v = vector[true, false];
                v[0..1];
            }
        }
    "#]]);
}

#[test]
fn test_incompatible_integers_in_gte() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun main() {
                1u8 >= 1u64;
                     //^^^^ err: Incompatible type 'u64', expected 'u8'
            }
        }
    "#]]);
}

#[test]
fn test_bit_shift_requires_u8() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun main() {
                1 << 1000u64;
                   //^^^^^^^ err: Incompatible type 'u64', expected 'u8'
            }
        }
    "#]]);
}

#[test]
fn test_comma_separator_allows_correctly_get_call_expr_type() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun call(a: u64, b: u8) {}
            fun main() {
                call(,2u64);
                    //^^^^ err: Incompatible type 'u64', expected 'u8'
            }
        }
    "#]]);
}

#[test]
fn test_if_else_tail_expr_returns_incorrect_type() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun main() {
                let my_vol_ref = 1u64;
                my_vol_ref =
                    if (true) {
                        1 + 1;
                        1u32
                      //^^^^ err: Incompatible type 'u32', expected 'u64'
                    } else {
                        1 + 1;
                        1u128
                      //^^^^^ err: Incompatible type 'u128', expected 'u64'
                    };
            }
        }
    "#]]);
}

#[test]
fn test_block_returns_incorrect_type_tail_expr() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun main() {
                let my_vol_ref = 1u64;
                my_vol_ref = {
                        1 + 1;
                        1u32
                      //^^^^ err: Incompatible type 'u32', expected 'u64'
                    };
            }
        }
    "#]]);
}

#[test]
fn test_unpack_mut_ref_with_tuple_pattern() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            struct Bin has store {
                reserves_x: u64,
                reserves_y: u64,
                token_data_id: u64,
            }
            fun main(bin: &mut Bin) {
                let (a, b) = bin;
                  //^^^^^^ err: Assigned expr of type '&mut 0x1::m::Bin' cannot be unpacked with tuple pattern
            }
        }
    "#]]);
}

#[test]
fn test_cannot_reference_another_reference() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            struct Pool<X, Y> {}
            fun main<X, Y>(pool: &mut Pool<X, Y>) {
                &pool;
               //^^^^ err: Expected a single non-reference type, but found '&mut 0x1::m::Pool<X, Y>'
            }
        }
    "#]]);
}

#[test]
fn test_cannot_reference_tuple() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun call(): (u8, u8) { (1, 1) }
            fun main() {
                &call();
               //^^^^^^ err: Expected a single non-reference type, but found '(u8, u8)'
            }
        }
    "#]]);
}

#[test]
fn test_cannot_dereference_non_ref() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun main() {
                *1;
               //^ err: Invalid dereference. Expected '&_' but found 'integer'
            }
        }
    "#]]);
}

#[test]
fn test_range_expr_end_has_different_type() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun main() {
                let a = 1..true;
                         //^^^^ err: Incompatible type 'bool', expected 'integer'
            }
        }
    "#]]);
}

#[test]
fn test_return_type_inference_for_generic_expr() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            native fun borrow<Value>(): Value;
            fun main() {
                borrow() + 3;
            }
        }
    "#]]);
}

#[test]
fn test_return_type_inference_for_deref_borrow_expr_lhs() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            native fun borrow<Value>(): &Value;
            fun main() {
                *borrow() + 3;
            }
        }
    "#]]);
}

#[test]
fn test_return_type_inference_for_deref_borrow_expr_rhs() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            native fun borrow<Value>(): &Value;
            fun main() {
                3 + *borrow();
            }
        }
    "#]]);
}

#[test]
fn test_unpacking_of_struct_ref_allowed() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            struct Field { id: u8 }
            fun main() {
                let Field { id } = &Field { id: 1 };
            }
        }
    "#]]);
}

#[test]
fn test_incorrect_types_for_vector_borrow_methods() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::vector {
            native public fun borrow<Element>(v: &vector<Element>, i: u64): &Element;
            native public fun borrow_mut<Element>(v: &mut vector<Element>, i: u64): &mut Element;
        }
        module 0x1::m {
            use 0x1::vector;

            fun main() {
                let v = vector[1, 2];

                vector::borrow(0, &v);
                             //^ err: Incompatible type 'integer', expected '&vector<Element>'
                                //^^ err: Incompatible type '&vector<integer>', expected 'u64'
                vector::borrow(v, 0);
                             //^ err: Incompatible type 'vector<integer>', expected '&vector<Element>'

                vector::borrow_mut(0, &mut v);
                                 //^ err: Incompatible type 'integer', expected '&mut vector<Element>'
                                    //^^^^^^ err: Incompatible type '&mut vector<integer>', expected 'u64'
                vector::borrow_mut(v, 0);
                                 //^ err: Incompatible type 'vector<integer>', expected '&mut vector<Element>'
                vector::borrow_mut(&v, 0);
                                 //^^ err: Incompatible type '&vector<integer>', expected '&mut vector<integer>'
            }
        }
    "#]]);
}

#[test]
fn test_circular_types_for_enum() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            enum S { One { s: S }, Two }
                            //^ err: Circular reference of type 'S'
        }
    "#]]);
}

#[test]
fn test_return_enum_value_from_function() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            enum Iterator { Empty, Some { item: u8 } }
            fun create_iterator(): Iterator {
                Iterator::Empty
            }
        }
    "#]]);
}

#[test]
fn test_return_enum_value_with_fields_from_function() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            enum Iterator { Empty, Some { item: u8 } }
            fun create_iterator(): Iterator {
                Iterator::Some { item: 1 }
            }
        }
    "#]]);
}

#[test]
fn test_return_generic_empty_enum_value_from_function() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            enum Iterator<IterT> { Empty, Some { item: IterT } }
            fun create_iterator<FunT>(): Iterator<FunT> {
                Iterator::Empty
            }
        }
    "#]]);
}

#[test]
fn test_no_invalid_dereference_inside_lambda() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::vector {
            public fun enumerate_ref<Element>(self: vector<Element>, _f: |&Element|) {}
        }
        module 0x1::m {
            fun main() {
                vector[@0x1].enumerate_ref(|to| { *to; });
            }
        }
    "#]]);
}

#[test]
fn test_invalid_dereference_inside_lambda() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::vector {
            public fun enumerate_ref<Element>(self: vector<Element>, _f: |Element|) {}
        }
        module 0x1::m {
            fun main() {
                vector[@0x1].enumerate_ref(|to| { *to; });
                                                 //^^ err: Invalid dereference. Expected '&_' but found 'address'
            }
        }
    "#]]);
}

#[test]
fn test_struct_lit_with_expected_type_of_different_generic_argument() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            struct S<R> { val: R }
            fun main() {
                let s: S<u8> = S<u16> { val: 1 };
                             //^^^^^^^^^^^^^^^^^ err: Incompatible type '0x1::m::S<u16>', expected '0x1::m::S<u8>'
            }
        }
    "#]]);
}

#[test]
fn test_tuple_struct_lit_with_expected_type_of_different_generic_argument() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            struct S<R>(R);
            fun main() {
                let s: S<u8> = S<u16>(1);
                             //^^^^^^^^^ err: Incompatible type '0x1::m::S<u16>', expected '0x1::m::S<u8>'
            }
        }
    "#]]);
}

#[test]
fn test_struct_lit_field_with_expected_type_of_different_generic_argument() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            struct S<R> { val: R }
            fun main() {
                let s: S<u8> = S { val: 1u16 };
                                      //^^^^ err: Incompatible type 'u16', expected 'u8'
            }
        }
    "#]]);
}

#[test]
fn test_tuple_struct_lit_field_with_expected_type_of_different_generic_argument() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            struct S<R>(R);
            fun main() {
                let s: S<u8> = S(1u16);
                               //^^^^ err: Incompatible type 'u16', expected 'u8'
            }
        }
    "#]]);
}

#[test]
fn test_return_only_call_expr() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            native fun call(): u16;
            fun main() {
                let a: u8 = call();
                          //^^^^^^ err: Incompatible type 'u16', expected 'u8'
            }
        }
    "#]]);
}

#[test]
fn test_num_and_u64_in_spec_block_from_outer_scope() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun main() {
                let validator_index = 1;
                spec {
                    validator_index + 1;
                }
            }
        }
    "#]]);
}

#[test]
fn test_num_and_integer_returning_spec_fun_for_spec_block() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            struct Aggregator<IntElement> has store, drop {
                value: IntElement,
                max_value: IntElement,
            }
            spec native fun spec_get_max_value<IntElement>(aggregator: Aggregator<IntElement>): IntElement;
            fun main() {
                let agg = Aggregator { value: 1, max_value: 1 };
                spec {
                    assert spec_get_max_value(agg) == 10;
                }
            }
        }
    "#]]);
}

#[test]
fn test_compare_two_generics_from_spec_fun() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            struct Aggregator<IntElement> has store, drop {
                value: IntElement,
                max_value: IntElement,
            }
            spec native fun spec_get_max_value<IntElement>(aggregator: Aggregator<IntElement>): IntElement;
            fun main() {
                let agg = Aggregator { value: 1, max_value: 1 };
                spec {
                    assert spec_get_max_value(agg) < spec_get_max_value(agg);
                }
            }
        }
    "#]]);
}

#[test]
fn test_for_each_vector_fold() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::vector {
            /// Apply the function to each element in the vector, consuming it.
            public inline fun for_each<ForEachElement>(self: vector<ForEachElement>, f: |ForEachElement|) {}
            public inline fun fold<Accumulator, Element>(
                self: vector<Element>,
                init: Accumulator,
                f: |Accumulator,Element|Accumulator
            ): Accumulator {
                let accu = init;
                self.for_each(|elem| accu = f(accu, elem));
                accu
            }
        }
    "#]]);
}

#[test]
fn test_no_error_for_cast_expr_precedence() {
    // language=Move
    check_diagnostics(expect![[r#"
        module 0x1::m {
            fun main() {
                (1u8 + 2u8 as u16);
            }
        }
    "#]]);
}
