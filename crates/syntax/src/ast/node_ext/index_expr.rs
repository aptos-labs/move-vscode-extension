use crate::ast::support;
use crate::{AstNode, ast};

impl ast::IndexExpr {
    pub fn base_expr(&self) -> ast::Expr {
        support::children(self.syntax()).next().expect("required")
    }
    pub fn arg_expr(&self) -> Option<ast::Expr> {
        support::children(self.syntax()).nth(1)
    }
}
