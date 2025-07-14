// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::SyntaxKind::*;
use crate::parse::grammar::expressions::atom::call_expr;
use crate::parse::grammar::items::{at_item_start, fun, use_item};
use crate::parse::grammar::lambdas::lambda_param_list;
use crate::parse::grammar::patterns::STMT_KEYWORDS_LIST;
use crate::parse::grammar::specs::predicates::{pragma_stmt, spec_predicate, update_stmt};
use crate::parse::grammar::specs::quants::{choose_expr, exists_expr, forall_expr, is_at_quant_kw};
use crate::parse::grammar::specs::schemas::{
    apply_schema, global_variable, include_schema, schema_field,
};
use crate::parse::grammar::utils::delimited_with_recovery;
use crate::parse::grammar::{attributes, error_block, name_ref, patterns, type_args, types};
use crate::parse::parser::{CompletedMarker, Marker, Parser};
use crate::parse::token_set::TokenSet;
use crate::{SyntaxKind, T, ts};
use std::io::Read;
use std::iter;
use std::ops::ControlFlow::Continue;

pub(crate) mod atom;
pub(crate) mod stmts;

pub(crate) fn expr(p: &mut Parser) -> bool {
    let r = Restrictions {
        forbid_structs: false,
        prefer_stmt: false,
    };
    expr_bp(p, None, r, 1).is_some()
}

// Parses expression with binding power of at least bp.
pub(crate) fn expr_bp(
    p: &mut Parser,
    stmt_m: Option<Marker>,
    mut r: Restrictions,
    bp: u8,
) -> Option<(CompletedMarker, BlockLike)> {
    let stmt_m = stmt_m.unwrap_or_else(|| p.start());
    let mut lhs = match lhs(p, r) {
        Some((lhs, blocklike)) => {
            let lhs = lhs.extend_to(p, stmt_m);
            if r.prefer_stmt && blocklike.is_block() {
                return Some((lhs, BlockLike::Block));
            }
            lhs
        }
        None => {
            stmt_m.abandon(p);
            return None;
        }
    };

    loop {
        let is_range = p.at(T![..]) /*|| p.at(T![..=])*/;
        let (op_bp, op) = current_op(p);
        if op_bp < bp {
            break;
        }
        if p.at(T![as]) {
            lhs = cast_expr(p, lhs);
            continue;
        }
        if p.at_contextual_kw_ident("is") {
            lhs = is_expr(p, lhs);
            continue;
        }
        let m = lhs.precede(p);
        p.bump(op);

        r = Restrictions { prefer_stmt: false, ..r };

        if is_range {
            let has_trailing_expression = p.at_ts(EXPR_FIRST) && !(r.forbid_structs && p.at(T!['{']));
            if !has_trailing_expression {
                // no RHS
                lhs = m.complete(p, RANGE_EXPR);
                break;
            }
        }

        let cm = expr_bp(p, None, Restrictions { prefer_stmt: false, ..r }, op_bp + 1);
        if cm.is_none() {
            p.error("expected expression");
        }
        lhs = m.complete(p, if is_range { RANGE_EXPR } else { BIN_EXPR });
    }
    Some((lhs, BlockLike::NotBlock))
}

fn is_expr(p: &mut Parser, lhs: CompletedMarker) -> CompletedMarker {
    let m = lhs.precede(p);
    p.bump_remap(T![is]);
    types::type_(p);
    while p.eat(T![|]) {
        types::type_(p);
    }
    m.complete(p, IS_EXPR)
}

fn cast_expr(p: &mut Parser, lhs: CompletedMarker) -> CompletedMarker {
    assert!(p.at(T![as]));
    let m = lhs.precede(p);
    p.bump(T![as]);
    types::type_(p);
    m.complete(p, CAST_EXPR)
}

pub(crate) fn struct_lit_field_list(p: &mut Parser) {
    assert!(p.at(T!['{']));
    let m = p.start();
    p.bump(T!['{']);
    p.iterate_to_EOF(T!['}'], |p| {
        let m = p.start();
        match p.current() {
            IDENT => {
                if p.nth_at(1, T![:]) {
                    name_ref(p);
                    p.expect(T![:]);
                }
                expr(p);
                m.complete(p, STRUCT_LIT_FIELD);
            }
            // T!['{'] => {
            //     error_block(p, "expected a field");
            //     m.abandon(p);
            // }
            _ => {
                p.error_and_bump("expected identifier");
                m.abandon(p);
            }
        }
        if !p.at(T!['}']) {
            p.expect(T![,]);
        }
        Continue(())
    });
    p.expect(T!['}']);
    m.complete(p, STRUCT_LIT_FIELD_LIST);
}

