use crate::parse::grammar::paths::PATH_FIRST;
use crate::parse::grammar::utils::delimited_with_recovery;
use crate::parse::grammar::{expressions, paths};
use crate::parse::parser::{CompletedMarker, Parser};
use crate::parse::token_set::TokenSet;
use crate::SyntaxKind::*;
use crate::{SyntaxKind, T};

pub(super) fn outer_attrs(p: &mut Parser) -> Vec<CompletedMarker> {
    let mut attrs = vec![];
    while p.at(T![#]) {
        attrs.push(attr(p, false));
    }
    attrs
}

fn attr(p: &mut Parser, inner: bool) -> CompletedMarker {
    assert!(p.at(T![#]));

    let attr = p.start();
    p.bump(T![#]);

    if inner {
        p.bump(T![!]);
    }

    if p.at(T!['[']) {
        attr_item_list(p, T!['['], T![']']);
    } else {
        p.error("expected `[`");
    }

    attr.complete(p, ATTR)
}

pub(super) fn attr_item(p: &mut Parser) -> bool {
    let meta = p.start();
    paths::use_path(p);

    match p.current() {
        T![=] => {
            p.bump(T![=]);
            if !expressions::expr(p) {
                p.error("expected expression");
            }
        }
        T!['('] => attr_item_list(p, T!['('], T![')']),
        _ => {}
    }

    meta.complete(p, ATTR_ITEM);
    true
}

pub(crate) fn attr_item_list(p: &mut Parser, lparen: SyntaxKind, rparen: SyntaxKind) {
    p.bump(lparen);
    delimited_with_recovery(p, attr_item, T![,], "expected attribute item", Some(rparen));
    p.expect(rparen);
}

pub(super) const ATTRIBUTE_FIRST: TokenSet = TokenSet::new(&[T![#]]);
