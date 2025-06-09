use crate::ast;

impl ast::FieldList {
    pub fn named_fields(&self) -> Vec<ast::NamedField> {
        self.clone()
            .named_field_list()
            .map(|list| list.fields().collect::<Vec<_>>())
            .unwrap_or_default()
    }

    pub fn tuple_fields(&self) -> Vec<ast::TupleField> {
        self.clone()
            .tuple_field_list()
            .map(|list| list.fields().collect::<Vec<_>>())
            .unwrap_or_default()
    }
}