pub(crate) fn lhs(p: &mut Parser, r: Restrictions) -> Option<(CompletedMarker, BlockLike)> {
    let m;
    let kind = match p.current() {
        T![|] => {
            m = p.start();
            if !lambda_param_list(p) {
                m.abandon(p);
                return None;
            }
            expr_bp(p, None, r, 1);
            let cm = m.complete(p, LAMBDA_EXPR);
            return Some((cm, BlockLike::NotBlock));
        }
        T![&] => {
            m = p.start();
            p.bump(T![&]);
            p.eat(T![mut]);
            BORROW_EXPR
        }
        IDENT if is_at_quant_kw(p) => {
            if let Some(cm) = forall_expr(p) {
                return Some((cm, BlockLike::NotBlock));
            }
            if let Some(cm) = exists_expr(p) {
                return Some((cm, BlockLike::NotBlock));
            }
            if let Some(cm) = choose_expr(p) {
                return Some((cm, BlockLike::NotBlock));
            }
            unreachable!()
        }
        IDENT if p.at_contextual_kw("copy") => {
            m = p.start();
            p.bump_remap(T![copy]);
            RESOURCE_EXPR
        }
        T![move] => {
            m = p.start();
            p.bump(T![move]);
            RESOURCE_EXPR
        }
        T![*] => {
            m = p.start();
            p.bump(T![*]);
            DEREF_EXPR
        }
        T![!] => {
            m = p.start();
            p.bump(T![!]);
            BANG_EXPR
        }
        T![..] => {
            m = p.start();
            p.bump(T![..]);
            if p.at_ts(EXPR_FIRST) && !(r.forbid_structs && p.at(T!['{'])) {
                expr_bp(p, None, r, 2);
            }
            let cm = m.complete(p, RANGE_EXPR);
            return Some((cm, BlockLike::NotBlock));
        }
        _ => {
            let (lhs, blocklike) = atom::atom_expr(p)?;

            let allow_calls = !(r.prefer_stmt && blocklike.is_block());
            let cm = postfix_expr(p, lhs, blocklike, allow_calls);

            return Some(cm);
        }
    };
    // parse the interior of the unary expression
    expr_bp(p, None, r, 255);
    let cm = m.complete(p, kind);
    Some((cm, BlockLike::NotBlock))
}

fn postfix_expr(
    p: &mut Parser,
    mut lhs: CompletedMarker,
    // Calls are disallowed if the type is a block and we prefer statements because the call cannot be disambiguated from a tuple
    // E.g. `while true {break}();` is parsed as
    // `while true {break}; ();`
    mut block_like: BlockLike,
    mut allow_calls: bool,
) -> (CompletedMarker, BlockLike) {
    loop {
        lhs = match p.current() {
            T!['('] if allow_calls => call_expr(p, lhs),
            T!['['] if allow_calls => index_expr(p, lhs),
            T![.] => postfix_dot_expr(p, lhs),
            _ => break,
        };
        allow_calls = true;
        block_like = BlockLike::NotBlock;
    }
    (lhs, block_like)
}

fn postfix_dot_expr(p: &mut Parser, lhs: CompletedMarker) -> CompletedMarker {
    assert!(p.at(T![.]));

    let m = lhs.precede(p);
    p.bump(T![.]);

    match p.current() {
        IDENT => {
            name_ref(p);
            'method_call: {
                if p.at_ts(T!['('] | T![::]) || (p.at(T![<]) && p.prev_ws() == 0) {
                    let is_error_in_type_args = !type_args::opt_type_arg_list_for_expr(p, true);
                    if is_error_in_type_args {
                        // cannot be a method
                        break 'method_call;
                    }
                    if !p.at(T!['(']) {
                        // cannot be a method
                        break 'method_call;
                    }
                    value_arg_list(p);
                    return m.complete(p, METHOD_CALL_EXPR);
                }
            }
        }
        INT_NUMBER => {
            let m = p.start();
            p.bump_any();
            m.complete(p, NAME_REF);
        }
        _ => {
            p.error("expected field name or number");
        }
    }

    m.complete(p, DOT_EXPR)
}

fn index_expr(p: &mut Parser, lhs: CompletedMarker) -> CompletedMarker {
    assert!(p.at(T!['[']));
    let m = lhs.precede(p);
    p.bump(T!['[']);
    expr(p);
    p.expect(T![']']);
    m.complete(p, INDEX_EXPR)
}

