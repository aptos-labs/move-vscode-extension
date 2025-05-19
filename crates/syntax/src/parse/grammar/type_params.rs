use crate::parse::grammar::attributes::ATTRIBUTE_FIRST;
use crate::parse::grammar::utils::list;
use crate::parse::grammar::{ability, name};
use crate::parse::parser::Parser;
use crate::parse::token_set::TokenSet;
use crate::SyntaxKind::*;
use crate::T;

pub(super) fn opt_type_param_list(p: &mut Parser<'_>) {
    if p.at(T![<]) {
        type_param_list(p);
    }
}

fn type_param_list(p: &mut Parser<'_>) {
    assert!(p.at(T![<]));
    let m = p.start();
    list(
        p,
        T![<],
        T![>],
        T![,],
        || "expected generic parameter".into(),
        TYPE_PARAM_FIRST.union(ATTRIBUTE_FIRST),
        |p| type_param(p),
    );

    m.complete(p, TYPE_PARAM_LIST);
}

fn type_param(p: &mut Parser<'_>) -> bool {
    let m = p.start();
    if p.at_contextual_kw_ident("phantom") {
        p.bump_remap(T![phantom]);
    }
    match p.current() {
        IDENT => {
            name(p);
            if p.at(T![:]) {
                p.bump(T![:]);
                ability_bound_list(p);
            }
            m.complete(p, TYPE_PARAM);
        }
        _ => {
            m.abandon(p);
            p.error_and_bump_any("expected type parameter");
            return false;
        }
    }
    true
}

// pub(crate) fn ability_bounds(p: &mut Parser) {
//     assert!(p.at(T![:]));
//     p.bump(T![:]);
//     ability_bound_list(p);
// }

pub(crate) fn ability_bound_list(p: &mut Parser) {
    let m = p.start();
    while ability(p) {
        if !p.eat(T![+]) {
            break;
        }
    }
    m.complete(p, ABILITY_BOUND_LIST);
}

const TYPE_PARAM_FIRST: TokenSet = TokenSet::new(&[IDENT, T![phantom]]);
