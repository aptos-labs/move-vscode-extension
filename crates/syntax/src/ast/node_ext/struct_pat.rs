use crate::ast;

impl ast::StructPat {
    pub fn fields(&self) -> Vec<ast::StructPatField> {
        self.struct_pat_field_list()
            .map(|it| it.fields().collect())
            .unwrap_or_default()
    }
}
