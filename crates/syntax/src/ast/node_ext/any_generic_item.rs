use crate::ast;

impl From<ast::StructOrEnum> for ast::AnyGenericElement {
    fn from(value: ast::StructOrEnum) -> Self {
        match value {
            ast::StructOrEnum::Struct(struct_) => struct_.into(),
            ast::StructOrEnum::Enum(enum_) => enum_.into(),
        }
    }
}
