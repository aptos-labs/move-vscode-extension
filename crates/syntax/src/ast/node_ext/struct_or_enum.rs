use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::ast::NamedElement;
use crate::files::InFile;
use crate::{ast, AstNode};
use std::collections::HashSet;

impl ast::StructOrEnum {
    pub fn module(&self) -> ast::Module {
        self.syntax()
            .parent_of_type::<ast::Module>()
            .expect("required by the parser")
    }

    pub fn fields(&self) -> Vec<(String, ast::AnyField)> {
        let mut fields = vec![];

        fields.extend(
            self.named_fields()
                .into_iter()
                .map(|it| (it.field_name().as_string(), it.into())),
        );
        fields.extend(
            self.tuple_fields()
                .into_iter()
                .enumerate()
                .map(|(idx, it)| (idx.to_string(), it.into())),
        );
        fields
    }

    pub fn named_fields(&self) -> Vec<ast::NamedField> {
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

    pub fn tuple_fields(&self) -> Vec<ast::TupleField> {
        match self {
            ast::StructOrEnum::Struct(struct_) => struct_.tuple_fields(),
            ast::StructOrEnum::Enum(enum_) => {
                let mut visited_names = HashSet::new();
                let mut fields = vec![];
                for variant in enum_.variants() {
                    for (i, field) in variant.tuple_fields().into_iter().enumerate() {
                        let field_name = i.to_string();
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

    pub fn abilities(&self) -> Vec<ast::Ability> {
        self.ability_list()
            .map(|it| it.abilities().collect())
            .unwrap_or_default()
    }
}
