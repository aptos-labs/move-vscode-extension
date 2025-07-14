// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use super::*;
use crate::parse::grammar::paths::PathMode;
use crate::parse::grammar::patterns::pat;
use crate::parse::grammar::specs::{opt_spec_block_expr, spec_block_expr};
use crate::parse::grammar::{any_address, paths};
use crate::parse::recovery_set::RecoverySet;
use crate::parse::token_set::TokenSet;
use crate::ts;
use std::ops::ControlFlow::Break;

pub(crate) fn literal(p: &mut Parser) -> Option<CompletedMarker> {
    let m = p.start();
    match p.current() {
        // 0x1::m
        INT_NUMBER if p.nth_at(1, T![::]) => {
            m.abandon(p);
            return None;
        }
        T![@] => {
            let m = p.start();
            p.bump(T![@]);
            any_address(p);
            m.complete(p, ADDRESS_LIT);
        }
        INT_NUMBER | BYTE_STRING | HEX_STRING | T![true] | T![false] => {
            p.bump_any();
        }
        BAD_CHARACTER => {
            m.abandon(p);
            p.error_and_bump("unexpected character");
            return None;
        }
        _ => {
            m.abandon(p);
            return None;
        }
    }
    Some(m.complete(p, LITERAL))
}

pub(crate) const LITERAL_FIRST: TokenSet =
    TokenSet::new(&[T![true], T![false], INT_NUMBER, T![@], BYTE_STRING, HEX_STRING]);

pub(crate) const KW_EXPR_FIRST: TokenSet = TokenSet::new(&[
    T![if],
    T![loop],
    T![while],
    T![for],
    // never matches
    // T![match],
    T![return],
    T![continue],
    T![break],
    T![move],
    T![copy],
    T![abort],
    // label
    QUOTE_IDENT,
]);

// E.g. for after the break in `if break {}`, this should not match
pub(crate) const ATOM_EXPR_FIRST: TokenSet = LITERAL_FIRST
    .union(paths::PATH_FIRST)
    .union(KW_EXPR_FIRST)
    .union(TokenSet::new(&[T!['('], T!['{'], T!['['], T![|], QUOTE_IDENT]));

pub(crate) fn atom_expr(p: &mut Parser) -> Option<(CompletedMarker, BlockLike)> {
    if let Some(m) = literal(p) {
        return Some((m, BlockLike::NotBlock));
    }
    if p.at_contextual_kw_ident("vector") && p.nth_at_ts(1, ts!(T!['['], T![<])) {
        let cm = vector_lit_expr(p);
        return Some((cm, BlockLike::NotBlock));
    }
    if p.at_contextual_kw("match") && p.nth_at(1, T!['(']) {
        let opt_cm = match_expr(p);
        // can be `match()` function call instead
        if let Some(cm) = opt_cm {
            return Some((cm, BlockLike::Block));
        }
    }
    if p.at_contextual_kw_ident("assert") && p.nth_at(1, T![!]) {
        let cm = assert_macro_expr(p);
        return Some((cm, BlockLike::NotBlock));
    }
    if p.at_contextual_kw("for") && p.nth_at(1, T!['(']) {
        let cm = for_expr(p, None);
        return Some((cm, BlockLike::Block));
    }
    if paths::is_path_start(p) {
        let cm = path_expr(p);
        return Some((cm, BlockLike::NotBlock));
    }
    let done = match p.current() {
        T!['('] => paren_or_tuple_or_annotated_expr(p),
        T![spec] => spec_block_expr(p),
        //     T![|] => closure_expr(p),
        T![if] => if_expr(p),
        T![loop] => loop_expr(p, None),
        T![while] => while_expr(p, None),
        QUOTE_IDENT if p.nth(1) == T![:] => {
            let m = p.start();
            label_decl(p);
            match p.current() {
                T![loop] => loop_expr(p, Some(m)),
                IDENT if p.at_contextual_kw("for") => for_expr(p, Some(m)),
                T![while] => while_expr(p, Some(m)),
                _ => {
                    p.error("expected a loop");
                    m.complete(p, ERROR);
                    return None;
                }
            }
        }
        T!['{'] => block_expr(p, false),
        T![return] => return_expr(p),
        T![abort] => abort_expr(p),
        T![continue] => continue_expr(p),
        T![break] => break_expr(p),
        _ => {
            // p.error("expected expression");
            // p.error_and_bump_any("expected expression");
            // p.err_and_bump("expected expression", EXPR_RECOVERY_SET);
            return None;
        }
    };
    let blocklike = if BlockLike::is_blocklike(done.kind()) {
        BlockLike::Block
    } else {
        BlockLike::NotBlock
    };
    Some((done, blocklike))
}

fn path_expr(p: &mut Parser) -> CompletedMarker {
    let m = p.start();
    paths::path(p, Some(PathMode::Expr));
    let cm = match p.current() {
        T!['{'] /*if !r.forbid_structs*/ => {
            struct_lit_field_list(p);
            m.complete(p, STRUCT_LIT)
        }
        _ => { m.complete(p, PATH_EXPR) }
    };
    cm
}

