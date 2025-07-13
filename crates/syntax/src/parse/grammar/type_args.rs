// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use super::*;
use crate::TextSize;
use crate::parse::grammar::paths::PathMode;
use crate::parse::grammar::types::{TYPE_FIRST, TYPE_FIRST_NO_LAMBDA, path_type};
use crate::parse::grammar::utils::delimited_with_recovery;
use std::ops::ControlFlow::{Break, Continue};

pub(crate) fn opt_path_type_arg_list(p: &mut Parser, mode: PathMode) {
    match mode {
        // TypeArgs::None => {}
        PathMode::Type => opt_type_arg_list_for_type(p),
        PathMode::Expr => opt_type_arg_list_for_expr(p, false),
    }
}

pub(crate) fn opt_type_arg_list_for_type(p: &mut Parser) {
    let _p = stdx::panic_context::enter("opt_type_arg_list_for_type".to_string());
    if !p.at(T![<]) {
        return;
    }
    let m = p.start();
    p.bump(T![<]);
    p.with_recovery_token(T![>], |p| {
        delimited_with_recovery(
            p,
            |p| type_arg(p, true),
            T![,],
            "expected type argument",
            Some(T![>]),
        )
    });
    p.expect(T![>]);
    m.complete(p, TYPE_ARG_LIST);
}

pub(super) fn opt_type_arg_list_for_expr(p: &mut Parser, colon_colon_required: bool) {
    let m;
    if p.at(T![::]) && p.nth_at(1, T![<]) {
        m = p.start();
        p.bump(T![::]);
    } else {
        if !p.at(T![<]) {
            return;
        }
        // '::' is optional if there's no whitespace between ident and '<'
        if !colon_colon_required || p.prev_ws_at(0) == 0 {
            m = p.start();
        } else {
            return;
        }
    }

    p.bump(T![<]);

    // NOTE: we cannot add recovery in type args for expr, it's ambiguous with the lt/gt expr
    let mut has_error = false;
    p.iterate_to_EOF(T![>] | T!['('] | T!['{'], |p| {
        if !type_arg(p, false) {
            has_error = true;
            return Break(());
        }
        if !p.eat(T![,]) {
            if p.at_ts(TYPE_ARG_FIRST) {
                p.error("expected ','");
                has_error = true;
            } else {
                return Break(());
            }
        }
        Continue(())
    });
    if has_error || !p.eat(T![>]) {
        m.abandon_with_rollback(p);
        return;
    }
    m.complete(p, TYPE_ARG_LIST);
}

pub(crate) const TYPE_ARG_FIRST: TokenSet = paths::PATH_FIRST;
// .union(types::TYPE_FIRST);

pub(crate) fn type_arg(p: &mut Parser, is_type: bool) -> bool {
    let type_arg_m = p.start();
    match p.current() {
        IDENT => {
            path_type(p);
        }
        _ if p.at_ts(TYPE_FIRST) => {
            let mut rec = vec![T![,]];
            let mut rec_token_set = TokenSet::from(T![,]);
            // can't recover at T![>] in expr due to ambiguity
            if is_type {
                rec_token_set = rec_token_set | T![>];
            }
            let is_valid_type = p.with_recovery_token_set(rec_token_set, types::type_);
            if !is_type && !is_valid_type {
                // have to be safe
                type_arg_m.abandon(p);
                return false;
            }
        }
        _ => {
            type_arg_m.abandon(p);
            return false;
        }
    }
    type_arg_m.complete(p, TYPE_ARG);
    true
}
