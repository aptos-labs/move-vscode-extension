use crate::parse::grammar::expressions::atom::block_expr;
use crate::parse::grammar::items::{at_item_start, item_start_rset};
use crate::parse::grammar::paths::PATH_FIRST;
use crate::parse::grammar::types::path_type;
use crate::parse::grammar::utils::delimited;
use crate::parse::grammar::{item_name_or_recover, params, paths, type_params, types};
use crate::parse::parser::{Marker, Parser, RecoverySet, RecoveryToken};
use crate::parse::token_set::TokenSet;
use crate::SyntaxKind::{
    ACQUIRES, EOF, FUN, IDENT, RET_TYPE, SPEC_FUN, SPEC_INLINE_FUN, VISIBILITY_MODIFIER,
};
use crate::{ts, SyntaxKind, T};
use std::collections::HashSet;

pub(crate) fn spec_function(p: &mut Parser, m: Marker) {
    opt_modifiers(p);
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
    opt_modifiers(p);
    if p.at(T![fun]) {
        fun_signature(p, false, true);
    } else {
        // p.error("expected 'fun'");
        p.error_and_recover("expected 'fun'", item_start_rset());
        // p.error_and_recover_until("expected 'fun'", at_item_start);
    }
    m.complete(p, FUN);
}

fn opt_modifiers(p: &mut Parser) {
    let mut remaining_modifiers: HashSet<SyntaxKind> = vec![
        T![inline],
        T![entry],
        T![public],
        T![native],
        T![friend],
        T![package],
    ]
    .into_iter()
    .collect();

    while !p.at(EOF) {
        match p.current() {
            T![native] => {
                bump_modifier_if_possible(p, &mut remaining_modifiers, T![native]);
            }
            T![inline] => {
                bump_modifier_if_possible(p, &mut remaining_modifiers, T![inline]);
            }
            IDENT if p.at_contextual_kw("entry") => {
                bump_modifier_if_possible(p, &mut remaining_modifiers, T![entry]);
            }
            T![public] => {
                let m = p.start();
                bump_modifier_if_possible(p, &mut remaining_modifiers, T![public]);
                opt_inner_public_modifier(p);
                m.complete(p, VISIBILITY_MODIFIER);
            }
            T![friend] => {
                let m = p.start();
                bump_modifier_if_possible(p, &mut remaining_modifiers, T![friend]);
                m.complete(p, VISIBILITY_MODIFIER);
            }
            IDENT if p.at_contextual_kw("package") => {
                let m = p.start();
                bump_modifier_if_possible(p, &mut remaining_modifiers, T![package]);
                m.complete(p, VISIBILITY_MODIFIER);
            }
            _ => {
                break;
            }
        }
    }
}

fn bump_modifier_if_possible(
    p: &mut Parser,
    possible_modifiers: &mut HashSet<SyntaxKind>,
    modifier: SyntaxKind,
) {
    let exists = possible_modifiers.remove(&modifier);
    if !exists {
        p.bump_with_error(&format!("duplicate modifier '{:?}'", modifier));
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
                p.error_and_recover_until_ts("expected public modifier", TokenSet::new(&[T![')']]));
            }
        }
        p.expect(T![')']);
    }
}

fn acquires(p: &mut Parser) {
    let m = p.start();
    p.bump(T![acquires]);
    if !paths::is_path_start(p) {
        p.error_and_recover_until("expected type", |p| {
            at_item_start(p) || p.at(T!['{']) || p.at(T![;])
        });
    }
    delimited(
        p,
        T![,],
        || "unexpected ','".into(),
        |p| p.at(T!['{']) || p.at(T![;]),
        PATH_FIRST,
        |p| {
            if paths::is_path_start(p) {
                path_type(p);
            } else {
                p.error("expected type");
            }
            true
        },
    );
    m.complete(p, ACQUIRES);
}

fn fun_signature(p: &mut Parser, is_spec: bool, allow_acquires: bool) {
    p.bump(T![fun]);

    if !item_name_or_recover(p, |p| p.at(T![<]) || p.at(T!['('])) {
        return;
    }
    type_params::opt_type_param_list(p);
    if p.at(T!['(']) {
        params::fun_param_list(p);
    } else {
        p.error_and_recover_until("expected function arguments", |p| {
            at_item_start(p) || p.at_ts(ts!(T![;], T!['{']))
        });
    }
    opt_ret_type(p);

    if p.at(T![acquires]) {
        if allow_acquires {
            acquires(p);
        } else {
            p.error("'acquires' not allowed");
        }
    }

    if p.at(T![;]) {
        p.bump(T![;]);
    } else {
        block_expr(p, is_spec);
    }
}

pub(crate) fn opt_ret_type(p: &mut Parser) {
    if p.at(T![:]) {
        let m = p.start();
        p.bump(T![:]);
        types::type_or_recover_until(p, |p| {
            at_item_start(p) || p.at_ts(ts!(T![acquires], T![;], T!['{']))
        });
        m.complete(p, RET_TYPE);
    }
}

// fn signature_end(p: &Parser) -> bool {
//     p.at_ts(ts!(T![;], T!['{']))
// }

pub(crate) fn on_function_modifiers_start(p: &Parser) -> bool {
    match p.current() {
        T![public] => true,
        T![native] => true,
        T![friend] => true,
        T![inline] => true,
        // not a name of a function
        IDENT if p.at_contextual_kw("entry") && !p.nth_at_ts(1, ts!(T!['('], T![<])) => true,
        IDENT if p.at_contextual_kw("package") && !p.nth_at_ts(1, ts!(T!['('], T![<])) => true,
        _ => false,
    }
}

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

pub(crate) fn function_modifier_recovery_set() -> RecoverySet {
    let mut rec_set = RecoverySet::new();
    rec_set
        .with_token_set(T![public] | T![native] | T![friend] | T![inline])
        .with_kw_ident("entry")
        .with_kw_ident("package")
}
