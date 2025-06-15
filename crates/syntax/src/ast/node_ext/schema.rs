use crate::ast;

impl ast::Schema {
    pub fn schema_fields(&self) -> Vec<ast::SchemaField> {
        self.spec_block().map(|b| b.schema_fields()).unwrap_or_default()
    }

    pub fn schema_fields_as_bindings(&self) -> Vec<ast::IdentPat> {
        self.schema_fields()
            .into_iter()
            .filter_map(|f| f.ident_pat())
            .collect()
    }

    pub fn missing_fields(&self, provided_fields: Vec<ast::SchemaLitField>) -> Vec<ast::SchemaField> {
        let declared_fields = self.schema_fields();
        let provided_field_names = provided_fields
            .iter()
            .filter_map(|it| it.field_name())
            .collect::<Vec<_>>();
        let mut missing_fields = vec![];
        for declared_field in declared_fields {
            if let Some(field_name) = declared_field.ident_pat().and_then(|it| it.name()) {
                if !provided_field_names.contains(&field_name.as_string()) {
                    missing_fields.push(declared_field);
                }
            }
        }
        missing_fields
    }
}
