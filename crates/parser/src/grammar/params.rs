use crate::grammar::patterns::PATTERN_FIRST;
use super::*;
use crate::grammar::utils::delimited;
use crate::{ts, T};

pub(crate) fn fun_param_list(p: &mut Parser) {
    let list_marker = p.start();
    p.bump(T!['(']);
    while !p.at(EOF) && !p.at(T![')']) {
        if !p.at_ts(PARAM_FIRST) {
            p.error("expected value parameter");
            break;
        }
        param(p);
        if !p.at(T![')']) {
            p.expect(T![,]);
        }
    }
    p.expect(T![')']);
    list_marker.complete(p, PARAM_LIST);
}

fn param(p: &mut Parser) {
    let m = p.start();
    patterns::ident_pat(p);
    if p.at(T![:]) {
        types::ascription(p);
    } else {
        p.error("missing type for function parameter");
    }
    m.complete(p, PARAM);
}

const PARAM_FIRST: TokenSet = patterns::PATTERN_FIRST/*.union(types::TYPE_FIRST)*/;

pub(crate) fn lambda_param_list(p: &mut Parser) -> bool {
    let list_marker = p.start();
    p.bump(T![|]);
    if p.at(T![,]) {
        list_marker.abandon(p);
        return false;
    }
    delimited(
        p,
        T![,],
        || "expected parameter".into(),
        |p| p.at(T![|]),
        ts!(IDENT, T!['_']),
        |p| {
            let m = p.start();
            patterns::pattern(p);
            if p.at(T![:]) {
                types::ascription(p);
            }
            m.complete(p, LAMBDA_PARAM);
            true
        },
    );
    if !p.eat(T![|]) {
        list_marker.abandon_with_rollback(p);
        return false;
    }

    list_marker.complete(p, LAMBDA_PARAM_LIST);
    true
}
