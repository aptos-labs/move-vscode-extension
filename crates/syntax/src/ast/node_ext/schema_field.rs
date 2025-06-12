use crate::ast;

impl ast::SchemaField {
    pub fn name(&self) -> Option<ast::Name> {
        self.ident_pat().and_then(|it| it.name())
    }
}
