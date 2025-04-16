use crate::ast;

impl ast::Schema {
    pub fn schema_fields_as_bindings(&self) -> Vec<ast::IdentPat> {
        let schema_field_stmts = self
            .spec_block()
            .map(|b| b.schema_field_stmts())
            .unwrap_or_default();
        schema_field_stmts
            .into_iter()
            .filter_map(|f| f.ident_pat())
            .collect()
    }
}
