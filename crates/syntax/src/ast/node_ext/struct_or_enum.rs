use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::ast::{FieldsOwner, NamedElement};
use crate::files::InFile;
use crate::{ast, AstNode};
use std::collections::HashSet;

impl ast::StructOrEnum {
    pub fn module(&self) -> ast::Module {
        self.syntax()
            .parent_of_type::<ast::Module>()
            .expect("required by the parser")
    }

    pub fn field_ref_lookup_fields(&self) -> Vec<ast::NamedField> {
        match self {
            ast::StructOrEnum::Struct(struct_) => struct_.named_fields(),
            ast::StructOrEnum::Enum(enum_) => {
                let mut visited_names = HashSet::new();
                let mut fields = vec![];
                for variant in enum_.variants() {
                    for field in variant.named_fields() {
                        let field_name = field.name().expect("always present").as_string();

                        if visited_names.contains(&field_name) {
                            continue;
                        }
                        visited_names.insert(field_name);

                        fields.push(field);
                    }
                }
                fields
            }
        }
    }
}

impl From<ast::StructOrEnum> for ast::Item {
    fn from(value: ast::StructOrEnum) -> Self {
        match value {
            ast::StructOrEnum::Struct(it) => it.into(),
            ast::StructOrEnum::Enum(it) => it.into(),
        }
    }
}
