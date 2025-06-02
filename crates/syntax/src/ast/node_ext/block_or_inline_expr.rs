use crate::ast;
use crate::ast::HasStmts;

impl ast::BlockOrInlineExpr {
    pub fn stmts(&self) -> Vec<ast::Stmt> {
        match self {
            ast::BlockOrInlineExpr::BlockExpr(block_expr) => block_expr.stmts().collect(),
            ast::BlockOrInlineExpr::InlineExpr(_) => vec![],
        }
    }

    pub fn tail_expr(&self) -> Option<ast::Expr> {
        match self {
            ast::BlockOrInlineExpr::BlockExpr(block_expr) => block_expr.tail_expr(),
            ast::BlockOrInlineExpr::InlineExpr(inline_expr) => inline_expr.expr(),
        }
    }
}
