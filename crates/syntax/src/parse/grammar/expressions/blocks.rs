use crate::SyntaxKind::{BLOCK_EXPR, ERROR, EXPR_STMT, LET_STMT, USE_STMT};
use crate::T;
use crate::parse::grammar::expressions::atom::inline_expr;
use crate::parse::grammar::expressions::{opt_initializer_expr, top_level_expr_in_stmt};
use crate::parse::grammar::items::{at_stmt_start, fun, use_item};
use crate::parse::grammar::patterns::pattern;
use crate::parse::grammar::specs::predicates::{pragma_stmt, spec_predicate, update_stmt};
use crate::parse::grammar::specs::proofs_and_lemmas::{apply_lemma, lemma};
use crate::parse::grammar::specs::schemas::{
    apply_schema, global_variable, include_schema, schema_field,
};
use crate::parse::grammar::{attributes, types};
use crate::parse::parser::{CompletedMarker, Marker, Parser};
use std::ops::ControlFlow::Continue;
use std::time::Instant;

#[derive(Debug, Copy, Clone)]
pub(crate) enum StmtKind {
    Move,
    Spec,
    Proof,
}

impl StmtKind {
    pub(crate) fn is_spec(&self) -> bool {
        matches!(self, StmtKind::Spec)
    }
}

pub(crate) fn block_or_inline_expr(p: &mut Parser, kind: StmtKind) {
    if p.at(T!['{']) {
        block_expr(p, kind);
    } else {
        inline_expr(p);
    }
}

pub(crate) fn block_expr(p: &mut Parser, kind: StmtKind) -> CompletedMarker {
    assert!(p.at(T!['{']));
    // we're in new block, we can't use recovery set rules from before
    p.reset_recovery(|p| {
        let m = p.start();
        stmt_list(p, kind);
        m.complete(p, BLOCK_EXPR)
    })
}

pub(crate) fn error_block(p: &mut Parser, message: &str) {
    assert!(p.at(T!['{']));
    let m = p.start();
    p.error(message);
    p.bump(T!['{']);
    expr_block_contents(p, StmtKind::Move);
    p.eat(T!['}']);
    m.complete(p, ERROR);
}

pub(crate) fn stmt_list(p: &mut Parser, kind: StmtKind) {
    assert!(p.at(T!['{']));
    p.bump(T!['{']);
    expr_block_contents(p, kind);
    p.expect(T!['}']);
}

pub(super) fn expr_block_contents(p: &mut Parser, kind: StmtKind) {
    p.iterate_to_EOF(T!['}'], |p| {
        if p.at(T![;]) {
            p.bump(T![;]);
            return Continue(());
        }
        p.with_recovery_token_set(T!['}'], |p| stmt(p, kind));
        Continue(())
    });
}

pub(crate) fn stmt(p: &mut Parser, stmt_kind: StmtKind) {
    // handle attributes
    let mut attrs = attributes::attrs(p);
    if let Some(last_attr) = attrs.pop() {
        p.wrap_with_error(last_attr, "attributes on statements are not allowed");
        if p.at(T!['}']) {
            return;
        }
    }

    // allowed in all stmt contexts
    if p.at(T![let]) {
        let_stmt(p, stmt_kind.is_spec());
        return;
    }

    match stmt_kind {
        StmtKind::Move => {
            // inline use stmt
            if p.at(T![use]) {
                let m = p.start();
                p.with_recovery(at_stmt_start(), |p| use_stmt(p, m));
                return;
            }
        }
        StmtKind::Spec => {
            // inline use stmt
            if p.at(T![use]) {
                let m = p.start();
                p.with_recovery(at_stmt_start(), |p| use_stmt(p, m));
                return;
            }

            if p.at(T![native]) && p.nth_at(1, T![fun]) || p.at(T![fun]) {
                fun::spec_inline_function(p);
                return;
            }

            if p.at_contextual_kw_ident("lemma") {
                lemma(p);
                return;
            }

            let is_spec_stmt = p.with_recovery_token(T![;], |p| {
                // enable stmt level items unique to specs
                let allowed_spec_stmts = vec![
                    schema_field,
                    global_variable,
                    pragma_stmt,
                    update_stmt,
                    include_schema,
                    apply_schema,
                    spec_predicate,
                ];
                if allowed_spec_stmts.iter().any(|spec_stmt| spec_stmt(p)) {
                    return true;
                }
                false
            });
            if is_spec_stmt {
                return;
            }
        }
        StmtKind::Proof => {
            let allowed_spec_stmts = vec![apply_lemma, spec_predicate];
            if allowed_spec_stmts.iter().any(|spec_stmt| spec_stmt(p)) {
                return;
            }
        }
    }

    // parse EXPR_STMT
    match p.with_recovery_token(T![;], top_level_expr_in_stmt) {
        Some((cm, blocklike)) => {
            // checks whether it's trailing expr in block
            if p.at(T!['}']) {
                return;
            }
            // wrap `cm` in EXPR_STMT
            let m = cm.precede(p);
            if blocklike.is_block() {
                // after blocks, trailing semicolon is optional
                p.eat(T![;]);
            } else {
                p.expect(T![;]);
            }
            m.complete(p, EXPR_STMT);
        }
        None => {
            p.error_and_bump(&format!("unexpected token {:?}", p.current()));
        }
    }
}

fn let_stmt(p: &mut Parser, allow_post: bool) {
    let m = p.start();
    p.bump(T![let]);
    if allow_post && p.at_contextual_kw_ident("post") {
        p.bump_remap(T![post]);
    }
    let rs = at_stmt_start().with_ts(T![=] | T![;]);
    // pattern
    p.with_recovery(rs.clone(), pattern);
    // : TYPE
    if p.at(T![:]) {
        p.with_recovery(rs, types::type_annotation);
    }
    // = EXPR
    opt_initializer_expr(p);
    p.expect(T![;]);

    m.complete(p, LET_STMT);
}

pub(crate) fn use_stmt(p: &mut Parser, stmt: Marker) {
    assert!(p.at(T![use]));
    p.bump(T![use]);
    p.with_recovery(T![;].into(), |p| use_item::use_speck(p, true));
    p.expect(T![;]);
    stmt.complete(p, USE_STMT);
}
