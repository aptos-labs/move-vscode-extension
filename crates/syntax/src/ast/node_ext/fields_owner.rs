use crate::ast::support;
use crate::parse::SyntaxKind;
use crate::{ast, AstNode};
use std::collections::HashMap;

impl ast::FieldsOwner {
    #[inline]
    pub fn named_field_list(&self) -> Option<ast::NamedFieldList> {
        self.field_list().and_then(|it| it.named_field_list())
    }

    #[inline]
    pub fn tuple_field_list(&self) -> Option<ast::TupleFieldList> {
        self.field_list().and_then(|it| it.tuple_field_list())
    }

    pub fn named_fields(&self) -> Vec<ast::NamedField> {
        self.named_field_list()
            .map(|list| list.fields().collect::<Vec<_>>())
            .unwrap_or_default()
    }

    pub fn named_fields_map(&self) -> HashMap<String, ast::NamedField> {
        self.named_fields()
            .into_iter()
            .map(|field| (field.field_name().as_string(), field))
            .collect()
    }

    pub fn tuple_fields(&self) -> Vec<ast::TupleField> {
        self.tuple_field_list()
            .map(|list| list.fields().collect::<Vec<_>>())
            .unwrap_or_default()
    }

    pub fn named_and_tuple_fields(&self) -> Vec<ast::AnyField> {
        self.named_fields()
            .into_iter()
            .map(|f| f.into())
            .chain(self.tuple_fields().into_iter().map(|f| f.into()))
            .collect()
    }

    pub fn is_fieldless(&self) -> bool {
        self.named_field_list().is_none() && self.tuple_field_list().is_none()
    }

    pub fn struct_or_enum(&self) -> ast::StructOrEnum {
        match self {
            ast::FieldsOwner::Struct(struct_) => struct_.clone().into(),
            ast::FieldsOwner::Variant(variant) => variant.enum_().into(),
        }
    }
}
