// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::SyntaxKind::*;
use crate::parse::grammar::items::item_start_rec_set;
use crate::parse::grammar::type_args::opt_path_type_arg_list;
use crate::parse::grammar::{any_address, items, name_ref, value_address};
use crate::parse::parser::{CompletedMarker, Parser};
use crate::parse::token_set::TokenSet;
use crate::{T, ts};

pub(super) const PATH_FIRST: TokenSet = TokenSet::new(&[IDENT, INT_NUMBER]);

pub(super) fn is_path_start(p: &Parser) -> bool {
    match p.current() {
        // addresses
        INT_NUMBER if p.nth_at(1, T![::]) => true,
        IDENT => true,
        // T![::] => true,
        T!['_'] => true,
        _ => false,
    }
}

// pub(crate) fn type_path_for_qualifier(p: &mut Parser, qual: CompletedMarker) -> CompletedMarker {
//     path_for_qualifier(p, Some(PathMode::Type), qual)
// }

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum PathMode {
    // None,
    Type,
    Expr,
}

pub(crate) fn path(p: &mut Parser, mode: Option<PathMode>) -> bool {
    let m = p.start();
    let has_first_path = path_segment(p, mode, true);
    if !has_first_path {
        m.abandon(p);
        return false;
    }
    let qual = m.complete(p, PATH);
    path_for_qualifier(p, mode, qual);
    true
}

pub(crate) fn path_for_qualifier(
    p: &mut Parser,
    mode: Option<PathMode>,
    mut qual: CompletedMarker,
) -> CompletedMarker {
    loop {
        let is_use_tree = matches!(p.nth(1), T!['{']);
        if p.at(T![::]) && !is_use_tree {
            let path = qual.precede(p);
            p.bump(T![::]);
            path_segment(p, mode, false);
            let path = path.complete(p, PATH);
            qual = path;
        } else {
            return qual;
        }
    }
}

// VALUE_ADDRESS | NAME_REF TYPE_ARGS? | '_'
fn path_segment(p: &mut Parser, type_args_kind: Option<PathMode>, is_first: bool) -> bool {
    let m = p.start();
    match p.current() {
        IDENT => {
            let m = p.start();
            p.bump(IDENT);
            m.complete(p, NAME_REF);

            #[cfg(debug_assertions)]
            let _p =
                stdx::panic_context::enter(format!("path_segment_type_args {:?}", p.current_text()));
            if let Some(type_args_kind) = type_args_kind {
                opt_path_type_arg_list(p, type_args_kind);
            }
        }
        T!['_'] => {
            let m = p.start();
            p.bump_remap(IDENT);
            m.complete(p, NAME_REF);
        }
        INT_NUMBER if is_first => {
            let m = p.start();
            value_address(p);
            m.complete(p, PATH_ADDRESS);
        }
        _ => {
            p.error_and_recover("expected identifier", item_start_rec_set());
            m.abandon(p);
            return false;
        }
    };
    m.complete(p, PATH_SEGMENT);
    true
}
