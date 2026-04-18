use crate::SyntaxKind::{LEMMA, PROOF, PROOF_BLOCK};
use crate::T;
use crate::parse::grammar::expressions::atom::block_expr;
use crate::parse::grammar::type_params::opt_type_param_list;
use crate::parse::grammar::{name, name_or_recover, params};
use crate::parse::parser::Parser;
use crate::parse::token_set::TokenSet;

pub(crate) fn proof(p: &mut Parser) {
    assert!(p.at_contextual_kw_ident("proof"));

    let m = p.start();
    p.bump_remap(T![proof]);
    if p.at(T!['{']) {
        block_expr(p, true);
    } else {
        p.error("expected a block");
    }
    m.complete(p, PROOF);
}

pub(crate) fn lemma(p: &mut Parser) {
    assert!(p.at_contextual_kw_ident("lemma"));
    let m = p.start();
    p.bump_remap(T![lemma]);
    name(p);
    opt_type_param_list(p);
    if p.at(T!['(']) {
        params::fun_param_list(p);
    } else {
        p.error_and_recover("expected parameters", TokenSet::EMPTY);
    }
    if p.at(T!['{']) {
        block_expr(p, true);
    } else {
        p.error("expected block");
    }
    m.complete(p, LEMMA);
}
