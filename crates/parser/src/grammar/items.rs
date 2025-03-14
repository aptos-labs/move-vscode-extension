mod adt;
pub(crate) mod use_item;

use crate::grammar::expressions::atom::block_expr;
use crate::grammar::expressions::expr;
use crate::grammar::paths::{use_path, PATH_FIRST};
use crate::grammar::specs::schemas::schema;
use crate::grammar::types::path_type_;
use crate::grammar::utils::delimited;
use crate::grammar::{
    attributes, error_block, generic_params, item_name_r, name_ref, opt_ret_type, params, types,
};
use crate::parser::{Marker, Parser};
use crate::token_set::TokenSet;
use crate::SyntaxKind::*;
use crate::{SyntaxKind, T};
use std::ops::Index;

// test mod_contents
// fn foo() {}
// macro_rules! foo {}
// foo::bar!();
// super::baz! {}
// struct S;
pub(super) fn mod_contents(p: &mut Parser, stop_on_r_curly: bool) {
    // attributes::inner_attrs(p);
    while !p.at(EOF) && !(p.at(T!['}']) && stop_on_r_curly) {
        item(p, stop_on_r_curly);
    }
    // while !p.at(EOF) && !(p.at(T!['}']) && stop_on_r_curly) {
    // }
    // stmt(p, StmtWithSemi::Yes, true);
}

pub(super) fn item(p: &mut Parser, stop_on_r_curly: bool) {
    let m = p.start();

    attributes::outer_attrs(p);
    let m = match opt_item(p, m) {
        Ok(()) => {
            if p.at(T![;]) {
                p.err_and_bump(
                    "expected item, found `;`\n\
                     consider removing this semicolon",
                );
            }
            return;
        }
        Err(m) => m,
    };

    // if paths::is_use_path_start(p) {
    //     match macro_call(p) {
    //         BlockLike::Block => (),
    //         BlockLike::NotBlock => {
    //             p.expect(T![;]);
    //         }
    //     }
    //     m.complete(p, MACRO_CALL);
    //     return;
    // }

    m.abandon(p);
    match p.current() {
        T!['{'] => error_block(p, "expected an item"),
        T!['}'] if !stop_on_r_curly => {
            let e = p.start();
            p.error("unmatched `}`");
            p.bump(T!['}']);
            e.complete(p, ERROR);
        }
        EOF | T!['}'] => p.error("expected an item"),
        _ => p.err_and_bump("expected an item"),
    }
}

