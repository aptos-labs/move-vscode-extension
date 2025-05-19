use crate::parse::grammar::attributes::ATTRIBUTE_FIRST;
use crate::parse::grammar::utils::list;
use crate::parse::grammar::{ability, name, name_or_recover, patterns, types};
use crate::parse::parser::Parser;
use crate::parse::token_set::TokenSet;
use crate::SyntaxKind::*;
use crate::{ts, T};

pub(super) fn opt_type_param_list(p: &mut Parser<'_>) {
    if p.at(T![<]) {
        type_param_list(p);
    }
}

fn type_param_list(p: &mut Parser<'_>) {
    let m = p.start();
    p.bump(T![<]);

    while !p.at(EOF) && !p.at(T![>]) {
        if p.at(IDENT) || p.at_contextual_kw_ident("phantom") {
            type_param(p);
        } else {
            p.error_and_bump_until_ts("expected type parameter", TYPE_PARAM_RECOVERY_SET);
        }
        if !p.at(T![>]) {
            p.expect(T![,]);
        }
    }
    p.expect(T![>]);
    m.complete(p, TYPE_PARAM_LIST);
}

fn type_param(p: &mut Parser<'_>) {
    let m = p.start();

    let mut has_phantom = false;
    if p.at_contextual_kw_ident("phantom") {
        has_phantom = true;
        p.bump_remap(T![phantom]);
    }
    let has_name = name_or_recover(p, |p| p.at_ts(TYPE_PARAM_RECOVERY_SET));
    if has_name {
        if p.at(T![:]) {
            ability_bound_list_recover_until(p, TYPE_PARAM_RECOVERY_SET);
        }
    }

    if has_name || has_phantom {
        m.complete(p, TYPE_PARAM);
    } else {
        m.abandon(p);
    }
}

pub(crate) fn ability_bound_list_recover_until(p: &mut Parser, recovery_set: TokenSet) {
    assert!(p.at(T![:]));
    let m = p.start();
    p.bump(T![:]);
    while !p.at(EOF) && !p.at_ts(recovery_set) {
        if !ability(p) {
            p.error_and_bump_until_ts("expected ability", recovery_set.union(ts!(T![+])));
        }
        if !p.at_ts(recovery_set) {
            p.eat(T![+]);
        }
    }
    m.complete(p, ABILITY_BOUND_LIST);
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

const TYPE_PARAM_RECOVERY_SET: TokenSet = TokenSet::new(&[T![,], T![>]]);
