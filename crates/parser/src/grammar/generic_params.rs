use crate::grammar::attributes::ATTRIBUTE_FIRST;
use crate::grammar::utils::list;
use crate::grammar::{ability, name};
use crate::parser::{CompletedMarker, Marker};
use crate::token_set::TokenSet;
use crate::SyntaxKind::*;
use crate::{Parser, T};

pub(super) fn opt_generic_param_list(p: &mut Parser<'_>) {
    if p.at(T![<]) {
        generic_param_list(p);
    }
}

// test_err generic_param_list_recover
// fn f<T: Clone,, U:, V>() {}
fn generic_param_list(p: &mut Parser<'_>) {
    assert!(p.at(T![<]));
    let m = p.start();
    list(
        p,
        T![<],
        T![>],
        T![,],
        || "expected generic parameter".into(),
        TYPE_PARAM_FIRST.union(ATTRIBUTE_FIRST),
        |p| {
            // test generic_param_attribute
            // fn foo<#[lt_attr] 'a, #[t_attr] T>() {}
            let m = p.start();
            // attributes::outer_attrs(p);
            generic_param(p, m)
        },
    );

    m.complete(p, TYPE_PARAM_LIST);
}

const TYPE_PARAM_FIRST: TokenSet = TokenSet::new(&[IDENT, T![phantom]]);

fn generic_param(p: &mut Parser<'_>, m: Marker) -> bool {
    if p.at_contextual_kw_ident("phantom") {
        p.bump_remap(T![phantom]);
    }
    match p.current() {
        // LIFETIME_IDENT => lifetime_param(p, m),
        IDENT => type_param(p, m),
        // T![const] => const_param(p, m),
        _ => {
            m.abandon(p);
            p.error_and_bump_any("expected generic parameter");
            return false;
        }
    }
    true
}

// // test lifetime_param
// // fn f<'a: 'b>() {}
// fn lifetime_param(p: &mut Parser<'_>, m: Marker) {
//     assert!(p.at(LIFETIME_IDENT));
//     lifetime(p);
//     if p.eat(T![:]) {
//         lifetime_bounds(p);
//     }
//     m.complete(p, LIFETIME_PARAM);
// }

// test type_param
// fn f<T: Clone>() {}
fn type_param(p: &mut Parser<'_>, m: Marker) {
    assert!(p.at(IDENT));
    name(p);
    if p.at(T![:]) {
        ability_bounds(p);
    }
    m.complete(p, TYPE_PARAM);
}

fn ability_bounds(p: &mut Parser) {
    assert!(p.at(T![:]));
    p.bump(T![:]);
    ability_bound_list(p);
}

pub(crate) fn ability_bound_list(p: &mut Parser) {
    let m = p.start();
    while ability(p) {
        if !p.eat(T![+]) {
            break;
        }
    }
    m.complete(p, ABILITY_BOUND_LIST);
}

// // test const_param
// // struct S<const N: u32>;
// fn const_param(p: &mut Parser<'_>, m: Marker) {
//     p.bump(T![const]);
//     name(p);
//     if p.at(T![:]) {
//         types::ascription(p);
//     } else {
//         p.error("missing type for const parameter");
//     }
//
//     if p.eat(T![=]) {
//         // test const_param_default_literal
//         // struct A<const N: i32 = -1>;
//
//         // test const_param_default_expression
//         // struct A<const N: i32 = { 1 }>;
//
//         // test const_param_default_path
//         // struct A<const N: i32 = i32::MAX>;
//         generic_args::const_arg(p);
//     }
//
//     m.complete(p, CONST_PARAM);
// }

// fn lifetime_bounds(p: &mut Parser<'_>) {
//     let marker = p.start();
//     while {
//         if !matches!(p.current(), LIFETIME_IDENT | T![>] | T![,]) {
//             p.error("expected lifetime");
//         }
//
//         type_bound(p)
//     } {
//         if !p.eat(T![+]) {
//             break;
//         }
//     }
//     marker.complete(p, TYPE_BOUND_LIST);
// }

// // test type_param_bounds
// // struct S<T: 'a + ?Sized + (Copy) + ~const Drop>;
// pub(super) fn bounds(p: &mut Parser<'_>) {
//     p.expect(T![:]);
//     bounds_without_colon(p);
// }

// pub(super) fn bounds_without_colon(p: &mut Parser<'_>) {
//     let m = p.start();
//     bounds_without_colon_m(p, m);
// }