/// Try to parse an item, completing `m` in case of success.
pub(super) fn opt_item(p: &mut Parser, m: Marker) -> Result<(), Marker> {
    // test_err pub_expr
    // fn foo() { pub 92; }
    // let has_visibility = opt_visibility(p, false);

    let m = match opt_item_without_modifiers(p, m) {
        Ok(()) => return Ok(()),
        Err(m) => m,
    };

    // let mut has_mods = false;
    // let mut has_extern = false;

    // modifiers
    // if p.at(T![const]) && p.nth(1) != T!['{'] {
    //     p.eat(T![const]);
    //     has_mods = true;
    // }

    // test_err async_without_semicolon
    // fn foo() { let _ = async {} }
    // if p.at(T![async]) && !matches!(p.nth(1), T!['{'] | T![move] | T![|]) {
    //     p.eat(T![async]);
    //     has_mods = true;
    // }

    // test_err unsafe_block_in_mod
    // fn foo(){} unsafe { } fn bar(){}
    // if p.at(T![unsafe]) && p.nth(1) != T!['{'] {
    //     p.eat(T![unsafe]);
    //     has_mods = true;
    // }

    // if p.at(T![extern]) {
    //     has_extern = true;
    //     has_mods = true;
    //     abi(p);
    // }
    // if p.at(IDENT) && p.at_contextual_kw("auto") && p.nth(1) == T![trait] {
    //     p.bump_remap(T![auto]);
    //     has_mods = true;
    // }

    // test default_item
    // default impl T for Foo {}
    // if p.at(IDENT) && p.at_contextual_kw("default") {
    //     match p.nth(1) {
    //         T![fn] | T![type] | T![const] | T![impl] => {
    //             p.bump_remap(T![default]);
    //             has_mods = true;
    //         }
    //         // test default_unsafe_item
    //         // default unsafe impl T for Foo {
    //         //     default unsafe fn foo() {}
    //         // }
    //         T![unsafe] if matches!(p.nth(2), T![impl] | T![fn]) => {
    //             p.bump_remap(T![default]);
    //             p.bump(T![unsafe]);
    //             has_mods = true;
    //         }
    //         // test default_async_fn
    //         // impl T for Foo {
    //         //     default async fn foo() {}
    //         // }
    //         T![async] => {
    //             let mut maybe_fn = p.nth(2);
    //             let is_unsafe = if matches!(maybe_fn, T![unsafe]) {
    //                 // test default_async_unsafe_fn
    //                 // impl T for Foo {
    //                 //     default async unsafe fn foo() {}
    //                 // }
    //                 maybe_fn = p.nth(3);
    //                 true
    //             } else {
    //                 false
    //             };
    //
    //             if matches!(maybe_fn, T![fn]) {
    //                 p.bump_remap(T![default]);
    //                 p.bump(T![async]);
    //                 if is_unsafe {
    //                     p.bump(T![unsafe]);
    //                 }
    //                 has_mods = true;
    //             }
    //         }
    //         _ => (),
    //     }
    // }

    // test existential_type
    // existential type Foo: Fn() -> usize;
    // if p.at(IDENT) && p.at_contextual_kw("existential") && p.nth(1) == T![type] {
    //     p.bump_remap(T![existential]);
    //     has_mods = true;
    // }

    // items
    match p.current() {
        T![spec] if p.nth_at(1, T![fun]) => spec_function(p, m),

        _ if p.at_ts_fn(on_function_modifiers_start) => function(p, m),
        T![fun] => function(p, m),

        // T![const] if p.nth(1) != T!['{'] => consts::konst(p, m),

        // todo: check for public/native/entry/inline start

        // _ => {
        //     p.error("expected an item");
        //     m.complete(p, ERROR);
        // }
        _ => return Err(m),
    }
    Ok(())
}

fn opt_item_without_modifiers(p: &mut Parser, m: Marker) -> Result<(), Marker> {
    match p.current() {
        T![use] => use_item::use_(p, m),
        T![struct] => adt::struct_(p, m),
        T![const] => const_(p, m),
        T![friend] if !p.nth_at(1, T![fun]) => friend_decl(p, m),
        T![spec] if !p.nth_at(1, T![fun]) => {
            p.bump(T![spec]);
            if p.at(IDENT) && p.at_contextual_kw("schema") {
                schema(p, m);
                return Ok(());
            }
            item_spec(p, m)
        }
        IDENT if p.at_contextual_kw("enum") => adt::enum_(p, m),
        // T![macro] => macro_def(p, m),
        // IDENT if p.at_contextual_kw("macro_rules") && p.nth(1) == BANG => macro_rules(p, m),

        // T![const] if (la == IDENT || la == T![_] || la == T![mut]) => consts::konst(p, m),
        // T![static] => consts::static_(p, m),
        _ => return Err(m),
    };
    Ok(())
}

fn const_(p: &mut Parser, m: Marker) {
    p.bump(T![const]);
    item_name_r(p);
    if p.at(T![:]) {
        types::ascription(p);
    } else {
        p.error("expected type annotation");
    }
    if p.expect(T![=]) {
        expr(p);
    }
    p.expect(T![;]);
    m.complete(p, CONST);
}

