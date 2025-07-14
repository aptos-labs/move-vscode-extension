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

pub(super) const PATH_FIRST: TokenSet = TokenSet::new(&[IDENT, INT_NUMBER, T!['_']]);

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

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum PathMode {
    Type,
    Expr,
}

pub(crate) fn path(p: &mut Parser, mode: Option<PathMode>) -> bool {
    let m = p.start();
    let has_first_segment = path_segment(p, mode, true).is_some();
    if !has_first_segment {
        m.abandon(p);
        return false;
    }
    let mut qual_path = m.complete(p, PATH);
    loop {
        if !p.at(T![::]) {
            break;
        }
        // stop if next is use group
        if p.nth_at(1, T!['{']) {
            break;
        }
        let path = qual_path.precede(p);
        p.bump(T![::]);
        path_segment(p, mode, false);
        qual_path = path.complete(p, PATH);
    }
    true
}

// VALUE_ADDRESS | NAME_REF TYPE_ARGS? | '_'
pub(crate) fn path_segment(
    p: &mut Parser,
    type_args_kind: Option<PathMode>,
    is_first: bool,
) -> Option<CompletedMarker> {
    let m = p.start();
    match p.current() {
        IDENT => {
            name_ref(p);
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
            return None;
        }
    };
    Some(m.complete(p, PATH_SEGMENT))
}
