use crate::nameres::scope::{NamedItemsExt, ScopeEntry};
use stdx::itertools::Itertools;
use syntax::ast::HasStmts;
use syntax::ast::node_ext::move_syntax_node::MoveSyntaxElementExt;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::files::{InFile, InFileVecExt};
use syntax::{AstNode, SyntaxNode, ast};

pub fn get_entries_in_blocks(scope: InFile<SyntaxNode>, prev: SyntaxNode) -> Vec<ScopeEntry> {
    use syntax::SyntaxKind::*;

    let mut entries = vec![];

    match scope.value.kind() {
        BLOCK_EXPR => {
            let block_expr = scope.syntax_cast::<ast::BlockExpr>().unwrap();

            let is_msl = scope.value.is_msl_context();
            if is_msl {
                let spec_inline_funs = block_expr.map_ref(|it| it.spec_inline_functions()).flatten();
                entries.extend(spec_inline_funs.to_entries());
            }

            let let_stmts = let_stmts_with_bindings(block_expr);
            let current_let_stmt = prev.clone().cast::<ast::LetStmt>();
            let bindings = let_stmts
                .into_iter()
                .filter(|(let_stmt, _)| {
                    if !is_msl {
                        return let_stmt.syntax().strictly_before(&prev);
                    }
                    if let Some(current_let_stmt) = current_let_stmt.as_ref() {
                        let is_post_visible = current_let_stmt.is_post() || !let_stmt.is_post();
                        return is_post_visible && let_stmt.syntax().strictly_before(&prev);
                    }
                    true
                })
                .collect::<Vec<_>>();

            let binding_entries = bindings.into_iter().rev().flat_map(|(_, bindings)| bindings);

            let binding_entries_with_shadowing =
                binding_entries.unique_by(|e| e.name.clone()).collect::<Vec<_>>();

            entries.extend(binding_entries_with_shadowing);
        }
        MATCH_ARM => {
            // coming from rhs, use pat bindings from lhs
            if !prev.is::<ast::Pat>() {
                let (file_id, match_arm) = scope.map(|it| it.cast::<ast::MatchArm>().unwrap()).unpack();
                let ident_pats = match_arm
                    .pat()
                    .map(|it| it.bindings())
                    .unwrap_or_default()
                    .wrapped_in_file(file_id);
                entries.extend(ident_pats.to_entries());
            }
        }
        _ => {}
    }

    entries
}

fn let_stmts_with_bindings(block: InFile<ast::BlockExpr>) -> Vec<(ast::LetStmt, Vec<ScopeEntry>)> {
    block
        .value
        .let_stmts()
        .map(|let_stmt| {
            let bindings = let_stmt.pat().map(|pat| pat.bindings()).unwrap_or_default();
            (let_stmt, bindings.wrapped_in_file(block.file_id).to_entries())
        })
        .collect()
}
