use crate::SyntaxKind::*;
use crate::T;
use crate::parse::grammar::expressions::blocks::{StmtKind, stmt};
use crate::parse::grammar::expressions::{atom, blocks, expr};
use crate::parse::grammar::paths::PathMode;
use crate::parse::grammar::specs::quants;
use crate::parse::grammar::specs::quants::{quant_binding_list, weight};
use crate::parse::grammar::type_params::opt_type_param_list;
use crate::parse::grammar::{expressions, name, params, paths};
use crate::parse::parser::{CompletedMarker, Marker, Parser};
use crate::parse::recovery_set::RecoverySet;
use crate::parse::token_set::TokenSet;

pub(crate) fn proof(p: &mut Parser) {
    assert!(p.at_contextual_kw_ident("proof"));
    let m = p.start();
    p.bump_remap(T![proof]);
    if p.at(T!['{']) {
        p.with_stmt_kind(StmtKind::Proof, blocks::block_expr);
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
        p.with_stmt_kind(StmtKind::Spec, blocks::block_expr);
    } else {
        p.error("expected block");
    }

    if p.at_contextual_kw_ident("proof") {
        proof(p);
    }

    m.complete(p, LEMMA);
}

pub(crate) fn apply_lemma(p: &mut Parser) -> bool {
    if !p.at_contextual_kw_ident("apply") {
        return false;
    }
    let m = p.start();
    p.bump_remap(T![apply]);
    // lemma path
    atom::path_expr(p);
    // lemma args
    expressions::value_arg_list(p);
    p.expect(T![;]);
    m.complete(p, APPLY_LEMMA);
    true
}

pub(crate) fn forall_apply_lemma(p: &mut Parser) -> bool {
    if !p.at_contextual_kw_ident("forall") {
        return false;
    }
    let m = p.start();
    p.bump_remap(T![forall]);
    quant_binding_list(
        p,
        RecoverySet::new()
            .with_kw("apply")
            .with_recovery_token(T!['['].into()),
    );
    if p.at(T!['{']) {
        quants::quant_trigger_list(p);
    }
    if p.at(T!['[']) {
        weight(p);
    }
    let has_lemma = apply_lemma(p);
    if !has_lemma {
        p.error("expected apply");
    }
    m.complete(p, FORALL_APPLY_LEMMA);
    true
}

pub(crate) fn post_stmt(p: &mut Parser) -> bool {
    if !p.at_contextual_kw_ident("post") {
        return false;
    }
    let m = p.start();
    p.bump_remap(T![post]);
    stmt(p);
    m.complete(p, POST_STMT);
    true
}

pub(crate) fn split_stmt(p: &mut Parser) -> bool {
    if !p.at_contextual_kw_ident("split") {
        return false;
    }
    let m = p.start();
    p.bump_remap(T![split]);
    expr(p);
    p.eat(T![;]);
    m.complete(p, SPLIT_STMT);
    true
}
