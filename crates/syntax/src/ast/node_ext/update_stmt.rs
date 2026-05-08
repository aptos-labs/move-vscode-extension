use crate::ast::{Expr, support};
use crate::{AstNode, ast};

impl ast::UpdateStmt {
    #[inline]
    pub fn lhs_expr(&self) -> Option<Expr> {
        support::children(self.syntax()).next()
    }
    #[inline]
    pub fn initializer_expr(&self) -> Option<Expr> {
        support::children(self.syntax()).nth(1)
    }
}
