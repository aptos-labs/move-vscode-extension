use crate::ast::{support, SchemaLit};
use crate::{ast, AstNode};

impl ast::AndIncludeExpr {
    pub fn left_schema_lit(&self) -> Option<SchemaLit> {
        support::children(self.syntax()).nth(0)
    }

    pub fn right_schema_lit(&self) -> Option<SchemaLit> {
        support::children(self.syntax()).nth(1)
    }
}
