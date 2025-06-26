use crate::parse::grammar::items::{at_item_start, item_start_rec_set};
use crate::parse::grammar::{name, paths};
use crate::parse::parser::{Marker, Parser};
use crate::SyntaxKind::*;
use crate::T;
use std::ops::ControlFlow::Continue;

// test use_tree
// use outer::tree::{inner::tree};
pub(crate) fn use_speck(p: &mut Parser, at_the_top: bool) {
    let m = p.start();
    match p.current() {
        T!['{'] => use_group(p),
        T![:] if p.at(T![::]) && p.nth(2) == T!['{'] => {
            p.bump(T![::]);
            use_group(p);
        }

        _ if paths::is_path_start(p) => {
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
            if at_the_top {
                p.error_and_recover(msg, item_start_rec_set());
                // p.error_and_recover_until(msg, at_item_start);
            } else {
                // if we are parsing a nested tree, we have to eat a token to
                // remain balanced `{}`
                p.error_and_bump(msg);
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
    p.iterate_to_EOF(T!['}'], |p| {
        use_speck(p, false);
        if !p.at(T!['}']) {
            p.expect(T![,]);
        }
        Continue(())
    });
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
