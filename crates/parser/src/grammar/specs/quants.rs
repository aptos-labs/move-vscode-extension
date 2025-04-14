use crate::grammar::specs::predicates::expect_expr;
use crate::grammar::{patterns, types};
use crate::parser::CompletedMarker;
use crate::SyntaxKind::*;
use crate::{Parser, T};

pub(crate) fn is_at_quant_kw(p: &mut Parser) -> bool {
    let at_kw =
        p.at_contextual_kw("forall") || p.at_contextual_kw("exists") || p.at_contextual_kw("choose");
    if at_kw {
        return p.nth_at(1, T![:]) || p.nth_at(1, IDENT);
    }
    false
}

pub(crate) fn forall_expr(p: &mut Parser) -> Option<CompletedMarker> {
    if !p.at_contextual_kw_ident("forall") {
        return None;
    }
    let m = p.start();
    p.bump_remap(T![forall]);
    quant_binding_list(p);
    opt_where_expr(p);
    if p.expect(T![:]) {
        expect_expr(p);
    }
    Some(m.complete(p, FORALL_EXPR))
}

pub(crate) fn exists_expr(p: &mut Parser) -> Option<CompletedMarker> {
    if !p.at_contextual_kw_ident("exists") {
        return None;
    };
    let m = p.start();
    p.bump_remap(T![exists]);
    quant_binding_list(p);
    opt_where_expr(p);
    if p.expect(T![:]) {
        expect_expr(p);
    }
    Some(m.complete(p, EXISTS_EXPR))
}

pub(crate) fn choose_expr(p: &mut Parser) -> Option<CompletedMarker> {
    if !p.at_contextual_kw_ident("choose") {
        return None;
    };
    let m = p.start();
    p.bump_remap(T![choose]);
    if p.at_contextual_kw_ident("min") {
        p.bump_remap(T![min]);
    }
    quant_binding_list(p);
    opt_where_expr(p);
    Some(m.complete(p, CHOOSE_EXPR))
}

pub(crate) fn quant_binding_list(p: &mut Parser) {
    if !p.at(IDENT) {
        return;
    }
    let m = p.start();
    while !p.at(EOF) && !p.at(T![;]) && !p.at(T![:]) && !p.at_contextual_kw_ident("where") {
        if p.at(T![,]) {
            // Recover if an argument is missing and only got a delimiter,
            // e.g. `(a, , b)`.
            // Wrap the erroneous delimiter in an error node so that fixup logic gets rid of it.
            let m = p.start();
            p.error("expected quant binding");
            p.bump(T![,]);
            m.complete(p, ERROR);
            continue;
        }
        if !quant_binding(p) {
            break;
        }
        if !p.eat(T![,]) {
            if p.at(IDENT) && !p.at_contextual_kw_ident("where") {
                p.error(format!("expected {:?}", T![,]));
            } else {
                break;
            }
        }
    }
    m.complete(p, QUANT_BINDING_LIST);
}

pub(crate) fn quant_binding(p: &mut Parser) -> bool {
    if p.at_contextual_kw_ident("where") {
        return false;
    }
    let m = p.start();
    patterns::ident_pat(p);
    match p.current() {
        IDENT if p.at_contextual_kw("in") => {
            p.bump_remap(T![in]);
            expect_expr(p);
        }
        T![:] => {
            types::ascription(p);
        }
        _ => {
            m.abandon_with_rollback(p);
            return false;
        }
    }
    m.complete(p, QUANT_BINDING);
    true
}

fn opt_where_expr(p: &mut Parser) {
    if !p.at_contextual_kw_ident("where") {
        return;
    }
    let m = p.start();
    p.bump_remap(T![where]);
    expect_expr(p);
    m.complete(p, WHERE_EXPR);
}
