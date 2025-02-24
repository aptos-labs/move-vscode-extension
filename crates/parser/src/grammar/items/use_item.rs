use crate::grammar::items::ITEM_KW_RECOVERY_SET;
use crate::grammar::{name, paths};
use crate::parser::Marker;
use crate::SyntaxKind::*;
use crate::{Parser, T};

// test use_item
// use std::collections;
pub(crate) fn use_(p: &mut Parser, m: Marker) {
    p.bump(T![use]);
    use_speck(p, true);
    p.expect(T![;]);
    m.complete(p, USE_ITEM);
}

// test use_tree
// use outer::tree::{inner::tree};
fn use_speck(p: &mut Parser, top_level: bool) {
    let m = p.start();
    match p.current() {
        T!['{'] => use_group(p),
        T![:] if p.at(T![::]) && p.nth(2) == T!['{'] => {
            p.bump(T![::]);
            use_group(p);
        }

        // test use_tree_path
        // use ::std;
        // use std::collections;
        //
        // use self::m;
        // use super::m;
        // use crate::m;
        _ if paths::is_use_path_start(p) => {
            paths::use_path(p);
            match p.current() {
                // test use_tree_alias
                // use std as stdlib;
                // use Trait as _;
                T![as] => opt_use_alias(p),
                // T![:] if p.at(T![::]) => {
                T![::] => {
                    p.bump(T![::]);
                    match p.current() {
                        // test use_tree_path_star
                        // use std::*;
                        // T![*] => p.bump(T![*]),
                        // test use_tree_path_use_tree
                        // use std::{collections};
                        T!['{'] => use_group(p),
                        _ => p.error("expected `{` or `*`"),
                    }
                }
                _ => (),
            }
        }
        _ => {
            m.abandon(p);
            let msg = "expected one of `*`, `::`, `{`, `self`, `super` or an identifier";
            if top_level {
                p.err_recover(msg, ITEM_KW_RECOVERY_SET);
            } else {
                // if we are parsing a nested tree, we have to eat a token to
                // main balanced `{}`
                p.err_and_bump(msg);
            }
            return;
        }
    }
    m.complete(p, USE_SPECK);
}

// use {a, b, c};
pub(crate) fn use_group(p: &mut Parser) {
    assert!(p.at(T!['{']));
    let m = p.start();
    p.bump(T!['{']);
    while !p.at(EOF) && !p.at(T!['}']) {
        use_speck(p, false);
        if !p.at(T!['}']) {
            p.expect(T![,]);
        }
    }
    p.expect(T!['}']);
    m.complete(p, USE_GROUP);
}

fn opt_use_alias(p: &mut Parser) {
    if p.at(T![as]) {
        let m = p.start();
        p.bump(T![as]);
        name(p);
        // if !p.eat(T![_]) {
        // }
        m.complete(p, USE_ALIAS);
    }
}
