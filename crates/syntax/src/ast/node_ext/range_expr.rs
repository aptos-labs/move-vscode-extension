use crate::ast::support;
use crate::{AstNode, ast};

impl ast::RangeExpr {
    pub fn start_expr(&self) -> Option<ast::Expr> {
        support::children(self.syntax()).nth(0)
    }

    pub fn end_expr(&self) -> Option<ast::Expr> {
        support::children(self.syntax()).nth(1)
    }
}
