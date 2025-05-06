use crate::ast;

impl ast::Schema {
    pub fn schema_fields(&self) -> Vec<ast::SchemaField> {
        self.spec_block().map(|b| b.schema_fields()).unwrap_or_default()
    }

    // pub fn schema_fields_as_bindings(&self) -> Vec<ast::IdentPat> {
    //     self.schema_fields()
    //         .into_iter()
    //         .filter_map(|f| f.ident_pat())
    //         .collect()
    // }
}
