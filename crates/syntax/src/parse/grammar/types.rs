// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::SyntaxKind::*;
use crate::parse::Parser;
use crate::parse::grammar::paths::PathMode;
use crate::parse::grammar::utils::delimited_with_recovery;
use crate::parse::grammar::{paths, type_params, types};
use crate::parse::recovery_set::RecoverySet;
use crate::parse::token_set::TokenSet;
use crate::{T, ts};
use std::ops::ControlFlow::{Break, Continue};

pub(super) fn path_type(p: &mut Parser) -> bool {
    if !paths::is_path_start(p) {
        return false;
    }
    let m = p.start();
    let is_path = paths::path(p, Some(PathMode::Type));
    if !is_path {
        m.abandon(p);
        return false;
    }

    m.complete(p, PATH_TYPE);
    true
}

pub(crate) fn type_annotation(p: &mut Parser) {
    assert!(p.at(T![:]));
    p.bump(T![:]);
    type_(p);
}

pub(crate) fn type_(p: &mut Parser) -> bool {
    type_or_recover(p, TokenSet::EMPTY)
}

pub(crate) fn type_or_recover(p: &mut Parser, extra: impl Into<RecoverySet>) -> bool {
    match p.current() {
        T!['('] => paren_or_tuple_or_unit_type(p),
        T![&] => ref_type(p),
        T![|] => lambda_type(p),
        _ if paths::is_path_start(p) => {
            return path_type(p);
        }
        _ => {
            p.error_and_recover("expected type", extra.into());
            return false;
        }
    }
    true
}

fn ref_type(p: &mut Parser) {
    assert!(p.at(T![&]));
    let m = p.start();
    p.bump(T![&]);
    p.eat(T![mut]);
    type_(p);
    m.complete(p, REF_TYPE);
}

fn lambda_type(p: &mut Parser) {
    assert!(p.at(T![|]));
    let m = p.start();
    p.bump(T![|]);
    if p.at(T![,]) {
        m.abandon_with_rollback(p);
        return;
    }
    delimited_with_recovery(
        p,
        |p| {
            let m = p.start();
            let is_type = type_or_recover(p, T![,] | T![|]);
            if is_type {
                m.complete(p, LAMBDA_TYPE_PARAM);
            } else {
                m.complete(p, ERROR);
            }
            true
        },
        T![,],
        "expected type",
        Some(T![|]),
    );
    if !p.eat(T![|]) {
        m.abandon_with_rollback(p);
        return;
    }
    // return type
    if !p.at_contextual_kw_ident("has") && p.at_ts(TYPE_FIRST) {
        type_(p);
    }
    if p.at_contextual_kw_ident("has") {
        let m = p.start();
        p.bump_remap(T![has]);
        type_params::ability_bound_list(p);
        m.complete(p, LAMBDA_TYPE_ABILITY_LIST);
    }
    m.complete(p, LAMBDA_TYPE);
}

fn paren_or_tuple_or_unit_type(p: &mut Parser) {
    assert!(p.at(T!['(']));
    let m = p.start();
    p.bump(T!['(']);
    let mut n_types: u32 = 0;
    let mut trailing_comma: bool = false;

    p.iterate_to_EOF(T![')'], |p| {
        n_types += 1;
        type_(p);
        if p.eat(T![,]) {
            trailing_comma = true;
        } else {
            trailing_comma = false;
            return Break(());
        }
        Continue(())
    });

    p.expect(T![')']);

    let kind = if n_types == 1 && !trailing_comma {
        // type T = (i32);
        PAREN_TYPE
    } else if n_types == 0 {
        // type T = ();
        UNIT_TYPE
    } else {
        // type T = (i32,);
        TUPLE_TYPE
    };
    m.complete(p, kind);
}

pub(super) const TYPE_FIRST_NO_LAMBDA: TokenSet =
    paths::PATH_FIRST.union(TokenSet::new(&[T!['('], T!['['], T![<], T![!], T![*], T![&]]));

pub(super) const TYPE_FIRST: TokenSet = TYPE_FIRST_NO_LAMBDA.union(ts!(T![|]));

pub(super) const TYPE_RECOVERY_SET: TokenSet = TokenSet::new(&[T![')'], T![>], T![,]]);
