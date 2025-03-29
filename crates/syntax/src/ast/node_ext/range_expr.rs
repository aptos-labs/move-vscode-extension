use crate::ast::support;
use crate::{ast, AstNode};

impl ast::RangeExpr {
    pub fn start_expr(&self) -> ast::Expr {
        support::children(self.syntax())
            .next()
            .expect("RangeExpr.expr_from is required")
    }

    pub fn end_expr(&self) -> Option<ast::Expr> {
        support::children(self.syntax()).nth(1)
    }
}
