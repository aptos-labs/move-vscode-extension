use crate::ast::{SchemaLit, support};
use crate::{AstNode, ast};

impl ast::IfElseIncludeExpr {
    #[inline]
    pub fn then_schema_lit(&self) -> Option<SchemaLit> {
        support::children(&self.syntax).nth(0)
    }
    #[inline]
    pub fn else_schema_lit(&self) -> Option<SchemaLit> {
        support::children(&self.syntax).nth(1)
    }
}
