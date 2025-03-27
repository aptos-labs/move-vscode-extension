use crate::ast;

impl ast::StructLit {
    pub fn fields(&self) -> Vec<ast::StructLitField> {
        self.struct_lit_field_list()
            .map(|it| it.fields().collect())
            .unwrap_or_default()
    }
}
