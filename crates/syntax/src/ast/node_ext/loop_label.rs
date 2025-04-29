use crate::ast;

impl ast::LabelDecl {
    pub fn name_as_string(&self) -> String {
        self.quote_ident_token().to_string()
    }
}

impl ast::Label {
    pub fn name_as_string(&self) -> String {
        self.quote_ident_token().to_string()
    }
}
