use crate::SyntaxKind::*;
use crate::T;
use crate::parse::grammar::attributes::ATTRIBUTE_FIRST;
use crate::parse::grammar::items::item_start_rec_set;
use crate::parse::grammar::utils::delimited_with_recovery;
use crate::parse::grammar::{
    abilities_list, attributes, error_block, name_or_recover, type_params, types,
};
use crate::parse::parser::{Marker, Parser};
use crate::parse::recovery_set::RecoverySet;
use crate::parse::token_set::TokenSet;
use std::ops::ControlFlow::Continue;
use std::sync::LazyLock;

pub(super) fn struct_(p: &mut Parser, m: Marker) {
    p.bump(T![struct]);
    name_or_recover(p, adt_name_recovery());
    type_params::opt_type_param_list(p);
    p.with_recovery_token_set(T!['{'] | T!['('], opt_abilities_list);
    match p.current() {
        T!['{'] => {
            p.with_recovery_token(T!['}'], |p| named_field_list(p));
            opt_abilities_list_with_semicolon(p);
        }
        T![;] => {
            p.bump(T![;]);
        }
        T!['('] => {
            tuple_field_list(p);
            p.with_recovery_token_set(T![;], opt_abilities_list);
            p.expect(T![;]);
        }
        _ => p.error("expected `;`, `{`, or `(`"),
    }
    m.complete(p, STRUCT);
}

pub(super) fn enum_(p: &mut Parser, m: Marker) {
    p.bump_remap(T![enum]);

    if !name_or_recover(p, adt_name_recovery()) {
        m.complete(p, ENUM);
        return;
    }
    type_params::opt_type_param_list(p);
    p.with_recovery_token_set(T!['{'], opt_abilities_list);

    if p.at(T!['{']) {
        enum_variant_list(p);
    } else {
        p.error("expected `{`");
    }
    opt_abilities_list_with_semicolon(p);
    m.complete(p, ENUM);
}

pub(crate) fn enum_variant_list(p: &mut Parser) {
    assert!(p.at(T!['{']));
    let m = p.start();
    p.bump(T!['{']);

    p.iterate_to_EOF(T!['}'], |p| {
        let is_curly = enum_variant(p);
        if !p.at(T!['}']) {
            if is_curly {
                p.eat(T![,]);
            } else {
                p.expect(T![,]);
            }
        }
        Continue(())
    });

    p.expect(T!['}']);
    m.complete(p, VARIANT_LIST);
}

fn enum_variant(p: &mut Parser) -> bool {
    let mut curly_braces = false;
    let m = p.start();
    attributes::outer_attrs(p);
    if p.at(IDENT) {
        // name(p);
        name_or_recover(p, TokenSet::EMPTY.into());
        match p.current() {
            T!['{'] => {
                curly_braces = true;
                named_field_list(p)
            }
            T!['('] => tuple_field_list(p),
            _ => (),
        }
        m.complete(p, VARIANT);
    } else {
        m.abandon(p);
        p.error_and_bump("expected enum variant");
    }
    curly_braces
}

fn opt_abilities_list_with_semicolon(p: &mut Parser) {
    let has_postfix_abilities = p.with_recovery_token_set(T![;], opt_abilities_list);
    if has_postfix_abilities {
        p.expect(T![;]);
    }
}

fn opt_abilities_list(p: &mut Parser) -> bool {
    if p.at_contextual_kw_ident("has") {
        p.with_recovery_set(item_start_rec_set(), abilities_list);
        return true;
    }
    false
}

fn named_field_list(p: &mut Parser) {
    assert!(p.at(T!['{']));
    let m = p.start();
    p.bump(T!['{']);
    p.iterate_to_EOF(T!['}'], |p| {
        named_field(p);
        if !p.at(T!['}']) {
            p.expect(T![,]);
        }
        Continue(())
    });
    p.expect(T!['}']);
    m.complete(p, NAMED_FIELD_LIST);
}

fn named_field(p: &mut Parser) -> bool {
    let m = p.start();
    if p.at(IDENT) {
        #[cfg(debug_assertions)]
        let _p = stdx::panic_context::enter(format!("named_field {:?}", p.current_text()));

        name_or_recover(p, TokenSet::EMPTY.into());

        let at_colon = p.eat(T![:]);
        if at_colon {
            p.with_recovery_token(T![,], types::type_);
        } else {
            p.error_and_recover("missing type annotation", RecoverySet::from_ts(T![,] | T![ident]));
        }
        m.complete(p, NAMED_FIELD);
    } else {
        m.abandon(p);
        p.error_and_bump("expected named field declaration");
        return false;
    }
    true
}

fn adt_name_recovery() -> RecoverySet {
    item_start_rec_set()
        .with_token_set(T![<] | T!['{'])
        .with_recovery_token("has".into())
    // item_start_rec_set().with_merged(struct_or_enum_name_rec_set())
}

// fn struct_or_enum_name_rec_set() -> RecoverySet {
//     RecoverySet::new()
//         .with_token_set(T![<] | T!['{'])
//         .with_recovery_token("has".into())
// }

fn struct_enum_recover_at(p: &Parser) -> bool {
    p.at(T![<]) || p.at_contextual_kw_ident("has")
}

const TUPLE_FIELD_FIRST: TokenSet =
    types::TYPE_FIRST.union(ATTRIBUTE_FIRST)/*.union(VISIBILITY_FIRST)*/;

fn tuple_field_list(p: &mut Parser) {
    assert!(p.at(T!['(']));
    let m = p.start();
    p.bump(T!['(']);
    delimited_with_recovery(
        p,
        |p| {
            let em = p.start();
            if !p.at_ts(types::TYPE_FIRST) {
                p.error("expected a type");
                em.abandon(p);
                return false;
            }
            types::type_(p);
            em.complete(p, TUPLE_FIELD);
            true
        },
        T![,],
        "expected tuple field",
        Some(T![')']),
    );
    p.expect(T![')']);
    m.complete(p, TUPLE_FIELD_LIST);
}
