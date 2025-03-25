use crate::InFile;
use crate::files::InFileVecExt;
use crate::nameres::scope::{NamedItemsExt, ScopeEntry};
use stdx::itertools::Itertools;
use syntax::ast::HasStmts;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::{AstNode, SyntaxNode, ast};

pub fn get_entries_in_blocks(scope: InFile<SyntaxNode>, prev: Option<SyntaxNode>) -> Vec<ScopeEntry> {
    use syntax::SyntaxKind::*;

    match scope.value.kind() {
        BLOCK_EXPR => {
            let block_expr = scope.map(|s| ast::BlockExpr::cast(s).unwrap());
            let prev = prev.unwrap();

            let bindings = visible_let_stmts(block_expr, prev);
            let binding_entries = bindings.into_iter().rev().flat_map(|(_, bindings)| bindings);

            let binding_entries_with_shadowing =
                binding_entries.unique_by(|e| e.name.clone()).collect::<Vec<_>>();

            return binding_entries_with_shadowing;
        }
        // todo: spec block expr
        _ => {}
    }

    vec![]
}

fn visible_let_stmts(
    block: InFile<ast::BlockExpr>,
    prev: SyntaxNode,
) -> Vec<(ast::LetStmt, Vec<ScopeEntry>)> {
    block
        .value
        .let_stmts()
        .filter(|let_stmt| let_stmt.syntax().strictly_before(&prev))
        .map(|let_stmt| {
            let bindings = let_stmt.pat().map(|pat| pat.bindings()).unwrap_or_default();
            (let_stmt, bindings.wrapped_in_file(block.file_id).to_entries())
        })
        .collect()
}
