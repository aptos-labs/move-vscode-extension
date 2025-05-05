use crate::grammar::expressions::atom::{block_expr, IDENT_FIRST};
use crate::grammar::expressions::opt_initializer_expr;
use crate::grammar::items::item_start;
use crate::grammar::paths::type_path;
use crate::grammar::specs::predicates::opt_predicate_property_list;
use crate::grammar::utils::{delimited_fn, list};
use crate::grammar::{expressions, generic_params, name, name_or_bump_until, patterns, types};
use crate::parser::Marker;
use crate::token_set::TokenSet;
use crate::SyntaxKind::*;
use crate::{Parser, T};

pub(crate) fn schema(p: &mut Parser, m: Marker) {
    assert!(p.at(IDENT) && p.at_contextual_kw("schema"));
    p.bump_remap(T![schema]);
    name_or_bump_until(p, item_start);
    generic_params::opt_generic_param_list(p);
    block_expr(p, true);
    m.complete(p, SCHEMA);
}

pub(crate) fn schema_field(p: &mut Parser) -> bool {
    let m = p.start();
    if p.at(IDENT) && p.at_contextual_kw("local") {
        p.bump_remap(T![local]);
    }
    patterns::ident_pat(p);
    if p.at(T![:]) {
        types::ascription(p);
    } else {
        m.abandon_with_rollback(p);
        return false;
    }
    p.expect(T![;]);
    m.complete(p, SCHEMA_FIELD_STMT);
    true
}

pub(crate) fn global_variable(p: &mut Parser) -> bool {
    let m = p.start();
    if p.at_contextual_kw_ident("global") {
        p.bump_remap(T![global]);
    }
    name(p);
    // patterns::ident_pat(p);
    generic_params::opt_generic_param_list(p);
    if p.at(T![:]) {
        types::ascription(p);
    } else {
        m.abandon_with_rollback(p);
        return false;
    }
    opt_initializer_expr(p);
    p.expect(T![;]);
    m.complete(p, GLOBAL_VARIABLE_DECL);
    true
}

pub(crate) fn include_schema(p: &mut Parser) -> bool {
    if !p.at_contextual_kw_ident("include") {
        return false;
    }
    let m = p.start();
    p.bump_remap(T![include]);
    opt_predicate_property_list(p);
    if !expressions::expr(p) {
        p.error("expected expression");
    }
    p.expect(T![;]);
    m.complete(p, INCLUDE_SCHEMA);
    true
}

pub(crate) fn apply_schema(p: &mut Parser) -> bool {
    if !p.at_contextual_kw_ident("apply") {
        return false;
    }
    let m = p.start();
    p.bump_remap(T![apply]);
    schema_lit(p);
    if p.at_contextual_kw_ident("to") {
        apply_to(p);
    } else {
        p.error("expected 'to'");
    }
    if p.at_contextual_kw_ident("except") {
        apply_except(p);
    }
    m.complete(p, APPLY_SCHEMA);
    true
}

fn apply_to(p: &mut Parser) {
    let m = p.start();
    p.bump_remap(T![to]);
    wildcard_pattern_list(p);
    m.complete(p, APPLY_TO);
}

fn apply_except(p: &mut Parser) {
    assert!(p.at_contextual_kw_ident("except"));
    let m = p.start();
    p.bump_remap(T![except]);
    wildcard_pattern_list(p);
    m.complete(p, APPLY_EXCEPT);
}

fn wildcard_pattern_list(p: &mut Parser) {
    delimited_fn(
        p,
        T![,],
        || "expected function pattern".into(),
        |p| p.at_contextual_kw_ident("except") || p.at(T![;]),
        |p| p.at_ts(TokenSet::new(&[IDENT, T![*], T![public]])) && !p.at_contextual_kw_ident("except"),
        wildcard_pattern,
    );
}

fn wildcard_pattern(p: &mut Parser) -> bool {
    let m = p.start();
    opt_wildcard_pattern_modifier(p);
    if !wildcard_ident(p) {
        m.abandon_with_rollback(p);
        return false;
    }
    generic_params::opt_generic_param_list(p);
    m.complete(p, WILDCARD_PATTERN);
    true
}

fn opt_wildcard_pattern_modifier(p: &mut Parser) {
    let mut all_modifiers = vec![T![public], T![internal]];
    let mut found = false;
    let m = p.start();
    while !p.at(EOF) {
        if p.at(T![public]) {
            if !all_modifiers.contains(&T![public]) {
                p.error_and_bump_any("duplicate modifier 'public'");
                continue;
            }
            found = true;
            p.bump(T![public]);
            all_modifiers = all_modifiers.into_iter().filter(|m| *m != T![public]).collect();
            continue;
        }
        if p.at_contextual_kw_ident("internal") {
            if !all_modifiers.contains(&T![internal]) {
                p.error_and_bump_any("duplicate modifier 'internal'");
                continue;
            }
            found = true;
            p.bump_remap(T![internal]);
            all_modifiers = all_modifiers.into_iter().filter(|m| *m != T![internal]).collect();
            continue;
        }
        break;
    }
    if !found {
        m.abandon(p);
        return;
    }
    m.complete(p, WILDCARD_PATTERN_MODIFIER);
}

fn wildcard_ident(p: &mut Parser) -> bool {
    let mut n_tokens = 0;
    let mut i = 0;
    loop {
        if p.nth_at(i, IDENT) || p.nth_at(i, T![*]) {
            n_tokens += 1;
        } else {
            break;
        }
        // stop if there's a whitespace next
        if !p.nth_is_jointed_to_next(i) {
            break;
        }
        i += 1;
    }
    if n_tokens > 0 {
        p.bump_remap_many(WILDCARD_IDENT, n_tokens);
    }
    n_tokens != 0
}

fn schema_lit(p: &mut Parser) {
    let m = p.start();
    type_path(p);
    if p.at(T!['{']) {
        list(
            p,
            T!['{'],
            T!['}'],
            T![,],
            || "expected identifier".into(),
            IDENT_FIRST,
            |p| {
                if !p.at(IDENT) {
                    return false;
                }
                let m = p.start();
                p.bump(IDENT);
                if p.at(T![:]) {
                    types::ascription(p);
                }
                m.complete(p, SCHEMA_LIT_FIELD);
                true
            },
        );
    }
    m.complete(p, SCHEMA_LIT);
}
