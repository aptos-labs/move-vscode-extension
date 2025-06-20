use super::*;
use crate::T;

pub(crate) fn fun_param_list(p: &mut Parser) {
    let m = p.start();
    p.bump(T!['(']);
    while !p.at(EOF) && !p.at(T![')']) {
        if p.at_ts(PARAM_FIRST) {
            p.with_recover_ts(ts!(T![,], T![')']), param);
            // param(p);
        } else {
            p.error_and_recover_until_ts("expected value parameter", PARAM_RECOVERY_SET);
        }
        if !p.at(T![')']) {
            p.expect(T![,]);
        }
    }
    p.expect(T![')']);
    m.complete(p, PARAM_LIST);
}

fn param(p: &mut Parser) {
    let m = p.start();
    patterns::ident_or_wildcard_pat_or_recover(p, PARAM_RECOVERY_SET);
    if p.at(T![:]) {
        p.with_recover_t(T![,], types::ascription);
        // types::ascription(p);
    } else {
        p.error_and_recover_until_ts("missing type for function parameter", PARAM_RECOVERY_SET);
    }
    m.complete(p, PARAM);
}

const PARAM_FIRST: TokenSet = ts!(IDENT, T!['_']);
const PARAM_RECOVERY_SET: TokenSet = ts!(T![')'], T![,]);
