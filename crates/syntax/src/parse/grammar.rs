// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

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
mod items;
mod lambdas;
mod params;
pub(crate) mod paths;
mod patterns;
pub(crate) mod specs;
mod type_args;
mod type_params;
mod types;
pub(crate) mod utils;

use crate::parse::grammar::attributes::outer_attrs;
use crate::parse::grammar::paths::PathMode;
use crate::parse::grammar::utils::delimited_with_recovery;
use crate::parse::parser::Marker;
use crate::parse::recovery_set::RecoverySet;
use crate::parse::token_set::TokenSet;
use crate::{SyntaxKind::*, T, parse::Parser, ts};
use std::ops::ControlFlow::Continue;

pub mod entry_points {
    use super::*;
    use std::ops::ControlFlow::Continue;

    pub fn source_file(p: &mut Parser) {
        let m = p.start();
        p.iterate_to_EOF(TokenSet::EMPTY, |p| {
            let m = p.start();
            outer_attrs(p);
            match p.current() {
                T![module] => module(p, m),
                T![spec] => module_spec(p, m),
                T![script] => script(p, m),
                IDENT if p.at_contextual_kw("address") => address_def(p, m),
                _ => {
                    m.abandon(p);
                    p.error_and_bump(&format!("unexpected ident '{}'", p.current_text()))
                }
            }
            Continue(())
        });
        m.complete(p, SOURCE_FILE);
    }
}

pub(crate) fn module(p: &mut Parser, m: Marker) {
    p.bump(T![module]);
    if p.nth_at(1, T![::]) {
        // named address
        any_address(p);
        p.bump(T![::]);
    }
    name_or_recover(p, TOP_LEVEL_FIRST.into());

    if p.at(T!['{']) {
        items::item_list(p);
    } else {
        p.error_and_recover("expected '{'", TOP_LEVEL_FIRST);
    }
    m.complete(p, MODULE);
}

pub(crate) fn address_def(p: &mut Parser, m: Marker) {
    p.bump_remap(T![address]);
    any_address(p);
    if p.at(T!['{']) {
        p.bump(T!['{']);
        p.iterate_to_EOF(T!['}'], |p| {
            let m = p.start();
            outer_attrs(p);
            if p.at(T![module]) {
                module(p, m);
            } else {
                m.abandon(p);
                p.error_and_recover("expected module", T![module] | T!['}']);
            }
            Continue(())
        });
        p.expect(T!['}']);
    } else {
        p.error_and_recover("expected '{'", TOP_LEVEL_FIRST);
    }
    m.complete(p, ADDRESS_DEF);
}

pub(crate) fn module_spec(p: &mut Parser, m: Marker) {
    p.bump(T![spec]);
    p.with_recovery_set(top_level_set().with_token_set(T!['{']), |p| paths::path(p, None));

    if p.at(T!['{']) {
        items::item_list(p);
    } else {
        p.error_and_recover("expected '{'", top_level_set());
    }

    m.complete(p, MODULE_SPEC);
}

pub(crate) fn script(p: &mut Parser, m: Marker) {
    p.bump(T![script]);
    if p.at(T!['{']) {
        items::item_list(p);
    } else {
        p.error_and_recover("expected `{`", TOP_LEVEL_FIRST);
    }
    m.complete(p, SCRIPT);
}

pub(crate) fn module_name(p: &mut Parser) {
    if p.nth_at(1, T![::]) {
        // named address
        any_address(p);
        p.bump(T![::]);
    }
    name_or_recover(p, TOP_LEVEL_FIRST.into());
}

pub(crate) fn any_address(p: &mut Parser) {
    if p.at(INT_NUMBER) {
        value_address(p);
    } else if p.at(IDENT) {
        // named address
        let m = p.start();
        p.bump(IDENT);
        m.complete(p, NAMED_ADDRESS);
    } else {
        p.error("expected address reference");
    }
}

pub(crate) fn value_address(p: &mut Parser) {
    assert!(p.at(INT_NUMBER));
    let m = p.start();
    p.bump(INT_NUMBER);
    m.complete(p, VALUE_ADDRESS);
}

pub(crate) fn top_level_set() -> RecoverySet {
    TOP_LEVEL_FIRST.into()
}

pub(crate) const TOP_LEVEL_FIRST: TokenSet =
    TokenSet::new(&[T![module], T![script], T![spec], T![address]]);

fn name(p: &mut Parser) -> bool {
    name_or_recover(p, TokenSet::EMPTY.into())
}

fn name_ref_or_recover(p: &mut Parser) -> bool {
    if !p.at(IDENT) {
        p.error_and_recover("expected identifier", TokenSet::EMPTY);
        return false;
    }

    let rec_set = p.outer_recovery_set();
    if rec_set.contains_current(p) {
        p.error(&format!("expected identifier, got '{}'", p.current_text()));
        return false;
    }

    let m = p.start();
    p.bump(IDENT);
    m.complete(p, NAME_REF);
    true
}

fn name_ref(p: &mut Parser) -> bool {
    if p.at(IDENT) {
        let m = p.start();
        p.bump(IDENT);
        m.complete(p, NAME_REF);
        true
    } else {
        p.error_and_bump("expected identifier");
        false
    }
}

#[allow(unused)]
const IDENT_OR_INT_NUMBER: TokenSet = TokenSet::new(&[INT_NUMBER, IDENT]);

#[allow(unused)]
fn name_ref_or_index(p: &mut Parser) {
    if p.at_ts(IDENT_OR_INT_NUMBER) {
        let m = p.start();
        p.bump_any();
        m.complete(p, NAME_REF);
    } else {
        p.error_and_bump("expected integer or identifier");
    }
}

fn name_or_recover(p: &mut Parser, extra: RecoverySet) -> bool {
    if !p.at(IDENT) {
        p.error_and_recover(&format!("expected identifier, got '{}'", p.current_text()), extra);
        return false;
    }

    let rec_set = p.outer_recovery_set().with_merged(extra);
    if rec_set.contains_current(p) {
        p.error(&format!("expected identifier, got '{}'", p.current_text()));
        return false;
    }

    let m = p.start();
    p.bump(IDENT);
    m.complete(p, NAME);
    true
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

pub(crate) fn abilities_list(p: &mut Parser) {
    assert!(p.at_contextual_kw_ident("has"));
    let m = p.start();
    p.bump_remap(T![has]);
    delimited_with_recovery(p, ability, T![,], "expected ability", None);
    m.complete(p, ABILITY_LIST);
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