fn vector_lit_expr(p: &mut Parser) -> CompletedMarker {
    // vector[1, 2]
    let m = p.start();
    p.bump(IDENT);
    type_args::opt_path_type_arg_list(p, PathMode::Type);
    if p.at(T!['[']) {
        p.bump(T!['[']);
        delimited_with_recovery(p, expr, T![,], "expected expression", Some(T![']']));
        p.expect(T![']']);
    } else {
        p.error_and_recover("expected '['", STMT_FIRST);
    }
    m.complete(p, VECTOR_LIT_EXPR)
}

fn match_expr(p: &mut Parser) -> Option<CompletedMarker> {
    let m = p.start();
    p.bump_remap(T![match]);

    p.bump(T!['(']);
    expr(p);
    p.expect(T![')']);

    if !p.at(T!['{']) {
        // this is a match() function
        m.abandon_with_rollback(p);
        return None;
    }

    match_arm_list(p);
    Some(m.complete(p, MATCH_EXPR))
}

fn assert_macro_expr(p: &mut Parser) -> CompletedMarker {
    assert!(p.at_contextual_kw_ident("assert"));
    let m = p.start();
    p.bump(IDENT);
    p.bump(T![!]);
    if p.at(T!['(']) {
        value_arg_list(p);
    } else {
        // emit an error when argument list is missing
        p.error("expected argument list");
    }
    m.complete(p, ASSERT_MACRO_EXPR)
}

pub(crate) fn call_expr(p: &mut Parser, lhs: CompletedMarker) -> CompletedMarker {
    assert!(p.at(T!['(']));
    let m = lhs.precede(p);
    value_arg_list(p);
    m.complete(p, CALL_EXPR)
}

fn paren_or_tuple_or_annotated_expr(p: &mut Parser) -> CompletedMarker {
    assert!(p.at(T!['(']));
    let m = p.start();
    p.bump(T!['(']);
    // ();
    if p.at(T![')']) {
        p.bump(T![')']);
        return m.complete(p, UNIT_EXPR);
    }

    let mut outer = true;
    let mut saw_comma = false;
    let mut saw_expr = false;
    let mut saw_type_annotation = false;
    p.iterate_to_EOF(T![')'], |p| {
        saw_expr = true;
        if !expr(p) {
            return Break(());
        }
        // try for `(a: u8)` annotated expr
        if outer {
            if p.at(T![:]) {
                types::type_annotation(p);
                saw_type_annotation = true;
                return Break(());
            }
            outer = false;
        }
        if !p.at(T![')']) {
            saw_comma = true;
            p.expect(T![,]);
        }
        Continue(())
    });

    p.expect(T![')']);
    if saw_type_annotation {
        return m.complete(p, ANNOTATED_EXPR);
    }
    m.complete(
        p,
        if saw_expr && !saw_comma {
            PAREN_EXPR
        } else {
            TUPLE_EXPR
        },
    )
}

fn if_expr(p: &mut Parser) -> CompletedMarker {
    assert!(p.at(T![if]));
    let m = p.start();
    p.bump(T![if]);
    condition(p);
    block_or_inline_expr(p, false);
    if p.at(T![else]) {
        p.bump(T![else]);
        // `else if /*expr*/` parsed as inline expr - `else (if /*expr*/)`
        block_or_inline_expr(p, false);
    }
    m.complete(p, IF_EXPR)
}

fn label_decl(p: &mut Parser) {
    assert!(p.at(QUOTE_IDENT) && p.nth(1) == T![:]);
    let m = p.start();
    p.bump(QUOTE_IDENT);
    p.bump(T![:]);
    m.complete(p, LABEL_DECL);
}

fn loop_expr(p: &mut Parser, m: Option<Marker>) -> CompletedMarker {
    assert!(p.at(T![loop]));
    let m = m.unwrap_or_else(|| p.start());
    p.bump(T![loop]);
    block_or_inline_expr(p, false);
    m.complete(p, LOOP_EXPR)
}

fn for_expr(p: &mut Parser, m: Option<Marker>) -> CompletedMarker {
    assert!(p.at(IDENT) && p.at_contextual_kw("for"));
    let m = m.unwrap_or_else(|| p.start());
    p.bump_remap(T![for]);
    for_condition(p);
    block_or_inline_expr(p, false);
    m.complete(p, FOR_EXPR)
}

fn for_condition(p: &mut Parser) {
    // todo: recovery
    let m = p.start();
    p.expect(T!['(']);
    patterns::ident_pat_or_recover(p);
    if p.at(IDENT) && p.at_contextual_kw("in") {
        p.bump_remap(T![in]);
        expr(p);
    } else {
        p.error_and_recover("expected 'in'", EXPR_FIRST | T![')']);
        // p.error_and_recover_until_ts("expected 'in'", EXPR_FIRST.union(ts!(T![')'])));
    }
    opt_spec_block_expr(p);
    p.expect(T![')']);
    m.complete(p, FOR_CONDITION);
}

