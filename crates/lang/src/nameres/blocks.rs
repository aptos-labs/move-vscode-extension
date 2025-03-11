use crate::nameres::scope::{NamedItemsExt, ScopeEntry};
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::ast::HasStmtList;
use syntax::{ast, AstNode, SyntaxNode};

pub fn get_entries_in_blocks(scope: SyntaxNode, prev: Option<SyntaxNode>) -> Vec<ScopeEntry> {
    use syntax::SyntaxKind::*;

    match scope.kind() {
        BLOCK_EXPR => {
            let block_expr = ast::BlockExpr::cast(scope).unwrap();
            // if prev is not stmt, there's something wrong
            let prev_stmt = prev
                .and_then(|p| ast::Stmt::cast(p))
                .expect("previous scope for block should be a stmt or tail expr");
            let bindings = visible_let_stmts(block_expr, prev_stmt);
            let binding_entries = bindings
                .into_iter()
                .rev()
                .flat_map(|(stmt, bindings)| bindings)
                .collect();
            return binding_entries;
        }
        // todo: spec block expr
        _ => {}
    }

    vec![]
}

fn visible_let_stmts(
    block: ast::BlockExpr,
    currently_at: ast::Stmt,
) -> Vec<(ast::LetStmt, Vec<ScopeEntry>)> {
    block
        .let_stmts()
        .filter(|let_stmt| let_stmt.syntax().strictly_before(currently_at.syntax()))
        .map(|let_stmt| {
            let bindings = let_stmt.pat().map(|pat| pat.bindings()).unwrap_or_default();
            (let_stmt, bindings.to_entries())
        })
        .collect()
}
