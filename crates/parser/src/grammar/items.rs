mod adt;
pub(crate) mod fun;
pub(crate) mod use_item;

use crate::grammar::expressions::atom::block_expr;
use crate::grammar::expressions::expr;
use crate::grammar::paths::use_path;
use crate::grammar::specs::schemas::schema;
use crate::grammar::{
    attributes, error_block, generic_params, item_name, name_ref_or_bump_until, params, types,
};
use crate::parser::{Marker, Parser};
use crate::token_set::TokenSet;
use crate::SyntaxKind::*;
use crate::T;

// // test mod_contents
// // fn foo() {}
// // macro_rules! foo {}
// // foo::bar!();
// // super::baz! {}
// // struct S;
// pub(super) fn mod_contents(p: &mut Parser) {
//     while !p.at(EOF) && !(p.at(T!['}'])) {
//         item(p);
//     }
// }

pub(crate) fn item_list(p: &mut Parser<'_>) {
    assert!(p.at(T!['{']));
    p.bump(T!['{']);
    while !p.at(EOF) && !(p.at(T!['}'])) {
        item(p);
    }
    p.expect(T!['}']);
}

pub(super) fn item(p: &mut Parser) {
    let m = p.start();
    attributes::outer_attrs(p);

    let m = match opt_item(p, m) {
        Ok(()) => {
            if p.at(T![;]) {
                p.error_and_bump_any(
                    "expected item, found `;`\n\
                     consider removing this semicolon",
                );
            }
            return;
        }
        Err(m) => m,
    };
    m.abandon(p);

    // couldn't find an item
    match p.current() {
        T!['{'] => error_block(p, "expected an item, got a block"),
        // T!['}'] if !stop_on_r_curly => {
        //     let e = p.start();
        //     p.error("unmatched `}`");
        //     p.bump(T!['}']);
        //     e.complete(p, ERROR);
        // }
        T!['}'] => p.error("unexpected '}'"),
        EOF => p.error("unexpected EOF"),

        _ => p.error_and_bump_any(&format!("expected an item, got {:?}", p.current())),
    }
}

/// Try to parse an item, completing `m` in case of success.
pub(super) fn opt_item(p: &mut Parser, m: Marker) -> Result<(), Marker> {
    let m = match try_items_with_no_modifiers(p, m) {
        Ok(()) => return Ok(()),
        Err(m) => m,
    };

    match p.current() {
        T![fun] => fun::function(p, m),
        _ if p.at_ts_fn(fun::on_function_modifiers_start) => fun::function(p, m),

        T![spec] => {
            p.bump(T![spec]);
            if p.at_contextual_kw_ident("schema") {
                schema(p, m);
                return Ok(());
            }
            match p.current() {
                T![fun] => fun::spec_function(p, m),
                _ if p.at_ts_fn(fun::on_function_modifiers_start) => fun::spec_function(p, m),
                _ => item_spec(p, m),
            }
        }

        // T![spec] if !p.nth_at(1, T![fun]) => {
        //     p.bump(T![spec]);
        //     if p.at_contextual_kw_ident("schema") {
        //         schema(p, m);
        //         return Ok(());
        //     }
        //     item_spec(p, m)
        // }
        // T![spec] if p.nth_at(1, T![fun]) => fun::spec_function(p, m),

        // _ => {
        //     p.error("expected an item");
        //     m.complete(p, ERROR);
        // }
        _ => return Err(m),
    }
    Ok(())
}

fn try_items_with_no_modifiers(p: &mut Parser, m: Marker) -> Result<(), Marker> {
    match p.current() {
        T![use] => use_item::use_(p, m),
        T![struct] => adt::struct_(p, m),
        T![const] => const_(p, m),
        T![friend] if !p.nth_at(1, T![fun]) => friend_decl(p, m),
        IDENT if p.at_contextual_kw("enum") => adt::enum_(p, m),
        _ => return Err(m),
    };
    Ok(())
}

fn const_(p: &mut Parser, m: Marker) {
    p.bump(T![const]);
    // name_or_bump_until(p, item_first);

    if !item_name(p) {
        m.complete(p, CONST);
        return;
    }

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

fn item_spec(p: &mut Parser, m: Marker) {
    if p.at(T![module]) {
        p.bump(T![module]);
    } else {
        let ref_exists = {
            let ref_m = p.start();
            let res = name_ref_or_bump_until(p, item_start);
            if res {
                ref_m.complete(p, ITEM_SPEC_REF);
            } else {
                ref_m.abandon(p);
            }
            res
        };
        if !ref_exists {
            m.complete(p, ITEM_SPEC);
            return;
        }
        generic_params::opt_generic_param_list(p);
        if p.at(T!['(']) {
            params::fun_param_list(p);
            fun::opt_ret_type(p);
        }
    }
    block_expr(p, true);
    m.complete(p, ITEM_SPEC);
}

pub(crate) fn friend_decl(p: &mut Parser, m: Marker) {
    p.bump(T![friend]);
    use_path(p);
    p.expect(T![;]);
    m.complete(p, FRIEND);
}

// pub(crate) fn item_first_or_l_curly(p: &Parser) -> bool {
//     item_first(p)
// }

pub(crate) fn block_start(p: &Parser) -> bool {
    p.at(T!['{'])
}

pub(crate) fn item_start(p: &Parser) -> bool {
    p.at_ts(ITEM_KEYWORDS)
        || p.at(T!['}'])
        || fun::on_function_modifiers_start(p)
        || p.at_contextual_kw_ident("enum")
}

const ITEM_KEYWORDS: TokenSet = TokenSet::new(&[
    T![fun],
    T![struct],
    T![const],
    T![spec],
    T![schema],
    T![friend],
    T![use],
]);
