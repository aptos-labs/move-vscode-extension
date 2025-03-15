use crate::ast;
use crate::ast::HasStmts;

impl ast::BlockExpr {
    pub fn schema_fields(&self) -> Vec<ast::SchemaField> {
        self.stmts().filter_map(|it| it.schema_field()).collect()
    }

    pub fn spec_inline_functions(&self) -> Vec<ast::SpecInlineFun> {
        self.stmts().filter_map(|it| it.spec_inline_fun()).collect()
    }
}
