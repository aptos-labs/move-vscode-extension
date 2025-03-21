//! This is the actual "grammar" of the Rust language.
//!
//! Each function in this module and its children corresponds
//! to a production of the formal grammar. Submodules roughly
//! correspond to different *areas* of the grammar. By convention,
//! each submodule starts with `use super::*` import and exports
//! "public" productions via `pub(super)`.
//!
//! See docs for [`Parser`](super::parser::Parser) to learn about API,
//! available to the grammar, and see docs for [`Event`](super::event::Event)
//! to learn how this actually manages to produce parse trees.
//!
//! Code in this module also contains inline tests, which start with
//! `// test name-of-the-test` comment and look like this:
//!
//! ```
//! // test function_with_zero_parameters
//! // fn foo() {}
//! ```
//!
//! After adding a new inline-test, run `cargo test -p xtask` to
//! extract it as a standalone text-fixture into
//! `crates/syntax/test_data/parser/`, and run `cargo test` once to
//! create the "gold" value.
//!
//! Coding convention: rules like `where_clause` always produce either a
//! node or an error, rules like `opt_where_clause` may produce nothing.
//! Non-opt rules typically start with `assert!(p.at(FIRST_TOKEN))`, the
//! caller is responsible for branching on the first token.

mod attributes;
mod expressions;
mod generic_params;
mod items;
mod params;
pub(crate) mod paths;
mod patterns;
pub(crate) mod specs;
pub(crate) mod type_args;
mod types;
pub(crate) mod utils;

use crate::grammar::items::item_recovery_set;
use crate::parser::Marker;
use crate::token_set::TokenSet;
use crate::{parser::Parser, SyntaxKind, SyntaxKind::*, T};

pub mod entry_points {
    use super::*;

    pub fn source_file(p: &mut Parser) {
        let m = p.start();
        while !p.at(EOF) {
            let m = p.start();
            attributes::outer_attrs(p);
            match p.current() {
                T![module] => module(p, m),
                T![spec] => module_spec(p, m),
                T![script] => script(p, m),
                IDENT if p.at_contextual_kw("address") => address_def(p, m),
                _ => {
                    m.abandon(p);
                    p.err_and_bump(&format!("unexpected token {:?}", p.current()))
                }
            }
        }
        m.complete(p, SOURCE_FILE);
    }

    pub fn expr(p: &mut Parser) {
        let m = p.start();
        expressions::expr(p);
        if p.at(EOF) {
            m.abandon(p);
            return;
        }
        while !p.at(EOF) {
            p.bump_any();
        }
        m.complete(p, ERROR);
    }
}

// test mod_item
// module 0x1::m {}
pub(crate) fn module(p: &mut Parser<'_>, m: Marker) {
    p.bump(T![module]);
    module_name(p);
    if p.at(T!['{']) {
        // test mod_item_curly
        // mod b { }
        item_list(p);
    } else {
        p.err_recover("expected `{`", TOP_LEVEL_RECOVERY_SET);
    }
    m.complete(p, MODULE);
}

pub(crate) fn address_def(p: &mut Parser<'_>, m: Marker) {
    p.bump_remap(T![address]);
    address_ref(p);
    if p.at(T!['{']) {
        // test mod_item_curly
        // mod b { }
        p.bump(T!['{']);
        while !p.at(EOF) && !p.at(T!['}']) {
            let m = p.start();
            module(p, m);
        }
        p.expect(T!['}']);
    } else {
        p.err_recover("expected `{`", TOP_LEVEL_RECOVERY_SET);
    }
    m.complete(p, ADDRESS_DEF);
}

pub(crate) fn module_spec(p: &mut Parser, m: Marker) {
    p.bump(T![spec]);
    module_name(p);
    if p.at(T!['{']) {
        // test mod_item_curly
        // mod b { }
        item_list(p);
    } else {
        p.err_recover("expected `{`", TOP_LEVEL_RECOVERY_SET);
    }
    m.complete(p, MODULE_SPEC);
}

pub(crate) fn script(p: &mut Parser, m: Marker) {
    p.bump(T![script]);
    if p.at(T!['{']) {
        // test mod_item_curly
        // mod b { }
        item_list(p);
    } else {
        p.err_recover("expected `{`", TOP_LEVEL_RECOVERY_SET);
    }
    m.complete(p, SCRIPT);
}

