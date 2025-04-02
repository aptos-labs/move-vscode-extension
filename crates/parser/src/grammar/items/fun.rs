use crate::grammar::expressions::atom::block_expr;
use crate::grammar::paths::PATH_FIRST;
use crate::grammar::types::path_type_;
use crate::grammar::utils::delimited;
use crate::grammar::{generic_params, item_name_r, opt_ret_type, params};
use crate::parser::Marker;
use crate::token_set::TokenSet;
use crate::SyntaxKind::{ACQUIRES, EOF, FUN, IDENT, SPEC_FUN, SPEC_INLINE_FUN, VISIBILITY_MODIFIER};
use crate::{Parser, SyntaxKind, T};
use std::collections::HashSet;
use crate::grammar::items::item_first;

pub(crate) fn spec_function(p: &mut Parser, m: Marker) {
    p.bump(T![spec]);
    fun_signature(p, true, false);
    m.complete(p, SPEC_FUN);
}

pub(crate) fn spec_inline_function(p: &mut Parser) {
    let m = p.start();
    p.eat(T![native]);
    fun_signature(p, true, false);
    m.complete(p, SPEC_INLINE_FUN);
}

pub(crate) fn function(p: &mut Parser, m: Marker) {
    opt_modifiers(p);
    if p.at(T![fun]) {
        fun_signature(p, false, true);
    } else {
        p.error_and_recover_until("expected 'fun'", item_first);
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
        p.error_and_bump_any(&format!("duplicate modifier '{:?}'", modifier));
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
    delimited(
        p,
        T![,],
        || "unexpected ','".into(),
        |p| p.at(T!['{']) || p.at(T![;]),
        PATH_FIRST,
        |p| {
            path_type_(p, false);
            true
        },
    );
    m.complete(p, ACQUIRES);
}

fn fun_signature(p: &mut Parser, is_spec: bool, allow_acquires: bool) {
    p.bump(T![fun]);

    item_name_r(p);
    // name_r(p, ITEM_KW_RECOVERY_SET);
    // test function_type_params
    // fn foo<T: Clone + Copy>(){}
    generic_params::opt_generic_param_list(p);

    if p.at(T!['(']) {
        params::fun_param_list(p);
    } else {
        p.error("expected function arguments");
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
        // test fn_decl
        // trait T { fn foo(); }
        p.bump(T![;]);
    } else {
        block_expr(p, is_spec);
    }
}

pub(crate) fn on_function_modifiers_start(p: &Parser) -> bool {
    match p.current() {
        T![public] => true,
        T![native] => true,
        T![friend] => true,
        T![inline] => true,
        IDENT if p.at_contextual_kw("entry") => true,
        IDENT if p.at_contextual_kw("package") => true,
        _ => false,
    }
}
