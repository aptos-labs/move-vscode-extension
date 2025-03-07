use crate::ast;

impl ast::Schema {
    pub fn schema_fields_as_bindings(&self) -> Vec<ast::IdentPat> {
        let schema_fields = self.spec_block().map(|b| b.schema_fields()).unwrap_or_default();
        schema_fields.into_iter().filter_map(|f| f.ident_pat()).collect()
    }
}
