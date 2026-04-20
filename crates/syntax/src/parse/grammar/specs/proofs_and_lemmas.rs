use crate::SyntaxKind::{APPLY_LEMMA, LEMMA, PROOF};
use crate::T;
use crate::parse::grammar::expressions::blocks;
use crate::parse::grammar::expressions::blocks::StmtKind;
use crate::parse::grammar::params::fun_param_list;
use crate::parse::grammar::paths::PathMode;
use crate::parse::grammar::type_params::opt_type_param_list;
use crate::parse::grammar::{expressions, name, params, paths};
use crate::parse::parser::Parser;
use crate::parse::token_set::TokenSet;

pub(crate) fn proof(p: &mut Parser) {
    assert!(p.at_contextual_kw_ident("proof"));

    let m = p.start();
    p.bump_remap(T![proof]);
    if p.at(T!['{']) {
        blocks::block_expr(p, StmtKind::Proof);
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
        blocks::block_expr(p, StmtKind::Spec);
    } else {
        p.error("expected block");
    }
    m.complete(p, LEMMA);
}

pub(crate) fn apply_lemma(p: &mut Parser) -> bool {
    if !p.at_contextual_kw_ident("apply") {
        return false;
    }
    let m = p.start();
    p.bump_remap(T![apply]);
    // lemma name
    paths::path(p, Some(PathMode::Type));
    // lemma args
    expressions::value_arg_list(p);
    p.expect(T![;]);
    m.complete(p, APPLY_LEMMA);
    true
}
