use super::*;
use crate::parse::grammar::utils::delimited_with_recovery;
use crate::T;

pub(crate) fn fun_param_list(p: &mut Parser) {
    let m = p.start();
    p.bump(T!['(']);
    delimited_with_recovery(p, T![')'], param, T![,], "expected value parameter");
    p.expect(T![')']);
    m.complete(p, PARAM_LIST);
}

fn param(p: &mut Parser) -> bool {
    let m = p.start();
    let is_ident = patterns::ident_or_wildcard_pat(p);
    if !is_ident {
        m.abandon(p);
        return false;
    }
    if p.expect_with_error(T![:], "expected type annotation") {
        p.with_recover_token(T![,], types::type_);
    }
    m.complete(p, PARAM);
    true
}

const PARAM_FIRST: TokenSet = ts!(IDENT, T!['_']);
const PARAM_RECOVERY_SET: TokenSet = ts!(T![')'], T![,]);
