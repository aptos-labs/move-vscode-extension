use crate::{ast, AstNode};

impl ast::Enum {
    pub fn variants(&self) -> Vec<ast::Variant> {
        self.variant_list()
            .map(|list| list.variants().collect())
            .unwrap_or_default()
    }
}

impl ast::Variant {
    pub fn enum_(&self) -> ast::Enum {
        let variant_list = self.syntax.parent().unwrap();
        let enum_ = variant_list.parent().unwrap();
        ast::Enum::cast(enum_).unwrap()
    }
}
