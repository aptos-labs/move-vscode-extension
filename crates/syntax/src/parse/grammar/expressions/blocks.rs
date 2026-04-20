use crate::SyntaxKind::{BLOCK_EXPR, EOF, ERROR, EXPR_STMT, LET_STMT, USE_STMT};
use crate::T;
use crate::parse::grammar::expressions::atom::inline_expr;
use crate::parse::grammar::expressions::{opt_initializer_expr, stmt_expr};
use crate::parse::grammar::items::{at_stmt_kw_start, fun, use_item};
use crate::parse::grammar::patterns::pat;
use crate::parse::grammar::specs::predicates::{pragma_stmt, spec_predicate, update_stmt};
use crate::parse::grammar::specs::proofs_and_lemmas::lemma;
use crate::parse::grammar::specs::schemas::{
    apply_schema, global_variable, include_schema, schema_field,
};
use crate::parse::grammar::{attributes, types};
use crate::parse::parser::{CompletedMarker, Marker, Parser};
use std::ops::ControlFlow::Continue;

#[derive(Debug, Copy, Clone)]
pub(crate) enum StmtKind {
    Move,
    Spec,
    Proof,
}

impl StmtKind {
    pub(crate) fn is_spec(&self) -> bool {
        !matches!(self, StmtKind::Move)
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
    p.reset_recovery_set(|p| {
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
        p.with_recovery_token_set(T!['}'], |p| stmt(p, false, kind));
        Continue(())
    });
}

pub(crate) fn stmt(p: &mut Parser, prefer_expr: bool, stmt_kind: StmtKind) {
    // handle attributes
    let mut attrs = attributes::attrs(p);
    if let Some(last_attr) = attrs.pop() {
        p.wrap_with_error(last_attr, "attributes on statements are not allowed");
        if p.at(T!['}']) {
            return;
        }
    }

    // inline use stmt
    if p.at(T![use]) {
        let m = p.start();
        p.with_recovery_set(at_stmt_kw_start(), |p| use_stmt(p, m));
        return;
    }

    if p.at(T![let]) {
        let_stmt(p, stmt_kind.is_spec());
        return;
    }

    if stmt_kind.is_spec() {
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
            let spec_only_stmts = vec![
                schema_field,
                global_variable,
                pragma_stmt,
                update_stmt,
                include_schema,
                apply_schema,
                spec_predicate,
            ];
            if spec_only_stmts.iter().any(|spec_stmt| spec_stmt(p)) {
                return true;
            }
            false
        });
        if is_spec_stmt {
            return;
        }
    }

    if let Some((cm, _)) = p.with_recovery_token(T![;], |p| stmt_expr(p)) {
        if !(p.at(T!['}']) || (prefer_expr && p.at(EOF))) {
            let m = cm.precede(p);
            p.expect(T![;]);
            m.complete(p, EXPR_STMT);
        }
        return;
    }

    p.error_and_bump(&format!("unexpected token {:?}", p.current()));
}

fn let_stmt(p: &mut Parser, allow_post: bool) {
    let m = p.start();
    p.bump(T![let]);
    if allow_post && p.at_contextual_kw_ident("post") {
        p.bump_remap(T![post]);
    }
    let recovery_set = at_stmt_kw_start().with_ts(T![=] | T![;]);
    // let rec_set = item_start_rec_set().with_token_set(T![=] | T![;]);
    p.with_recovery_set(recovery_set.clone(), pat);
    // pat_or_recover(p, recovery_set.clone());
    if p.at(T![:]) {
        p.with_recovery_set(recovery_set, types::type_annotation);
    }

    opt_initializer_expr(p);
    p.expect(T![;]);

    m.complete(p, LET_STMT);
}

pub(crate) fn use_stmt(p: &mut Parser, stmt: Marker) {
    assert!(p.at(T![use]));
    p.bump(T![use]);
    p.with_recovery_set(T![;].into(), |p| use_item::use_speck(p, true));
    p.expect(T![;]);
    stmt.complete(p, USE_STMT);
}
