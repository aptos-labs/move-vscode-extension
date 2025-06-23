use crate::parse::grammar::expressions::atom::{block_expr, condition};
use crate::parse::grammar::expressions::{expr, expr_bp, opt_initializer_expr, Restrictions};
use crate::parse::grammar::items::item_start_rec_set;
use crate::parse::grammar::paths::type_path;
use crate::parse::grammar::patterns::ident_pat;
use crate::parse::grammar::specs::predicates::opt_predicate_property_list;
use crate::parse::grammar::utils::delimited_with_recovery;
use crate::parse::grammar::{name, name_or_recover, name_ref, type_params, types};
use crate::parse::parser::{CompletedMarker, Marker, Parser};
use crate::parse::recovery_set::RecoveryToken;
use crate::parse::token_set::TokenSet;
use crate::SyntaxKind::*;
use crate::T;

pub(crate) fn schema(p: &mut Parser, m: Marker) {
    assert!(p.at_contextual_kw_ident("schema"));
    p.bump_remap(T![schema]);
    p.with_recovery_set(item_start_rec_set(), |p| {
        name_or_recover(p, item_start_rec_set());
        type_params::opt_type_param_list(p);
    });
    // name_or_recover(p, at_item_start);
    // type_params::opt_type_param_list(p);
    block_expr(p, true);
    m.complete(p, SCHEMA);
}

pub(crate) fn schema_field(p: &mut Parser) -> bool {
    let m = p.start();
    if p.at(IDENT) && p.at_contextual_kw("local") {
        p.bump_remap(T![local]);
    }
    ident_pat(p);
    // patterns::ident_pat(p);
    if p.at(T![:]) {
        types::type_annotation(p);
    } else {
        m.abandon_with_rollback(p);
        return false;
    }
    p.expect(T![;]);
    m.complete(p, SCHEMA_FIELD);
    true
}

pub(crate) fn global_variable(p: &mut Parser) -> bool {
    let m = p.start();
    if p.at_contextual_kw_ident("global") {
        p.bump_remap(T![global]);
    }
    name(p);
    // patterns::ident_pat(p);
    type_params::opt_type_param_list(p);
    if p.at(T![:]) {
        types::type_annotation(p);
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
    include_schema_expr(p);
    p.expect(T![;]);
    m.complete(p, INCLUDE_SCHEMA);
    true
}

const INCLUDE_SCHEMA_RECOVERY_SET: TokenSet = TokenSet::new(&[T![;], T!['}']]);

fn include_schema_expr(p: &mut Parser) -> Option<()> {
    if p.at(T![if]) {
        include_if_else_expr(p);
        return Some(());
    }

    let parent_pos = p.event_pos();
    // allow all ops besides '==>'
    let lhs_expr = inner_expr(p, 2)?;
    if p.at(T![==>]) {
        let m = lhs_expr.precede(p);
        p.bump(T![==>]);
        schema_lit(p);
        m.complete(p, IMPLY_INCLUDE_EXPR);
        return Some(());
    }
    lhs_expr.abandon_with_rollback(p, parent_pos);

    // allow all ops besides '&&'
    let parent_pos = p.event_pos();
    let lhs_expr = inner_expr(p, 6)?;

    let at_amp = p.at(T![&&]);
    lhs_expr.abandon_with_rollback(p, parent_pos);

    if at_amp {
        let m = p.start();
        schema_lit(p);
        if !p.at(T![&&]) {
            p.error_and_recover("expected schema lit", INCLUDE_SCHEMA_RECOVERY_SET);
            return None;
        }
        p.bump(T![&&]);
        schema_lit(p);
        m.complete(p, AND_INCLUDE_EXPR);
        return Some(());
    }

    let m = p.start();
    schema_lit(p);
    m.complete(p, SCHEMA_INCLUDE_EXPR);

    Some(())
}

fn inner_expr(p: &mut Parser, bp: u8) -> Option<CompletedMarker> {
    let cm = expr_bp(p, None, Restrictions::default(), bp).map(|it| it.0);
    if cm.is_none() {
        p.error_and_recover("expected expression", INCLUDE_SCHEMA_RECOVERY_SET);
    }
    cm
}

fn include_if_else_expr(p: &mut Parser) -> CompletedMarker {
    let m = p.start();
    p.bump(T![if]);
    condition(p);
    schema_lit(p);
    if p.expect(T![else]) {
        schema_lit(p);
    }
    m.complete(p, IF_ELSE_INCLUDE_EXPR)
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
    p.expect(T![;]);
    m.complete(p, APPLY_SCHEMA);
    true
}

fn apply_to(p: &mut Parser) {
    let m = p.start();
    p.bump_remap(T![to]);
    p.with_recovery_token(RecoveryToken::from("except"), wildcard_pattern_list);
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
    delimited_with_recovery(p, wildcard_pattern, T![,], "expected function pattern", None);
}

fn wildcard_pattern(p: &mut Parser) -> bool {
    let m = p.start();
    opt_wildcard_pattern_modifier(p);
    if !wildcard_ident(p) {
        m.abandon_with_rollback(p);
        return false;
    }
    type_params::opt_type_param_list(p);
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
                p.error_and_bump("duplicate modifier 'public'");
                continue;
            }
            found = true;
            p.bump(T![public]);
            all_modifiers = all_modifiers.into_iter().filter(|m| *m != T![public]).collect();
            continue;
        }
        if p.at_contextual_kw_ident("internal") {
            if !all_modifiers.contains(&T![internal]) {
                p.error_and_bump("duplicate modifier 'internal'");
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

fn schema_lit(p: &mut Parser) -> CompletedMarker {
    let m = p.start();
    type_path(p);

    if p.at(T!['{']) {
        let m = p.start();
        p.bump(T!['{']);
        delimited_with_recovery(
            p,
            |p| {
                if !p.at(IDENT) {
                    return false;
                }
                let m = p.start();
                if p.nth_at(1, T![:]) {
                    name_ref(p);
                    p.expect(T![:]);
                }
                expr(p);
                m.complete(p, SCHEMA_LIT_FIELD);
                true
            },
            T![,],
            "expected identifier",
            Some(T!['}']),
        );
        p.expect(T!['}']);
        m.complete(p, SCHEMA_LIT_FIELD_LIST);
    }
    m.complete(p, SCHEMA_LIT)
}
