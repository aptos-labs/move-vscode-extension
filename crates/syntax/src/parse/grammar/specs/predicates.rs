use crate::parse::grammar::expressions::atom::EXPR_FIRST;
use crate::parse::grammar::expressions::{expr, opt_initializer_expr, Restrictions};
use crate::parse::grammar::utils::{delimited, list};
use crate::parse::grammar::{expressions, type_params};
use crate::parse::parser::Parser;
use crate::parse::token_set::TokenSet;
use crate::SyntaxKind::*;
use crate::T;

pub(crate) fn spec_predicate(p: &mut Parser) -> bool {
    condition_predicates(p)
        || aborts_if_predicate(p)
        || aborts_with_predicate(p)
        || invariant_predicate(p)
        || axiom_predicate(p)
        || emits_predicate(p)
}

pub(crate) fn condition_predicates(p: &mut Parser) -> bool {
    let kw = match p.current() {
        IDENT if p.at_contextual_kw("assume") => T![assume],
        IDENT if p.at_contextual_kw("assert") => T![assert],
        IDENT if p.at_contextual_kw("requires") => T![requires],
        IDENT if p.at_contextual_kw("ensures") => T![ensures],
        IDENT if p.at_contextual_kw("decreases") => T![decreases],
        IDENT if p.at_contextual_kw("modifies") => T![modifies],
        _ => {
            return false;
        }
    };
    let m = p.start();
    p.bump_remap(kw);
    opt_predicate_property_list(p);
    expect_expr(p);
    p.eat(T![;]);
    m.complete(p, SPEC_PREDICATE_STMT);
    true
}

pub(crate) fn aborts_if_predicate(p: &mut Parser) -> bool {
    if !p.at_contextual_kw_ident("aborts_if") {
        return false;
    }
    let m = p.start();
    p.bump_remap(T![aborts_if]);
    opt_predicate_property_list(p);
    expect_expr(p);
    if p.at_contextual_kw_ident("with") {
        let m = p.start();
        p.bump_remap(T![with]);
        expect_expr(p);
        m.complete(p, ABORTS_IF_WITH);
    }
    p.eat(T![;]);
    m.complete(p, ABORTS_IF_STMT);
    true
}

pub(crate) fn emits_predicate(p: &mut Parser) -> bool {
    if !p.at_contextual_kw_ident("emits") {
        return false;
    }
    let m = p.start();
    p.bump_remap(T![emits]);
    opt_predicate_property_list(p);
    expr(p);
    if p.at_contextual_kw_ident("to") {
        p.bump_remap(T![to]);
    } else {
        p.error("expected 'to'");
        m.complete(p, EMITS_STMT);
        return true;
    }
    expr(p);
    opt_emits_condition(p);
    m.complete(p, EMITS_STMT);
    true
}

pub(crate) fn opt_emits_condition(p: &mut Parser) {
    if !p.at(T![if]) {
        return;
    }
    let m = p.start();
    p.bump(T![if]);
    expr(p);
    m.complete(p, EMITS_CONDITION);
}

pub(crate) fn invariant_predicate(p: &mut Parser) -> bool {
    if !p.at(T![invariant]) {
        return false;
    }
    let m = p.start();
    p.bump(T![invariant]);
    type_params::opt_type_param_list(p);
    if p.at_contextual_kw_ident("update") {
        p.bump_remap(T![update]);
    }
    opt_predicate_property_list(p);
    expect_expr(p);
    m.complete(p, INVARIANT_STMT);
    true
}

pub(crate) fn aborts_with_predicate(p: &mut Parser) -> bool {
    if !p.at_contextual_kw_ident("aborts_with") {
        return false;
    }
    let m = p.start();
    p.bump_remap(T![aborts_with]);
    opt_predicate_property_list(p);
    delimited(
        p,
        T![,],
        || "expected expression".into(),
        |p| p.at(T![;]) || p.at(T!['}']),
        EXPR_FIRST,
        expr,
    );
    // comma_separated_list(
    //     p,
    //     "expected expression",
    //     |p| p.at(T![;]) || p.at(T!['}']),
    //     EXPR_FIRST,
    //     expr,
    // );
    m.complete(p, ABORTS_WITH_STMT);
    true
}

pub(crate) fn axiom_predicate(p: &mut Parser) -> bool {
    if !p.at_contextual_kw_ident("axiom") {
        return false;
    }
    let m = p.start();
    p.bump_remap(T![axiom]);
    type_params::opt_type_param_list(p);
    opt_predicate_property_list(p);
    expect_expr(p);
    m.complete(p, AXIOM_STMT);
    true
}

pub(crate) fn update_stmt(p: &mut Parser) -> bool {
    if !p.at_contextual_kw_ident("update") {
        return false;
    }
    let m = p.start();
    p.bump_remap(T![update]);
    if let Some((_, _)) = expressions::lhs(p, Restrictions::default()) {
        if p.eat(T![=]) {
            if !expr(p) {
                p.error("expected expression");
            }
        } else {
            p.error("expected '='");
        }
        m.complete(p, UPDATE_STMT);
    } else {
        m.abandon(p);
    }
    true
}

pub(crate) fn pragma_stmt(p: &mut Parser) -> bool {
    if !p.at_contextual_kw_ident("pragma") {
        return false;
    }
    let m = p.start();
    p.bump_remap(T![pragma]);
    delimited(
        p,
        T![,],
        || "expected attribute".into(),
        |p| p.at(T![;]) || p.at(T!['}']),
        TokenSet::new(&[IDENT, T![friend]]),
        |p| {
            let m = p.start();
            match p.current() {
                T![friend] => {
                    // special case for pragma
                    p.bump_remap(IDENT);
                }
                IDENT => p.bump(IDENT),
                _ => {
                    m.abandon(p);
                    return false;
                }
            }
            opt_initializer_expr(p);
            m.complete(p, PRAGMA_ATTR_ITEM);
            true
        },
    );
    p.expect(T![;]);
    m.complete(p, PRAGMA_STMT);
    true
}

pub(crate) fn opt_predicate_property_list(p: &mut Parser) -> bool {
    if !p.at(T!['[']) {
        return false;
    }
    let m = p.start();
    list(
        p,
        T!['['],
        T![']'],
        T![,],
        || "expected identifier".into(),
        TokenSet::new(&[IDENT]),
        |p| {
            let m = p.start();
            let found = p.eat(IDENT);
            if p.at(T![=]) {
                p.bump(T![=]);
                expressions::atom::literal(p);
            }
            m.complete(p, SPEC_PREDICATE_PROPERTY);
            found
        },
    );
    m.complete(p, SPEC_PREDICATE_PROPERTY_LIST);
    true
}

pub(super) fn expect_expr(p: &mut Parser) {
    if !expr(p) {
        p.error("expected expression");
    }
}

pub(crate) fn on_predicate_start(p: &Parser) -> bool {
    let predicate_keywords = &[
        "assert",
        "assume",
        "requires",
        "decreases",
        "ensures",
        "modifies",
        "include",
        "apply",
        "aborts_if",
        "aborts_with",
        "emits",
        "axiom",
        "pragma",
    ];
    p.at(T![invariant])
        // inline functions
        || p.at(T![native])
        || p.at(T![fun])
        || predicate_keywords.iter().any(|kw| p.at_contextual_kw_ident(kw))
}
