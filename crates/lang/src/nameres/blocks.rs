use crate::files::InFileVecExt;
use crate::nameres::scope::{NamedItemsExt, ScopeEntry};
use crate::InFile;
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::ast::HasStmtList;
use syntax::{ast, AstNode, SyntaxNode};

pub fn get_entries_in_blocks(scope: InFile<SyntaxNode>, prev: Option<SyntaxNode>) -> Vec<ScopeEntry> {
    use syntax::SyntaxKind::*;

    match scope.value.kind() {
        BLOCK_EXPR => {
            let block_expr = scope.map(|s| ast::BlockExpr::cast(s).unwrap());
            let prev = prev.unwrap();
            // if prev is not stmt, there's something wrong
            // let prev_stmt = prev
            //     .and_then(|p| ast::Stmt::cast(p))
            //     .expect(&format!("previous scope for block should be a stmt or tail expr, actual {:?}", prev_kind));
            let bindings = visible_let_stmts(block_expr, prev);
            let binding_entries = bindings
                .into_iter()
                .rev()
                .flat_map(|(_, bindings)| bindings)
                .collect();

            // todo: use speck entries

            return binding_entries;
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
