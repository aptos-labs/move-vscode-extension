use crate::SyntaxKind::*;
use crate::T;
use crate::parse::grammar::items::item_start_kws_only;
use crate::parse::grammar::utils::delimited_with_recovery;
use crate::parse::grammar::{name, paths};
use crate::parse::parser::Parser;
use crate::parse::token_set::TokenSet;
use std::ops::ControlFlow::Continue;

// recovery set is passed from the outside
pub(crate) fn use_speck(p: &mut Parser, is_top_level: bool) -> bool {
    if !paths::is_path_start(p) {
        let message = if is_top_level {
            "expected address or identifier"
        } else {
            "expected identifier"
        };
        p.error_and_recover(message, TokenSet::EMPTY);
        return true;
    }
    let m = p.start();
    paths::use_path(p);
    match p.current() {
        T![as] => use_alias(p),
        T![::] => {
            p.bump(T![::]);
            match p.current() {
                T!['{'] => use_group(p),
                _ => p.error("expected `{` or identifier"),
            }
        }
        _ => (),
    }
    m.complete(p, USE_SPECK);
    true
}

pub(crate) fn use_group(p: &mut Parser) {
    assert!(p.at(T!['{']));
    let m = p.start();
    p.bump(T!['{']);
    p.reset_recovery_set(|p| {
        delimited_with_recovery(
            p,
            |p| use_speck(p, false),
            T![,],
            "expected identifier",
            Some(T!['}']),
        )
    });
    p.expect(T!['}']);
    m.complete(p, USE_GROUP);
}

fn use_alias(p: &mut Parser) {
    assert!(p.at(T![as]));
    let m = p.start();
    p.bump(T![as]);
    name(p);
    m.complete(p, USE_ALIAS);
}
