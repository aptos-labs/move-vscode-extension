use crate::ast;
use crate::ast::FieldsOwner;

impl ast::Struct {
    pub fn is_tuple_struct(&self) -> bool {
        self.tuple_field_list().is_some()
    }

    pub fn wrapped_lambda_type(&self) -> Option<ast::LambdaType> {
        let mut tuple_fields = self.tuple_fields();
        match tuple_fields.len() {
            1 => {
                let tuple_field = tuple_fields.pop().unwrap();
                let lambda_type = tuple_field.type_().and_then(|it| it.lambda_type());
                lambda_type
            }
            _ => None,
        }
    }
}
