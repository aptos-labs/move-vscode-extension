use crate::ast;
use crate::ast::FieldsOwner;

impl ast::Struct {
    pub fn is_tuple_struct(&self) -> bool {
        self.tuple_field_list().is_some()
    }
}
