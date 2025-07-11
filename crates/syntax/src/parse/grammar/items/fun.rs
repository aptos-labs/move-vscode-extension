// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::SyntaxKind::{
    ACQUIRES, EOF, FUN, IDENT, RET_TYPE, SPEC_FUN, SPEC_INLINE_FUN, VISIBILITY_MODIFIER,
};
use crate::parse::grammar::expressions::atom::block_expr;
use crate::parse::grammar::items::{at_item_start, item_start_rec_set};
use crate::parse::grammar::paths::PATH_FIRST;
use crate::parse::grammar::types::{path_type, type_, type_or_recover};
use crate::parse::grammar::utils::delimited_with_recovery;
use crate::parse::grammar::{name_or_recover, params, paths, type_params, types};
use crate::parse::parser::{Marker, Parser};
use crate::parse::recovery_set::{RecoverySet, RecoveryToken};
use crate::parse::token_set::TokenSet;
use crate::{SyntaxKind, T, ts};
use std::cell::RefCell;
use std::collections::HashSet;
use std::ops::ControlFlow::{Break, Continue};
use std::ops::{ControlFlow, DerefMut};

pub(crate) fn spec_function(p: &mut Parser, m: Marker) {
    opt_fun_modifiers(p);
    if !p.at(T![fun]) {
        m.abandon(p);
        return;
    }
    fun_signature(p, true, false);
    m.complete(p, SPEC_FUN);
}

pub(crate) fn spec_inline_function(p: &mut Parser) {
    let m = p.start();
    p.eat(T![native]);
    if !p.at(T![fun]) {
        m.abandon(p);
        return;
    }
    fun_signature(p, true, false);
    m.complete(p, SPEC_INLINE_FUN);
}

pub(crate) fn function(p: &mut Parser, m: Marker) {
    opt_fun_modifiers(p);
    if p.at(T![fun]) {
        fun_signature(p, false, true);
    } else {
        // p.error("expected 'fun'");
        p.error_and_recover("expected 'fun'", item_start_rec_set());
        // p.error_and_recover_until("expected 'fun'", at_item_start);
    }
    m.complete(p, FUN);
}

fn opt_fun_modifiers(p: &mut Parser) {
    p.iterate_to_EOF(TokenSet::EMPTY, |p| {
        if visibility_modifier(p) {
            return Continue(());
        }
        match p.current() {
            T![native] => p.bump(T![native]),
            T![inline] => p.bump(T![inline]),
            IDENT if p.at_contextual_kw("entry") => {
                p.bump_remap(T![entry]);
            }
            _ => {
                return Break(());
            }
        }
        Continue(())
    });
}

pub(crate) fn visibility_modifier(p: &mut Parser) -> bool {
    match p.current() {
        T![public] => {
            let m = p.start();
            p.bump(T![public]);
            opt_inner_public_modifier(p);
            m.complete(p, VISIBILITY_MODIFIER);
        }
        T![friend] => {
            let m = p.start();
            p.bump_remap(T![friend]);
            m.complete(p, VISIBILITY_MODIFIER);
        }
        IDENT if p.at_contextual_kw("package") => {
            let m = p.start();
            p.bump_remap(T![package]);
            m.complete(p, VISIBILITY_MODIFIER);
        }
        _ => {
            return false;
        }
    }
    true
}

fn bump_modifier_if_possible(
    p: &mut Parser,
    possible_modifiers: &mut HashSet<SyntaxKind>,
    modifier: SyntaxKind,
) {
    let exists = possible_modifiers.remove(&modifier);
    if !exists {
        p.error_and_bump(&format!("duplicate modifier '{:?}'", modifier));
        return;
    }
    p.bump_remap(modifier);

    // if !possible_modifiers.contains(&kind) {
    //     return possible_modifiers;
    // }
    // let left = possible_modifiers.into_iter().filter(|m| *m != kind).collect();
    // left
}