fn while_expr(p: &mut Parser, m: Option<Marker>) -> CompletedMarker {
    assert!(p.at(T![while]));
    let m = m.unwrap_or_else(|| p.start());
    p.bump(T![while]);
    condition(p);
    block_or_inline_expr(p, false);
    opt_spec_block_expr(p);
    m.complete(p, WHILE_EXPR)
}

pub(crate) fn condition(p: &mut Parser) {
    let m = p.start();
    p.expect(T!['(']);
    let r = Restrictions {
        forbid_structs: true,
        prefer_stmt: false,
    };
    expr_bp(p, None, r, 1);
    p.expect(T![')']);

    m.complete(p, CONDITION);
}

pub(crate) fn match_arm_list(p: &mut Parser) {
    assert!(p.at(T!['{']));
    let m = p.start();
    p.eat(T!['{']);
    // it's an expr block too
    p.reset_recovery_set(|p| {
        p.iterate_to_EOF(T!['}'], |p| {
            match_arm(p, TokenSet::EMPTY);
            Continue(())
        });
    });
    p.expect(T!['}']);
    m.complete(p, MATCH_ARM_LIST);
}

fn match_arm(p: &mut Parser, recovery_set: TokenSet) -> bool {
    let m = p.start();
    p.with_recovery_token_set(T![=>] | T!['}'], pat);
    // if !is_pat {
    //     m.abandon(p);
    //     return false;
    // }
    if p.at(T![if]) {
        let m = p.start();
        p.bump(T![if]);
        expr(p);
        m.complete(p, MATCH_GUARD);
    }

    // if !p.expect(T![=>]) {
    //     m.abandon(p);
    //     return false;
    // }
    if !p.at(T![=>]) {
        p.error_and_recover("expected '=>'", recovery_set);
        m.abandon(p);
        return false;
    }
    p.bump(T![=>]);

    let blocklike = match stmt_expr(p) {
        Some((_, blocklike)) => blocklike,
        None => BlockLike::NotBlock,
    };

    if !p.eat(T![,]) && !blocklike.is_block() && !p.at(T!['}']) {
        p.error("expected `,`");
    }
    m.complete(p, MATCH_ARM);
    true
}

fn match_guard(p: &mut Parser) -> CompletedMarker {
    assert!(p.at(T![if]));
    let m = p.start();
    p.bump(T![if]);
    expr(p);
    m.complete(p, MATCH_GUARD)
}

pub(crate) fn block_or_inline_expr(p: &mut Parser, is_spec: bool) {
    if p.at(T!['{']) {
        block_expr(p, is_spec);
    } else {
        inline_expr(p);
    }
}

pub(crate) fn block_expr(p: &mut Parser, is_spec: bool) -> CompletedMarker {
    assert!(p.at(T!['{']));
    // we're in new block, we can't use recovery set rules from before
    p.reset_recovery_set(|p| {
        let m = p.start();
        stmt_list(p, is_spec);
        m.complete(p, BLOCK_EXPR)
    })
}

pub(crate) fn inline_expr(p: &mut Parser) {
    assert!(!p.at(T!['{']));
    let m = p.start();
    expr(p);
    m.complete(p, INLINE_EXPR);
}

fn stmt_list(p: &mut Parser, is_spec: bool) {
    assert!(p.at(T!['{']));
    p.bump(T!['{']);
    expr_block_contents(p, is_spec);
    p.expect(T!['}']);
}

fn return_expr(p: &mut Parser) -> CompletedMarker {
    assert!(p.at(T![return]));
    let m = p.start();
    p.bump(T![return]);
    if p.at_ts(EXPR_FIRST) {
        expr(p);
    }
    m.complete(p, RETURN_EXPR)
}

fn abort_expr(p: &mut Parser) -> CompletedMarker {
    assert!(p.at(T![abort]));
    let m = p.start();
    p.bump(T![abort]);
    if p.at_ts(EXPR_FIRST) {
        expr(p);
    }
    m.complete(p, ABORT_EXPR)
}

fn continue_expr(p: &mut Parser) -> CompletedMarker {
    assert!(p.at(T![continue]));
    let m = p.start();
    p.bump(T![continue]);
    opt_label(p);
    m.complete(p, CONTINUE_EXPR)
}

fn break_expr(p: &mut Parser) -> CompletedMarker {
    assert!(p.at(T![break]));
    let m = p.start();
    p.bump(T![break]);
    opt_label(p);
    //     if break {}
    //     while break {}
    //     for i in break {}
    //     match break {}
    // }
    if p.at_ts(EXPR_FIRST)
    /*&& !(false && p.at(T!['{']))*/
    {
        expr(p);
    }
    m.complete(p, BREAK_EXPR)
}

fn opt_label(p: &mut Parser) {
    if p.at(QUOTE_IDENT) {
        let m = p.start();
        p.eat(QUOTE_IDENT);
        m.complete(p, LABEL);
    }
}

// pub(crate) const EXPR_FIRST: TokenSet = EXPR_LHS_FIRST;

pub(crate) const IDENT_FIRST: TokenSet = TokenSet::new(&[IDENT]);
