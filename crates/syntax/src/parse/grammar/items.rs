pub(crate) mod adt;
pub(crate) mod fun;
pub(crate) mod item_spec;
pub(crate) mod use_item;

use crate::parse::grammar::expressions::{expr, stmts, EXPR_FIRST};
use crate::parse::grammar::items::fun::{function_modifier_recovery_set, function_modifier_tokens};
use crate::parse::grammar::paths::use_path;
use crate::parse::grammar::patterns::STMT_FIRST;
use crate::parse::grammar::specs::schemas::schema;
use crate::parse::grammar::{attributes, error_block, name_or_recover, types};
use crate::parse::parser::{Marker, Parser};
use crate::parse::recovery_set::RecoverySet;
use crate::parse::token_set::TokenSet;
use crate::SyntaxKind::*;
use crate::{SyntaxKind, T};
use std::ops::ControlFlow;
use std::ops::ControlFlow::Continue;

pub(crate) fn item_list(p: &mut Parser) {
    assert!(p.at(T!['{']));
    p.bump(T!['{']);

    p.iterate_to_EOF(T!['}'], |p| {
        p.with_recovery_token(T!['}'], item);
        Continue(())
    });

    p.expect(T!['}']);
}

pub(super) fn item(p: &mut Parser) {
    let m = p.start();
    attributes::outer_attrs(p);
    let m = match opt_item(p, m) {
        // let m = match opt_item(p, m) {
        Ok(()) => {
            if p.at(T![;]) {
                p.error_and_bump(
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
        _ => p.error_and_bump(&format!("expected an item, got {:?}", p.current())),
    }
}

/// Try to parse an item, completing `m` in case of success.
pub(super) fn opt_item(p: &mut Parser, m: Marker) -> Result<(), Marker> {
    match p.current() {
        T![use] => stmts::use_stmt(p, m),
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

    if !name_or_recover(p, item_start_rec_set().with_token_set(T![;])) {
        m.complete(p, CONST);
        return;
    }

    p.with_recovery_set(item_start_rec_set().with_token_set(T![;]), |p| {
        p.with_recovery_token(T![=], |p| {
            if p.at(T![:]) {
                types::type_annotation(p);
            } else {
                p.error("missing type annotation");
            }
        });
        if p.expect(T![=]) {
            let is_expr = expr(p);
            if !is_expr {
                p.error("expected expression");
            }
        }
    });

    p.expect(T![;]);
    m.complete(p, CONST);
}

pub(crate) fn friend_decl(p: &mut Parser, m: Marker) {
    p.bump(T![friend]);
    use_path(p);
    p.expect(T![;]);
    m.complete(p, FRIEND);
}

pub(crate) fn at_block_start(p: &Parser) -> bool {
    p.at(T!['{'])
}

pub(crate) fn at_item_start(p: &Parser) -> bool {
    p.at_ts(ITEM_KEYWORDS)
        || p.at(T!['}'])
        || fun::on_function_modifiers_start(p)
        || p.at_contextual_kw_ident("enum")
}

// pub(crate) fn item_start_tokens() -> Vec<RecoveryToken> {
//     let mut tokens = vec![];
//     tokens.extend(ITEM_KW_START_LIST.iter().map(|it| it.clone().into()));
//     tokens.extend(function_modifier_tokens());
//     tokens.push("enum".into());
//     tokens
// }

pub(crate) fn item_start_rec_set() -> RecoverySet {
    RecoverySet::new()
        .with_token_set(ITEM_KEYWORDS)
        .with_kw("enum")
        .with_merged(function_modifier_recovery_set())
}

pub(crate) fn stmt_start_rec_set() -> RecoverySet {
    RecoverySet::from_ts(STMT_FIRST)
    // RecoverySet::new()
    //     .with_token_set(ITEM_KEYWORDS)
    //     .with_kw_ident("enum")
    //     .with_merged(function_modifier_recovery_set())
}

const ITEM_KW_START_LIST: &[SyntaxKind] = &[
    T![struct],
    T![fun],
    T![const],
    T![spec],
    // T![schema],
    T![friend],
    T![use],
];

const ITEM_KEYWORDS: TokenSet = TokenSet::new(ITEM_KW_START_LIST);