fn opt_inner_public_modifier(p: &mut Parser) {
    if p.eat(T!['(']) {
        match p.current() {
            IDENT if p.at_contextual_kw("package") => {
                p.bump_remap(T![package]);
            }
            T![friend] => {
                p.bump(T![friend]);
            }
            T![script] => {
                p.bump(T![script]);
            }
            _ => {
                p.error_and_recover("expected public modifier", TokenSet::new(&[T![')']]));
                // p.error_and_recover_until_ts("expected public modifier", TokenSet::new(&[T![')']]));
            }
        }
        p.expect(T![')']);
    }
}

fn acquires(p: &mut Parser) {
    let m = p.start();
    p.bump(T![acquires]);
    delimited_with_recovery(
        p,
        |p| {
            let is_path = paths::is_path_start(p);
            if !is_path {
                return false;
            }
            path_type(p);
            true
        },
        T![,],
        "expected type",
        None,
    );
    m.complete(p, ACQUIRES);
}

fn fun_signature(p: &mut Parser, is_spec: bool, allow_acquires: bool) {
    p.bump(T![fun]);

    let has_name = p.with_recovery_set(item_start_rec_set(), |p| {
        if !name_or_recover(p, (T![<] | T!['(']).into()) {
            return false;
        }
        type_params::opt_type_param_list(p);
        true
    });
    if !has_name {
        return;
    }

    let signature_recovery_set = item_start_rec_set().with_token_set(T![;] | T!['{']);
    p.with_recovery_set(signature_recovery_set, |p| {
        if p.at(T!['(']) {
            params::fun_param_list(p);
        } else {
            p.error_and_recover("expected function arguments", TokenSet::EMPTY);
        }
    });

    let item_rec_set = item_start_rec_set().with_token_set(T!['{'] | T![;]);
    p.with_recovery_set(item_rec_set, |p| {
        p.with_recovery_token(T![acquires], opt_ret_type);
        if p.at(T![acquires]) {
            if allow_acquires {
                acquires(p);
            } else {
                p.error("'acquires' not allowed");
            }
        }
    });

    if p.at(T![;]) {
        p.bump(T![;]);
        return;
    }
    if p.at(T!['{']) {
        block_expr(p, is_spec);
        return;
    }

    p.error("expected a block");
}

pub(crate) fn opt_ret_type(p: &mut Parser) {
    if !p.at(T![:]) {
        return;
    }
    let m = p.start();
    p.bump(T![:]);
    type_(p);
    m.complete(p, RET_TYPE);
}

// fn signature_end(p: &Parser) -> bool {
//     p.at_ts(ts!(T![;], T!['{']))
// }

pub(crate) fn on_function_modifiers_start(p: &Parser) -> bool {
    match p.current() {
        T![native] => true,
        T![inline] => true,
        IDENT
            if p.at_contextual_kw("entry")
            // not a name of a function
            && !p.nth_at_ts(1, ITEM_BRACE_START) =>
        {
            true
        }
        _ => false,
    }
}

pub(crate) fn on_visibility_modifier_start(p: &Parser) -> bool {
    match p.current() {
        T![public] => true,
        T![friend] => true,
        // not a name of an item
        IDENT if p.at_contextual_kw("package") && !p.nth_at_ts(1, ITEM_BRACE_START) => true,
        _ => false,
    }
}

const ITEM_BRACE_START: TokenSet = TokenSet::new(&[T![<], T!['('], T!['{']]);

pub(crate) fn function_modifier_tokens() -> Vec<RecoveryToken> {
    vec![
        T![public].into(),
        T![native].into(),
        T![friend].into(),
        T![inline].into(),
        "entry".into(),
        "package".into(),
    ]
}

pub(crate) fn function_modifier_kws() -> RecoverySet {
    let mut rec_set = RecoverySet::new();
    rec_set.with_token_set(T![public] | T![native] | T![friend] | T![inline])
    // .with_kw_ident("entry")
    // .with_kw_ident("package")
}

pub(crate) fn function_modifier_recovery_set() -> RecoverySet {
    let mut rec_set = RecoverySet::new();
    rec_set.with_token_set(T![public] | T![native] | T![friend] | T![inline])
    // .with_kw_ident("entry")
    // .with_kw_ident("package")
}
