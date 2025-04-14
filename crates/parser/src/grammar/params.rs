use super::*;
use crate::grammar::patterns::PATTERN_FIRST;
use crate::grammar::utils::{delimited, list_with_recover};
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
