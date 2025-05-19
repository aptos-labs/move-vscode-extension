use crate::parse::grammar::attributes::ATTRIBUTE_FIRST;
use crate::parse::grammar::expressions::atom::block_expr;
use crate::parse::grammar::items::{fun, item_start};
use crate::parse::grammar::patterns::ident_or_wildcard_pat_or_recover;
use crate::parse::grammar::utils::list;
use crate::parse::grammar::{name_ref, name_ref_or_bump_until, patterns, type_params, types};
use crate::parse::parser::{Marker, Parser};
use crate::parse::token_set::TokenSet;
use crate::SyntaxKind::*;
use crate::{ts, T};

pub(crate) fn item_spec(p: &mut Parser, m: Marker) {
    if p.at(T![module]) {
        p.bump(T![module]);
    } else {
        let ref_m = p.start();
        let res = name_ref_or_bump_until(p, |p| item_start(p) || p.at(T!['{']));
        if res {
            ref_m.complete(p, ITEM_SPEC_REF);
        } else {
            ref_m.abandon(p);
        }
        if p.at(T![<]) {
            item_spec_type_param_list(p);
        }
        if p.at(T!['(']) {
            item_spec_param_list(p);
            fun::opt_ret_type(p);
        }
    }
    block_expr(p, true);
    m.complete(p, ITEM_SPEC);
}

fn item_spec_type_param_list(p: &mut Parser<'_>) {
    assert!(p.at(T![<]));
    let m = p.start();
    list(
        p,
        T![<],
        T![>],
        T![,],
        || "expected type parameter".into(),
        ts!(IDENT).union(ATTRIBUTE_FIRST),
        |p| item_spec_type_param(p),
    );
    m.complete(p, ITEM_SPEC_TYPE_PARAM_LIST);
}

fn item_spec_type_param(p: &mut Parser<'_>) -> bool {
    let m = p.start();
    if p.at_contextual_kw_ident("phantom") {
        p.bump_remap(T![phantom]);
    }
    match p.current() {
        IDENT => {
            name_ref(p);
            if p.at(T![:]) {
                p.bump(T![:]);
                type_params::ability_bound_list(p);
            }
            m.complete(p, ITEM_SPEC_TYPE_PARAM);
        }
        _ => {
            m.abandon(p);
            p.error_and_bump_any("expected type parameter");
            return false;
        }
    }
    true
}

pub(crate) fn item_spec_param_list(p: &mut Parser) {
    let list_marker = p.start();
    p.bump(T!['(']);
    while !p.at(EOF) && !p.at(T![')']) {
        if p.at_ts(ITEM_SPEC_PARAM_FIRST) {
            item_spec_param(p);
        } else {
            p.error_and_bump_until_ts("expected value parameter", ITEM_SPEC_PARAM_RECOVERY_SET);
        }
        if !p.at(T![')']) {
            p.expect(T![,]);
        }
    }
    p.expect(T![')']);
    list_marker.complete(p, ITEM_SPEC_PARAM_LIST);
}

fn item_spec_param(p: &mut Parser) {
    let m = p.start();
    ident_or_wildcard_pat_or_recover(p, ITEM_SPEC_PARAM_RECOVERY_SET);

    if p.at(T![:]) {
        types::ascription(p);
    } else {
        p.error_and_bump_until_ts("missing type for parameter", ITEM_SPEC_PARAM_RECOVERY_SET);
    }

    m.complete(p, ITEM_SPEC_PARAM);
}

const ITEM_SPEC_PARAM_RECOVERY_SET: TokenSet = ts!(T![')'], T![,]);
const ITEM_SPEC_PARAM_FIRST: TokenSet = ts!(IDENT, T!['_']);
