use crate::ast::{SchemaLit, support};
use crate::{AstNode, ast};

impl ast::AndIncludeExpr {
    pub fn left_schema_lit(&self) -> Option<SchemaLit> {
        support::children(self.syntax()).nth(0)
    }

    pub fn right_schema_lit(&self) -> Option<SchemaLit> {
        support::children(self.syntax()).nth(1)
    }
}
