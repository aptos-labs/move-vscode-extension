use crate::ast;

impl ast::SchemaLit {
    pub fn fields(&self) -> Vec<ast::SchemaLitField> {
        self.schema_lit_field_list()
            .map(|it| it.fields().collect())
            .unwrap_or_default()
    }
}
