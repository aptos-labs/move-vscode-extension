use crate::ast::NamedElement;
use crate::{ast, AstNode};
use std::collections::HashSet;

impl ast::Enum {
    pub fn variants(&self) -> Vec<ast::Variant> {
        self.variant_list()
            .map(|list| list.variants().collect())
            .unwrap_or_default()
    }
}

impl ast::Variant {
    pub fn named_fields(&self) -> Vec<ast::NamedField> {
        self.field_list().map(|it| it.named_fields()).unwrap_or_default()
    }

    pub fn tuple_fields(&self) -> Vec<ast::TupleField> {
        self.field_list().map(|it| it.tuple_fields()).unwrap_or_default()
    }

    pub fn enum_(&self) -> ast::Enum {
        let variant_list = self.syntax.parent().unwrap();
        let enum_ = variant_list.parent().unwrap();
        ast::Enum::cast(enum_).unwrap()
    }
}