// test fn
// fn foo() {}
fn function(p: &mut Parser, m: Marker) {
    let mut all_modifiers = vec![
        T![inline],
        T![entry],
        T![public],
        T![native],
        T![friend],
        T![package],
    ];
    while !p.at(EOF) {
        match p.current() {
            T![public] => {
                let m = p.start();
                all_modifiers = bump_fun_modifier(p, all_modifiers, T![public]);
                opt_visibility_modifier(p, m);
            }
            T![native] => {
                all_modifiers = bump_fun_modifier(p, all_modifiers, T![native]);
            }
            T![friend] => {
                all_modifiers = bump_fun_modifier(p, all_modifiers, T![friend]);
            }
            T![inline] => {
                all_modifiers = bump_fun_modifier(p, all_modifiers, T![inline]);
            }
            IDENT if p.at_contextual_kw("entry") => {
                all_modifiers = bump_fun_modifier(p, all_modifiers, T![entry]);
            }
            IDENT if p.at_contextual_kw("package") => {
                all_modifiers = bump_fun_modifier(p, all_modifiers, T![package]);
            }
            _ => {
                break;
            }
        }
    }
    fun_signature(p, false, true);
    m.complete(p, FUN);
}

fn bump_fun_modifier(
    p: &mut Parser,
    possible_modifiers: Vec<SyntaxKind>,
    kind: SyntaxKind,
) -> Vec<SyntaxKind> {
    if !possible_modifiers.contains(&kind) {
        p.err_and_bump(&format!("duplicate modifier '{:?}'", kind));
        return possible_modifiers;
    }
    p.bump_remap(kind);
    possible_modifiers.into_iter().filter(|m| *m != kind).collect()
}

fn opt_visibility_modifier(p: &mut Parser, m: Marker) {
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
                p.err_recover("expected public modifier", TokenSet::new(&[T![')']]));
            }
        }
        p.expect(T![')']);
    }
    m.complete(p, VISIBILITY_MODIFIER);
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

fn spec_function(p: &mut Parser, m: Marker) {
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

fn item_spec(p: &mut Parser, m: Marker) {
    // p.bump(T![spec]);
    if p.at(T![module]) {
        p.bump(T![module]);
    } else {
        name_ref(p);
        // item_name_r(p);
        // function signature
        generic_params::opt_generic_param_list(p);
        if p.at(T!['(']) {
            params::function_parameter_list(p);
            opt_ret_type(p);
        }
    }
    block_expr(p, true);
    m.complete(p, ITEM_SPEC);
}

fn fun_signature(p: &mut Parser, is_spec: bool, allow_acquires: bool) {
    p.bump(T![fun]);

    item_name_r(p);
    // name_r(p, ITEM_KW_RECOVERY_SET);
    // test function_type_params
    // fn foo<T: Clone + Copy>(){}
    generic_params::opt_generic_param_list(p);

    if p.at(T!['(']) {
        params::function_parameter_list(p);
    } else {
        p.error("expected function arguments");
    }
    // test function_ret_type
    // fn foo() {}
    // fn bar() -> () {}
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

pub(crate) fn friend_decl(p: &mut Parser, m: Marker) {
    p.bump(T![friend]);
    use_path(p);
    p.expect(T![;]);
    m.complete(p, FRIEND);
}

pub(crate) fn item_recovery_set(p: &Parser) -> bool {
    if p.at_ts(ITEM_KW_RECOVERY_SET) {
        return true;
    }
    on_function_modifiers_start(p)
}

fn on_function_modifiers_start(p: &Parser) -> bool {
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

pub(super) const ITEM_KW_RECOVERY_SET: TokenSet = TokenSet::new(&[
    T![fun],
    T![struct],
    T![const],
    T![spec],
    T![public],
    T![friend],
    T![package],
    T![native],
    T![use],
    T![;],
    T!['}'],
]);

// pub(crate) const FUNCTION_VIS_FIRST: TokenSet =
//     TokenSet::new(&[T![public], T![native]]);
