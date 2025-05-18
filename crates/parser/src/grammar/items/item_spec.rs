use crate::grammar::attributes::ATTRIBUTE_FIRST;
use crate::grammar::expressions::atom::block_expr;
use crate::grammar::items::{fun, item_start};
use crate::grammar::utils::list;
use crate::grammar::{name_ref, name_ref_or_bump_until, patterns, type_params, types};
use crate::parser::Marker;
use crate::token_set::TokenSet;
use crate::SyntaxKind::*;
use crate::{ts, Parser, T};

pub(crate) fn item_spec(p: &mut Parser, m: Marker) {
    if p.at(T![module]) {
        p.bump(T![module]);
    } else {
        let ref_exists = {
            let ref_m = p.start();
            let res = name_ref_or_bump_until(p, item_start);
            if res {
                ref_m.complete(p, ITEM_SPEC_REF);
            } else {
                ref_m.abandon(p);
            }
            res
        };
        if !ref_exists {
            m.complete(p, ITEM_SPEC);
            return;
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

// test_err generic_param_list_recover
// fn f<T: Clone,, U:, V>() {}
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
        if !p.at(IDENT) {
            p.error("expected value parameter");
            break;
        }
        // if !p.at_ts(ITEM_SPEC_PARAM_FIRST) {
        //     p.error("expected value parameter");
        //     break;
        // }
        item_spec_param(p);
        if !p.at(T![')']) {
            p.expect(T![,]);
        }
    }
    p.expect(T![')']);
    list_marker.complete(p, ITEM_SPEC_PARAM_LIST);
}

fn item_spec_param(p: &mut Parser) {
    let m = p.start();
    patterns::pattern(p);
    if p.at(T![:]) {
        types::ascription(p);
    } else {
        p.error("missing type for parameter");
    }
    m.complete(p, ITEM_SPEC_PARAM);
}

const ITEM_SPEC_PARAM_FIRST: TokenSet = ts!(IDENT);
