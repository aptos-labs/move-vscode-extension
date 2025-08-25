use crate::ast;

impl ast::NamedAddress {
    pub fn name(&self) -> String {
        self.ident_token().to_string()
    }
}
