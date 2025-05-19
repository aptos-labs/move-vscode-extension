use crate::parse::grammar::paths::PATH_FIRST;
use crate::parse::grammar::utils::list;
use crate::parse::grammar::{expressions, paths};
use crate::parse::parser::Parser;
use crate::parse::token_set::TokenSet;
use crate::SyntaxKind::*;
use crate::{SyntaxKind, T};

pub(super) fn outer_attrs(p: &mut Parser) {
    while p.at(T![#]) {
        attr(p, false);
    }
}

fn attr(p: &mut Parser, inner: bool) {
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

    // if p.eat(T!['[']) {
    //     attr_item_list(p, );
    //     // if !p.eat(T![']']) {
    //     //     p.error("expected `]`");
    //     // }
    // } else {
    // }
    attr.complete(p, ATTR);
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
    list(
        p,
        lparen,
        rparen,
        T![,],
        || "expected attribute item".into(),
        PATH_FIRST,
        |p| attr_item(p),
    );
}

pub(super) const ATTRIBUTE_FIRST: TokenSet = TokenSet::new(&[T![#]]);
