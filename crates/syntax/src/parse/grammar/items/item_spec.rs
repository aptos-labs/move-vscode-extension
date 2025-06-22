use crate::parse::grammar::attributes::ATTRIBUTE_FIRST;
use crate::parse::grammar::expressions::atom::block_expr;
use crate::parse::grammar::items::{at_item_start, fun, item_start_rset};
use crate::parse::grammar::patterns::ident_or_wildcard_pat_with_recovery;
use crate::parse::grammar::utils::delimited_with_recovery;
use crate::parse::grammar::{name_ref, name_ref_or_recover, patterns, type_params, types};
use crate::parse::parser::{Marker, Parser};
use crate::parse::token_set::TokenSet;
use crate::SyntaxKind::*;
use crate::{ts, T};

pub(crate) fn item_spec(p: &mut Parser, m: Marker) {
    if p.at(T![module]) {
        p.bump(T![module]);
    } else {
        p.with_recovery_set(item_start_rset().with_token_set(T!['{']), item_spec_signature);
    }
    block_expr(p, true);
    m.complete(p, ITEM_SPEC);
}

fn item_spec_signature(p: &mut Parser) {
    let ref_m = p.start();
    let res = name_ref_or_recover(p);
    // let res = name_ref_or_bump_until(p, |p| at_item_start(p) || p.at(T!['{']));
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
        p.with_recovery_token(T!['{'], fun::opt_ret_type);
    }
}

fn item_spec_type_param_list(p: &mut Parser) {
    assert!(p.at(T![<]));
    let m = p.start();
    p.bump(T![<]);
    delimited_with_recovery(
        p,
        item_spec_type_param,
        T![,],
        "expected type parameter",
        Some(T![>]),
    );
    p.expect(T![>]);
    m.complete(p, ITEM_SPEC_TYPE_PARAM_LIST);
}

fn item_spec_type_param(p: &mut Parser) -> bool {
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
            p.bump_with_error("expected type parameter");
            return false;
        }
    }
    true
}

pub(crate) fn item_spec_param_list(p: &mut Parser) {
    let list_marker = p.start();
    p.bump(T!['(']);
    delimited_with_recovery(
        p,
        item_spec_param,
        T![,],
        "expected value parameter",
        Some(T![')']),
    );
    // while !p.at(EOF) && !p.at(T![')']) {
    //     if p.at_ts(ITEM_SPEC_PARAM_FIRST) {
    //         item_spec_param(p);
    //     } else {
    //         p.error_and_recover("expected value parameter", ITEM_SPEC_PARAM_RECOVERY_SET.into());
    //         // p.error_and_recover_until_ts("expected value parameter", ITEM_SPEC_PARAM_RECOVERY_SET);
    //     }
    //     if !p.at(T![')']) {
    //         p.expect(T![,]);
    //     }
    // }
    p.expect(T![')']);
    list_marker.complete(p, ITEM_SPEC_PARAM_LIST);
}

fn item_spec_param(p: &mut Parser) -> bool {
    let m = p.start();
    let is_ident = ident_or_wildcard_pat_with_recovery(p);
    if is_ident {
        if p.at(T![:]) {
            types::ascription(p);
        } else {
            p.error_and_recover("missing type for parameter", TokenSet::EMPTY.into());
        }
    }

    // if p.at(T![:]) {
    //     types::ascription(p);
    // } else {
    //     p.error_and_recover("missing type for parameter", TokenSet::EMPTY.into());
    //     // p.error_and_recover_until_ts("missing type for parameter", ITEM_SPEC_PARAM_RECOVERY_SET);
    // }

    m.complete(p, ITEM_SPEC_PARAM);
    true
}

const TYPE_PARAM_RECOVERY_SET: TokenSet = TokenSet::new(&[T![,], T![>]]);

const ITEM_SPEC_PARAM_RECOVERY_SET: TokenSet = ts!(T![')'], T![,]);
const ITEM_SPEC_PARAM_FIRST: TokenSet = ts!(IDENT, T!['_']);
