use crate::ast;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;

impl ast::NamedField {
    pub fn fields_owner(&self) -> ast::AnyFieldsOwner {
        let named_field_list = self
            .syntax
            .parent_of_type::<ast::NamedFieldList>()
            .expect("`NamedField.named_field_list` is required");
        let fields_owner = named_field_list
            .syntax
            .parent_of_type::<ast::AnyFieldsOwner>()
            .expect("NamedFieldList.fields_owner is required");
        fields_owner
    }
}
