pub(crate) mod adt;
pub(crate) mod fun;
pub(crate) mod item_spec;
pub(crate) mod use_item;

use crate::parse::grammar::expressions::expr;
use crate::parse::grammar::paths::use_path;
use crate::parse::grammar::specs::schemas::schema;
use crate::parse::grammar::{attributes, error_block, item_name_or_recover, types};
use crate::parse::parser::{Marker, Parser};
use crate::parse::token_set::TokenSet;
use crate::SyntaxKind::*;
use crate::{ts, T};

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

pub(crate) fn item_list(p: &mut Parser) {
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
                p.bump_with_error(
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
        _ => p.bump_with_error(&format!("expected an item, got {:?}", p.current())),
    }
}

/// Try to parse an item, completing `m` in case of success.
pub(super) fn opt_item(p: &mut Parser, m: Marker) -> Result<(), Marker> {
    match p.current() {
        T![use] => use_item::use_stmt(p, m),
        T![struct] => adt::struct_(p, m),
        T![const] => const_(p, m),
        T![friend] if !p.nth_at(1, T![fun]) => friend_decl(p, m),
        IDENT if p.at_contextual_kw("enum") => adt::enum_(p, m),

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
                _ => item_spec::item_spec(p, m),
            }
        }
        _ => return Err(m),
    };
    Ok(())
}

fn const_(p: &mut Parser, m: Marker) {
    p.bump(T![const]);
    // name_or_bump_until(p, item_first);

    if !item_name_or_recover(p, |p| p.at(T![;])) {
        m.complete(p, CONST);
        return;
    }

    p.with_recover_fn(
        |p| at_item_start(p) || p.at(T![;]),
        |p| {
            p.with_recover_t(T![=], |p| {
                if p.at(T![:]) {
                    // p.with_recover_ts(ts!(T![;]), types::ascription);
                    types::ascription(p);
                    // p.with_recover_fn(|p| at_item_start(p) || p.at_ts(ts!(T![;])), types::ascription);
                    // types::ascription(p);
                } else {
                    p.error("expected type annotation");
                }
            });
            if p.expect(T![=]) {
                expr(p);
            }
        },
    );

    // if p.at(T![:]) {
    //     // p.with_recover_ts(ts!(T![;]), types::ascription);
    //     p.with_recover_fn(|p| at_item_start(p) || p.at_ts(ts!(T![;])), types::ascription);
    //     // types::ascription(p);
    // } else {
    //     p.error("expected type annotation");
    // }
    // if p.expect(T![=]) {
    //     expr(p);
    // }
    p.expect(T![;]);
    m.complete(p, CONST);
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

pub(crate) fn at_block_start(p: &Parser) -> bool {
    p.at(T!['{'])
}

pub(crate) fn at_item_start(p: &Parser) -> bool {
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
