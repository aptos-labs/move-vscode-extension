// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::SyntaxKind::*;
use crate::T;
use crate::parse::grammar::expressions::atom::block_expr;
use crate::parse::grammar::expressions::{expr, expr_block_contents, stmt_expr, stmts};
use crate::parse::grammar::specs::predicates::expect_expr;
use crate::parse::grammar::utils::delimited_with_recovery;
use crate::parse::grammar::{patterns, types};
use crate::parse::parser::{CompletedMarker, Parser};
use crate::parse::recovery_set::RecoverySet;
use std::ops::ControlFlow;
use std::ops::ControlFlow::{Break, Continue};

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
    if p.at(T!['{']) {
        quant_trigger_list(p);
    }
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
    if p.at(T!['{']) {
        quant_trigger_list(p);
    }
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
    quant_binding(p);
    // quant_binding_list(p);
    opt_where_expr(p);
    Some(m.complete(p, CHOOSE_EXPR))
}

pub(crate) fn quant_binding_list(p: &mut Parser) {
    let m = p.start();
    let stop_at = RecoverySet::new()
        // end of statement
        .with_token_set(T![;])
        // quantifier hint
        .with_token_set(T!['{'])
        // end of quant bindings
        .with_kw("where")
        .with_token_set(T![:]);
    p.with_recovery_set(stop_at, |p| {
        delimited_with_recovery(p, quant_binding, T![,], "expected quant binding", None)
    });
    m.complete(p, QUANT_BINDING_LIST);
}

pub(crate) fn quant_binding(p: &mut Parser) -> bool {
    if p.at_contextual_kw_ident("where") {
        return false;
    }
    let m = p.start();
    patterns::ident_pat_or_recover(p);
    match p.current() {
        T![:] => {
            types::type_annotation(p);
        }
        IDENT if p.at_contextual_kw("in") => {
            p.bump_remap(T![in]);
            expect_expr(p);
        }
        _ => {
            m.abandon_with_rollback(p);
            return false;
        }
    }
    m.complete(p, QUANT_BINDING);
    true
}

fn quant_trigger_list(p: &mut Parser) {
    assert!(p.at(T!['{']));
    // we're in new block, we can't use recovery set rules from before
    let m = p.start();
    p.bump(T!['{']);
    p.reset_recovery_set(|p| {
        delimited_with_recovery(p, expr, T![,], "expected expr", Some(T!['}']));
    });
    p.expect(T!['}']);
    m.complete(p, QUANT_TRIGGER_LIST);
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
