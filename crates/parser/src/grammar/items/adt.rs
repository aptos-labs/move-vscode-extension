use crate::grammar::attributes::ATTRIBUTE_FIRST;
use crate::grammar::items::{item_recovery_set, ITEM_KW_RECOVERY_SET};
use crate::grammar::utils::list;
use crate::grammar::{
    ability, error_block, expressions, generic_params, item_name_r, name, name_r, types,
};
use crate::parser::Marker;
use crate::token_set::TokenSet;
use crate::SyntaxKind::*;
use crate::{Parser, T};

// test struct_item
// struct S {}
pub(super) fn struct_(p: &mut Parser<'_>, m: Marker) {
    p.bump(T![struct]);
    item_name_r(p);
    generic_params::opt_generic_param_list(p);
    opt_abilities_list(p);
    match p.current() {
        T!['{'] => {
            struct_field_list(p);
            opt_abilities_list_with_semicolon(p);
        }
        // test unit_struct
        // struct S;
        T![;] => {
            p.bump(T![;]);
        }
        // test tuple_struct
        // struct S(String, usize);
        T!['('] => {
            tuple_field_list(p);
            // test tuple_struct_where
            // struct S<T>(T) where T: Clone;
            // generic_params::opt_where_clause(p);
            opt_abilities_list(p);
            p.expect(T![;]);
        }
        _ => p.error("expected `;`, `{`, or `(`"),
    }
    // opt_abilities_list(p);
    m.complete(p, STRUCT);
}

fn opt_abilities_list_with_semicolon(p: &mut Parser) {
    let has_postfix_abilities = opt_abilities_list(p);
    if has_postfix_abilities {
        p.expect(T![;]);
    }
}

fn opt_abilities_list(p: &mut Parser) -> bool {
    if p.at(IDENT) && p.at_contextual_kw("has") {
        let m = p.start();
        p.bump_remap(T![has]);

        let delim = T![,];
        while !p.at(T!['{']) && !p.at(EOF) {
            if p.at(delim) {
                // Recover if an argument is missing and only got a delimiter,
                // e.g. `(a, , b)`.
                let m = p.start();
                p.error("expected ability");
                p.bump(delim);
                m.complete(p, ERROR);
                continue;
            }
            if !ability(p) {
                break;
            }
            if !p.eat(delim) {
                if p.at_ts(ABILITY_FIRST) {
                    p.error(format!("expected {delim:?}"));
                } else {
                    break;
                }
            }
        }
        m.complete(p, ABILITY_LIST);
        return true;
    }
    false
}

pub(crate) const ABILITY_FIRST: TokenSet = TokenSet::new(&[IDENT]);

pub(super) fn enum_(p: &mut Parser<'_>, m: Marker) {
    p.bump_remap(T![enum]);
    item_name_r(p);
    // name_r(p, ITEM_KW_RECOVERY_SET);
    generic_params::opt_generic_param_list(p);
    opt_abilities_list(p);
    if p.at(T!['{']) {
        variant_list(p);
    } else {
        p.error("expected `{`");
    }
    opt_abilities_list_with_semicolon(p);
    m.complete(p, ENUM);
}

pub(crate) fn variant_list(p: &mut Parser<'_>) {
    assert!(p.at(T!['{']));
    let m = p.start();
    p.bump(T!['{']);
    while !p.at(EOF) && !p.at(T!['}']) {
        if p.at(T!['{']) {
            error_block(p, "expected enum variant");
            continue;
        }
        let is_curly = variant(p);
        if !p.at(T!['}']) {
            if is_curly {
                p.eat(T![,]);
            } else {
                p.expect(T![,]);
            }
        }
    }
    p.expect(T!['}']);
    m.complete(p, VARIANT_LIST);

    fn variant(p: &mut Parser<'_>) -> bool {
        let mut curly_braces = false;
        let m = p.start();
        // attributes::outer_attrs(p);
        if p.at(IDENT) {
            name(p);
            match p.current() {
                T!['{'] => {
                    curly_braces = true;
                    struct_field_list(p)
                }
                T!['('] => tuple_field_list(p),
                _ => (),
            }

            // test variant_discriminant
            // enum E { X(i32) = 10 }
            // if p.eat(T![=]) {
            //     expressions::expr(p);
            // }
            m.complete(p, VARIANT);
        } else {
            m.abandon(p);
            p.err_and_bump("expected enum variant");
        }
        curly_braces
    }
}

// test record_field_list
// struct S { a: i32, b: f32 }
pub(crate) fn struct_field_list(p: &mut Parser<'_>) {
    assert!(p.at(T!['{']));
    let m = p.start();
    p.bump(T!['{']);
    while !p.at(T!['}']) && !p.at(EOF) {
        if p.at(T!['{']) {
            error_block(p, "expected field");
            continue;
        }
        struct_field(p);
        if !p.at(T!['}']) {
            p.expect(T![,]);
        }
    }
    p.expect(T!['}']);
    m.complete(p, STRUCT_FIELD_LIST);

    fn struct_field(p: &mut Parser<'_>) {
        let m = p.start();
        // test record_field_attrs
        // struct S { #[attr] f: f32 }
        // attributes::outer_attrs(p);
        // opt_visibility(p, false);
        if p.at(IDENT) {
            name(p);
            p.expect(T![:]);
            types::type_(p);
            m.complete(p, STRUCT_FIELD);
        } else {
            m.abandon(p);
            p.err_and_bump("expected field declaration");
        }
    }
}

const TUPLE_FIELD_FIRST: TokenSet =
    types::TYPE_FIRST.union(ATTRIBUTE_FIRST)/*.union(VISIBILITY_FIRST)*/;

// test_err tuple_field_list_recovery
// struct S(struct S;
// struct S(A,,B);
fn tuple_field_list(p: &mut Parser<'_>) {
    assert!(p.at(T!['(']));
    let m = p.start();
    list(
        p,
        T!['('],
        T![')'],
        T![,],
        || "expected tuple field".into(),
        TUPLE_FIELD_FIRST,
        |p| {
            let m = p.start();
            // test tuple_field_attrs
            // struct S (#[attr] f32);
            // attributes::outer_attrs(p);
            let has_vis = false;
            // let has_vis = opt_visibility(p, true);
            if !p.at_ts(types::TYPE_FIRST) {
                p.error("expected a type");
                if has_vis {
                    m.complete(p, ERROR);
                } else {
                    m.abandon(p);
                }
                return false;
            }
            types::type_(p);
            m.complete(p, TUPLE_FIELD);
            true
        },
    );
    m.complete(p, TUPLE_FIELD_LIST);
}
