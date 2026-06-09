use crate::ast::support;
use crate::{AstNode, ast};

impl ast::RangePat {
    pub fn lhs(&self) -> Option<ast::LiteralPat> {
        support::children(self.syntax()).next()
    }

    pub fn rhs(&self) -> Option<ast::LiteralPat> {
        support::children(self.syntax()).nth(1)
    }
}