pub(crate) fn module_name(p: &mut Parser) {
    if p.nth_at(1, T![::]) {
        // named address
        address_ref(p);
        p.bump(T![::]);
    }
    name_r(p, |p| p.at_ts(TOP_LEVEL_RECOVERY_SET));
}

pub(crate) fn address_ref(p: &mut Parser) {
    // named address
    // let m = p.start();
    address(p);
    // m.complete(p, ADDRESS_REF);
}

pub(crate) fn address(p: &mut Parser) {
    if p.at(INT_NUMBER) {
        // value address
        let m = p.start();
        p.bump(INT_NUMBER);
        m.complete(p, VALUE_ADDRESS);
    } else if p.at(IDENT) {
        // named address
        let m = p.start();
        p.bump(IDENT);
        m.complete(p, NAMED_ADDRESS);
    } else {
        p.err_and_bump("expected address reference");
    }
}

pub(crate) const TOP_LEVEL_RECOVERY_SET: TokenSet =
    TokenSet::new(&[T![module], T![script], T![spec], EOF]);

pub(crate) fn item_list(p: &mut Parser<'_>) {
    assert!(p.at(T!['{']));
    // let m = p.start();
    p.bump(T!['{']);
    items::mod_contents(p, true);
    p.expect(T!['}']);
    // m.complete(p, ITEM_LIST);
}

fn name(p: &mut Parser) -> bool {
    name_r(p, |p| p.at_ts(TokenSet::EMPTY))
}

fn name_ref(p: &mut Parser) {
    if p.at(IDENT) {
        let m = p.start();
        p.bump(IDENT);
        m.complete(p, NAME_REF);
    } else {
        p.err_and_bump("expected identifier");
    }
}

const PATH_NAME_REF_OR_INDEX_KINDS: TokenSet = TokenSet::new(&[INT_NUMBER, IDENT]);

fn name_ref_or_index(p: &mut Parser<'_>) {
    if p.at_ts(PATH_NAME_REF_OR_INDEX_KINDS) {
        let m = p.start();
        p.bump_any();
        m.complete(p, NAME_REF);
    } else {
        p.err_and_bump("expected integer or identifier");
    }
}

// fn name_ref_or_index(p: &mut Parser<'_>) {
//     assert!(p.at(IDENT) || p.at(INT_NUMBER));
//     let m = p.start();
//     p.bump_any();
//     m.complete(p, NAME_REF);
// }

fn named_address(p: &mut Parser) {
    let named_addr = p.start();
    p.bump(IDENT);
    named_addr.complete(p, NAMED_ADDRESS);
}

fn opt_ret_type(p: &mut Parser<'_>) -> bool {
    if p.at(T![:]) {
        let m = p.start();
        p.bump(T![:]);
        types::type_no_bounds(p);
        m.complete(p, RET_TYPE);
        true
    } else {
        false
    }
}

fn item_name_r(p: &mut Parser) {
    // if p.at(IDENT) {
    //     let m = p.start();
    //     p.bump(IDENT);
    //     m.complete(p, NAME);
    // } else {
    //     p.err_recover("expected a name", recovery);
    // }
    name_r(p, item_recovery_set);
}

fn name_r<Recovery>(p: &mut Parser, recovery: Recovery) -> bool
where
    Recovery: Fn(&Parser) -> bool,
{
    if p.at(IDENT) {
        let m = p.start();
        p.bump(IDENT);
        m.complete(p, NAME);
        true
    } else {
        p.err_recover_fn("expected a name", recovery);
        false
    }
}

fn error_block(p: &mut Parser, message: &str) {
    assert!(p.at(T!['{']));
    let m = p.start();
    p.error(message);
    p.bump(T!['{']);
    expressions::expr_block_contents(p, false);
    p.eat(T!['}']);
    m.complete(p, ERROR);
}

pub(crate) fn ability(p: &mut Parser) -> bool {
    if !p.at(IDENT) {
        return false;
    }
    let m = p.start();
    p.bump(IDENT);
    m.complete(p, ABILITY);
    true
}
