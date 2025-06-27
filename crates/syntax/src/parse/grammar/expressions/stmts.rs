use crate::parse::grammar::expressions::{opt_initializer_expr, stmt_expr};
use crate::parse::grammar::items::{fun, stmt_start_rec_set, use_item};
use crate::parse::grammar::patterns::pat;
use crate::parse::grammar::specs::predicates::{pragma_stmt, spec_predicate, update_stmt};
use crate::parse::grammar::specs::schemas::{
    apply_schema, global_variable, include_schema, schema_field,
};
use crate::parse::grammar::{attributes, types};
use crate::parse::parser::{Marker, Parser};
use crate::SyntaxKind::{EOF, EXPR_STMT, LET_STMT, USE_STMT};
use crate::T;

pub(super) fn stmt(p: &mut Parser, prefer_expr: bool, is_spec: bool) {
    let stmt_m = p.start();

    attributes::outer_attrs(p);

    if p.at(T![let]) {
        let_stmt(p, stmt_m, is_spec);
        return;
    }
    if p.at(T![use]) {
        use_stmt(p, stmt_m);
        return;
    }

    if is_spec {
        if p.at(T![native]) && p.nth_at(1, T![fun]) || p.at(T![fun]) {
            fun::spec_inline_function(p);
            stmt_m.abandon(p);
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
            stmt_m.abandon(p);
            return;
        }
    }

    if let Some((cm, _)) = p.with_recovery_token(T![;], |p| stmt_expr(p, Some(stmt_m))) {
        if !(p.at(T!['}']) || (prefer_expr && p.at(EOF))) {
            let m = cm.precede(p);
            p.expect(T![;]);
            m.complete(p, EXPR_STMT);
        }
        return;
    }

    p.error_and_bump(&format!("unexpected token {:?}", p.current()));
}

fn let_stmt(p: &mut Parser, m: Marker, is_spec: bool) {
    p.bump(T![let]);
    if is_spec && p.at_contextual_kw_ident("post") {
        p.bump_remap(T![post]);
    }
    let recovery_set = stmt_start_rec_set().with_token_set(T![=] | T![;]);
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
    p.bump(T![use]);
    use_item::use_speck(p, true);
    p.expect(T![;]);
    stmt.complete(p, USE_STMT);
}
