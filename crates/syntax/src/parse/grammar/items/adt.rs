use crate::parse::grammar::attributes::ATTRIBUTE_FIRST;
use crate::parse::grammar::items::{at_block_start, at_item_start, item_start_tokens};
// use crate::parse::grammar::types::type_or;
use crate::parse::grammar::utils::{delimited_with_recovery, list};
use crate::parse::grammar::{
    ability, attributes, error_block, item_name_or_recover, name, name_or_recover, type_params, types,
};
use crate::parse::parser::{Marker, Parser};
use crate::parse::token_set::TokenSet;
use crate::SyntaxKind::*;
use crate::{ts, SyntaxKind, T};

// test struct_item
// struct S {}
pub(super) fn struct_(p: &mut Parser, m: Marker) {
    p.bump(T![struct]);
    item_name_or_recover(p, struct_enum_recover_at);
    type_params::opt_type_param_list(p);
    opt_abilities_list(p, vec![T!['{'], T!['(']]);
    // opt_abilities_list(p, T!['{'] | T!['(']);
    // if p.at_contextual_kw_ident("has") {
    //     p.with_recover_fn(at_item_start, |p| {
    //         p.with_recover_ts(T!['{'] | T!['('], |p| abilities_list(p, TokenSet::EMPTY));
    //     })
    // }
    // p.with_recover_fn(at_item_start, |p| {
    // });
    // p.with_recover_ts(T!['{'] | T!['('], |p| opt_abilities_list(p, TokenSet::EMPTY));
    // opt_abilities_list(p, ts!(T!['{']));
    match p.current() {
        T!['{'] => {
            p.with_recover_token(T!['}'], |p| named_field_list(p));
            opt_abilities_list_with_semicolon(p);
        }
        T![;] => {
            p.bump(T![;]);
        }
        T!['('] => {
            tuple_field_list(p);
            opt_abilities_list(p, vec![T![;]]);
            p.expect(T![;]);
        }
        _ => p.error("expected `;`, `{`, or `(`"),
    }
    // opt_abilities_list(p);
    m.complete(p, STRUCT);
}

fn opt_abilities_list_with_semicolon(p: &mut Parser) {
    let has_postfix_abilities = opt_abilities_list(p, vec![T![;]]);
    if has_postfix_abilities {
        p.expect(T![;]);
    }
}

fn opt_abilities_list(p: &mut Parser, extra_recover_set: Vec<SyntaxKind>) -> bool {
    if p.at_contextual_kw_ident("has") {
        let extra_set = extra_recover_set.into_iter().map(|it| it.into()).collect();
        p.with_recover_tokens(item_start_tokens(), |p| {
            p.with_recover_tokens(extra_set, |p| abilities_list(p, TokenSet::EMPTY));
        });
        // p.with_recover_fn(at_item_start, |p| {});
        return true;
    }
    // if p.at_contextual_kw_ident("has") {
    //     abilities_list(p, extra_recover_set);
    //     return true;
    // }
    false
}

fn abilities_list(p: &mut Parser, extra_set: TokenSet) {
    assert!(p.at_contextual_kw_ident("has"));
    let m = p.start();
    p.bump_remap(T![has]);
    // let mut is_empty = true;
    delimited_with_recovery(p, EOF, ability, T![,], "expected ability", TokenSet::EMPTY);
    // while !p.at(EOF) && !at_next_item_start(p, extra_set) {
    //     is_empty = false;
    //     if p.at(IDENT) {
    //         let m = p.start();
    //         p.bump(IDENT);
    //         m.complete(p, ABILITY);
    //     } else {
    //         p.error_and_recover_until("expected ability", |p| at_next_item_start(p, extra_set));
    //     }
    //     if !at_next_item_start(p, extra_set) {
    //         p.expect(T![,]);
    //     }
    // }
    // if is_empty {
    //     p.error("expected ability");
    // }
    m.complete(p, ABILITY_LIST);
}

fn at_next_item_start(p: &Parser, extra_set: TokenSet) -> bool {
    at_item_start(p) || p.at_ts(extra_set)
}