fn value_arg_list(p: &mut Parser) {
    assert!(p.at(T!['(']));
    let m = p.start();
    p.bump(T!['(']);
    delimited_with_recovery(
        p,
        |p| {
            let m = p.start();
            let is_expr = expr(p);
            if is_expr {
                m.complete(p, VALUE_ARG);
                true
            } else {
                if p.at(T![,]) {
                    // ,,
                    m.complete(p, VALUE_ARG);
                } else {
                    m.abandon(p);
                }
                false
            }
            // if !is_expr && p.at(T![,]) {
            //     // ,,,,
            //     m.complete(p, VALUE_ARG);
            // }
            // if !is_expr && p.current() == T![,] {
            //     m.complete(p, VALUE_ARG);
            //     return true;
            // } else {
            //     m.abandon(p);
            // }
            // false
            // m.complete(p, VALUE_ARG);
            // true
            // if is_expr {
            //     m.complete(p, VALUE_ARG);
            // } else {
            //     m.abandon(p);
            // }
            // is_expr
        },
        T![,],
        "expected argument",
        Some(T![')']),
    );
    // delimited_items_with_recover(p, T![')'], T![,], ts!(T![;], T![let], T!['}']), VALUE_ARG, |p| {
    //     let m = p.start();
    //     let is_expr = expr(p);
    //     if is_expr {
    //         m.complete(p, VALUE_ARG);
    //     } else {
    //         m.abandon(p);
    //     }
    //     is_expr
    // });
    p.expect(T![')']);
    m.complete(p, VALUE_ARG_LIST);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlockLike {
    Block,
    NotBlock,
}

impl BlockLike {
    fn is_block(self) -> bool {
        self == BlockLike::Block
    }

    fn is_blocklike(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            BLOCK_EXPR | IF_EXPR | WHILE_EXPR | FOR_EXPR | LOOP_EXPR | MATCH_EXPR
        )
    }
}

pub(crate) fn opt_initializer_expr(p: &mut Parser) {
    if p.eat(T![=]) {
        if !expr(p) {
            p.error("expected expression");
        }
    }
}

pub(crate) fn stmt_expr(p: &mut Parser) -> Option<(CompletedMarker, BlockLike)> {
    let r = Restrictions {
        forbid_structs: false,
        prefer_stmt: true,
    };
    expr_bp(p, None, r, 1)
}

pub(super) fn expr_block_contents(p: &mut Parser, is_spec: bool) {
    p.iterate_to_EOF(T!['}'], |p| {
        if p.at(T![;]) {
            p.bump(T![;]);
            return Continue(());
        }
        p.with_recovery_token_set(T!['}'], |p| stmts::stmt(p, false, is_spec));
        Continue(())
    });
}

#[derive(Clone, Copy, Default)]
pub(crate) struct Restrictions {
    pub forbid_structs: bool,
    pub prefer_stmt: bool,
}

pub(crate) const EXPR_FIRST: TokenSet =
    atom::ATOM_EXPR_FIRST.union(TokenSet::new(&[T![&], T![*], T![!]]));

pub(crate) const STMT_FIRST: TokenSet = EXPR_FIRST.union(TokenSet::new(&[T![let]]));

/// Binding powers of operators for a Pratt parser.
///
/// See <https://matklad.github.io/2020/04/13/simple-but-powerful-pratt-parsing.html>
#[rustfmt::skip]
fn current_op(p: &Parser) -> (u8, SyntaxKind) {
    const NOT_AN_OP: (u8, SyntaxKind) = (0, T![@]);
    match p.current() {
        T![=]                  => (1,  T![=]),
        T![>] if p.at(T![>>=]) => (1,  T![>>=]),
        T![<] if p.at(T![<==>])  => (1, T![<==>]),
        T![<] if p.at(T![<<=]) => (1,  T![<<=]),
        T![==>]  => (1, T![==>]),
        T![+=]  => (1,  T![+=]),
        T![-=]  => (1,  T![-=]),
        T![*=]  => (1,  T![*=]),
        T![/=]  => (1,  T![/=]),
        T![|=]  => (1,  T![|=]),
        T![^=]  => (1,  T![^=]),
        T![&=]  => (1,  T![&=]),
        T![%=]  => (1,  T![%=]),

        T![as]                 => (2, T![as]),
        T![ident] if p.at_contextual_kw("is") => (2, T![is]),

        T![..]  => (3,  T![..]),

        T![|] if p.at(T![||])  => (4,  T![||]),

        T![&] if p.at(T![&&])  => (5,  T![&&]),

        T![>] if p.at(T![>=])  => (6,  T![>=]),
        T![>] if p.at(T![>>])  => (10,  T![>>]),
        T![>]                  => (6,  T![>]),

        T![<] if p.at(T![<=])  => (6,  T![<=]),
        T![<] if p.at(T![<<])  => (10,  T![<<]),
        T![<]                 => (6,  T![<]),
        T![!=]  => (6,  T![!=]),
        T![==]  => (6,  T![==]),

        T![|]                  => (7,  T![|]),

        T![^]                  => (8,  T![^]),

        T![&]                  => (9,  T![&]),


        T![+]                  => (11, T![+]),
        T![-]                  => (11, T![-]),

        T![%]                  => (12, T![%]),
        T![/]                  => (12, T![/]),
        T![*]                  => (12, T![*]),

        // T![as]                 => (13, T![as]),
        // T![ident] if p.at_contextual_kw("is") => (13, T![is]),

        T![=] if p.at(T![=>])  => NOT_AN_OP,
        _                      => NOT_AN_OP
    }
}