// pub(super) fn bounds_without_colon_m(p: &mut Parser<'_>, marker: Marker) -> CompletedMarker {
//     while type_bound(p) {
//         if !p.eat(T![+]) {
//             break;
//         }
//     }
//     marker.complete(p, TYPE_BOUND_LIST)
// }

// fn type_bound(p: &mut Parser<'_>) -> bool {
//     let m = p.start();
//     let has_paren = p.eat(T!['(']);
//     match p.current() {
//         LIFETIME_IDENT => lifetime(p),
//         T![for] => types::for_type(p, false),
//         // test precise_capturing
//         // fn captures<'a: 'a, 'b: 'b, T>() -> impl Sized + use<'b, T, Self> {}
//
//         // test_err precise_capturing_invalid
//         // type T = impl use<self, 1>;
//         T![use] if p.nth_at(1, T![<]) => {
//             p.bump_any();
//             let m = p.start();
//             delimited(
//                 p,
//                 T![<],
//                 T![>],
//                 T![,],
//                 || "expected identifier or lifetime".into(),
//                 TokenSet::new(&[T![Self], IDENT, LIFETIME_IDENT]),
//                 |p| {
//                     if p.at(LIFETIME_IDENT) {
//                         lifetime(p);
//                     } else {
//                         name_ref_or_upper_self(p);
//                     }
//                     true
//                 },
//             );
//             m.complete(p, USE_BOUND_GENERIC_ARGS);
//         }
//         T![?] if p.nth_at(1, T![for]) => {
//             // test question_for_type_trait_bound
//             // fn f<T>() where T: ?for<> Sized {}
//             p.bump_any();
//             types::for_type(p, false)
//         }
//         current => {
//             match current {
//                 T![?] => p.bump_any(),
//                 T![~] => {
//                     p.bump_any();
//                     p.expect(T![const]);
//                 }
//                 // test const_trait_bound
//                 // const fn foo(_: impl const Trait) {}
//                 T![const] => {
//                     p.bump_any();
//                 }
//                 // test async_trait_bound
//                 // fn async_foo(_: impl async Fn(&i32)) {}
//                 T![async] => {
//                     p.bump_any();
//                 }
//                 _ => (),
//             }
//             if paths::is_use_path_start(p) {
//                 types::path_type_bounds(p, false);
//             } else {
//                 m.abandon(p);
//                 return false;
//             }
//         }
//     }
//     if has_paren {
//         p.expect(T![')']);
//     }
//     m.complete(p, TYPE_BOUND);
//
//     true
// }

// // test where_clause
// // fn foo()
// // where
// //    'a: 'b + 'c,
// //    T: Clone + Copy + 'static,
// //    Iterator::Item: 'a,
// //    <T as Iterator>::Item: 'a
// // {}
// pub(super) fn opt_where_clause(p: &mut Parser<'_>) {
//     if !p.at(T![where]) {
//         return;
//     }
//     let m = p.start();
//     p.bump(T![where]);
//
//     while is_where_predicate(p) {
//         where_predicate(p);
//
//         let comma = p.eat(T![,]);
//
//         match p.current() {
//             T!['{'] | T![;] | T![=] => break,
//             _ => (),
//         }
//
//         if !comma {
//             p.error("expected comma");
//         }
//     }
//
//     m.complete(p, WHERE_CLAUSE);
//
//     // fn is_where_predicate(p: &mut Parser<'_>) -> bool {
//     //     match p.current() {
//     //         LIFETIME_IDENT => true,
//     //         T![impl] => false,
//     //         token => types::TYPE_FIRST.contains(token),
//     //     }
//     // }
// }

// fn where_predicate(p: &mut Parser<'_>) {
//     let m = p.start();
//     match p.current() {
//         LIFETIME_IDENT => {
//             lifetime(p);
//             if p.at(T![:]) {
//                 bounds(p);
//             } else {
//                 p.error("expected colon");
//             }
//         }
//         T![impl] => {
//             p.error("expected lifetime or type");
//         }
//         _ => {
//             if p.at(T![for]) {
//                 // test where_pred_for
//                 // fn for_trait<F>()
//                 // where
//                 //    for<'a> F: Fn(&'a str)
//                 // { }
//                 types::for_binder(p);
//             }
//
//             types::type_(p);
//
//             if p.at(T![:]) {
//                 bounds(p);
//             } else {
//                 p.error("expected colon");
//             }
//         }
//     }
//     m.complete(p, WHERE_PRED);
// }