pub(crate) const ABILITY_FIRST: TokenSet = TokenSet::new(&[IDENT]);

pub(super) fn enum_(p: &mut Parser, m: Marker) {
    p.bump_remap(T![enum]);

    if !item_name_or_recover(p, struct_enum_recover_at) {
        m.complete(p, ENUM);
        return;
    }
    type_params::opt_type_param_list(p);
    opt_abilities_list(p, vec![T!['{']]);
    if p.at(T!['{']) {
        variant_list(p);
    } else {
        p.error("expected `{`");
    }
    opt_abilities_list_with_semicolon(p);
    m.complete(p, ENUM);
}

pub(crate) fn variant_list(p: &mut Parser) {
    assert!(p.at(T!['{']));
    let m = p.start();
    p.bump(T!['{']);
    while !p.at(EOF) && !p.at(T!['}']) {
        if p.at(T!['{']) {
            error_block(p, "expected enum variant");
            continue;
        }
        let is_curly = variant(p);
        if !p.at(T!['}']) {
            if is_curly {
                p.eat(T![,]);
            } else {
                p.expect(T![,]);
            }
        }
    }
    p.expect(T!['}']);
    m.complete(p, VARIANT_LIST);

    fn variant(p: &mut Parser) -> bool {
        let mut curly_braces = false;
        let m = p.start();
        attributes::outer_attrs(p);
        if p.at(IDENT) {
            name(p);
            match p.current() {
                T!['{'] => {
                    curly_braces = true;
                    named_field_list(p)
                }
                T!['('] => tuple_field_list(p),
                _ => (),
            }
            m.complete(p, VARIANT);
        } else {
            m.abandon(p);
            p.bump_with_error("expected enum variant");
        }
        curly_braces
    }
}

// test record_field_list
// struct S { a: i32, b: f32 }
pub(crate) fn named_field_list(p: &mut Parser) {
    assert!(p.at(T!['{']));
    let m = p.start();
    p.bump(T!['{']);
    while !p.at(T!['}']) && !p.at(EOF) {
        if p.at(T!['{']) {
            error_block(p, "expected field");
            continue;
        }
        named_field(p);
        if !p.at(T!['}']) {
            p.expect(T![,]);
        }
    }
    p.expect(T!['}']);
    m.complete(p, NAMED_FIELD_LIST);
}

fn named_field(p: &mut Parser) {
    let m = p.start();
    // attributes::outer_attrs(p);
    if p.at(IDENT) {
        #[cfg(debug_assertions)]
        let _p = stdx::panic_context::enter(format!("named_field {:?}", p.current_text()));

        name(p);
        let at_colon = p.eat(T![:]);
        if at_colon {
            p.with_recover_token(T![,], |p| types::type_(p));
        } else {
            let rec_set = p.outer_recovery_set().with_recovery_set(T![,] | T![ident]);
            p.error_and_recover_until("expected type annotation", |p| rec_set.contains_current(p));
        }
        m.complete(p, NAMED_FIELD);
    } else {
        m.abandon(p);
        p.bump_with_error("expected named field declaration");
    }
}

fn struct_enum_recover_at(p: &Parser) -> bool {
    p.at(T![<]) || p.at_contextual_kw_ident("has")
}

const TUPLE_FIELD_FIRST: TokenSet =
    types::TYPE_FIRST.union(ATTRIBUTE_FIRST)/*.union(VISIBILITY_FIRST)*/;

fn tuple_field_list(p: &mut Parser) {
    assert!(p.at(T!['(']));
    let m = p.start();
    list(
        p,
        T!['('],
        T![')'],
        T![,],
        || "expected tuple field".into(),
        TUPLE_FIELD_FIRST,
        |p| {
            let em = p.start();
            // attributes::outer_attrs(p);
            if !p.at_ts(types::TYPE_FIRST) {
                p.error("expected a type");
                em.abandon(p);
                return false;
            }
            types::type_(p);
            em.complete(p, TUPLE_FIELD);
            true
        },
    );
    m.complete(p, TUPLE_FIELD_LIST);
}
