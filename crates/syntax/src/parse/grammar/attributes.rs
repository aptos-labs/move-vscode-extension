// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::SyntaxKind::*;
use crate::parse::grammar::items::initializer_expr;
use crate::parse::grammar::paths::{PATH_FIRST, PathMode};
use crate::parse::grammar::utils::delimited_with_recovery;
use crate::parse::grammar::{expressions, paths};
use crate::parse::parser::{CompletedMarker, Parser};
use crate::parse::token_set::TokenSet;
use crate::{SyntaxKind, T};

pub(super) fn outer_attrs(p: &mut Parser) -> Vec<CompletedMarker> {
    let mut attrs = vec![];
    while p.at(T![#]) {
        attrs.push(attr(p));
    }
    attrs
}

fn attr(p: &mut Parser) -> CompletedMarker {
    assert!(p.at(T![#]));

    let attr = p.start();
    p.bump(T![#]);

    if p.at(T!['[']) {
        attr_item_list(p, T!['['], T![']']);
    } else {
        p.error("expected `[`");
    }

    attr.complete(p, ATTR)
}

pub(super) fn attr_item(p: &mut Parser) -> bool {
    let m = p.start();
    paths::path(p, None);

    match p.current() {
        T![=] => {
            initializer_expr(p);
            // p.bump(T![=]);
            // let m = p.start();
            // let is_expr = expressions::expr(p);
            // if !is_expr {
            //     p.error("expected expression");
            //     m.abandon(p);
            // } else {
            //     m.complete(p, INITIALIZER);
            // }
        }
        T!['('] => {
            let m = p.start();
            attr_item_list(p, T!['('], T![')']);
            m.complete(p, ATTR_ITEM_LIST);
        }
        _ => (),
    }

    m.complete(p, ATTR_ITEM);
    true
}

pub(crate) fn attr_item_list(p: &mut Parser, lparen: SyntaxKind, rparen: SyntaxKind) {
    p.bump(lparen);
    delimited_with_recovery(p, attr_item, T![,], "expected attribute item", Some(rparen));
    p.expect(rparen);
}

pub(super) const ATTRIBUTE_FIRST: TokenSet = TokenSet::new(&[T![#]]);
