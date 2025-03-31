use crate::{ast, AstNode};
use parser::SyntaxKind;

impl ast::AnyFieldsOwner {
    pub fn struct_or_enum(&self) -> ast::StructOrEnum {
        match self.syntax.kind() {
            SyntaxKind::STRUCT => self.cast_into::<ast::Struct>().unwrap().into(),
            SyntaxKind::VARIANT => {
                let enum_variant = self.cast_into::<ast::Variant>().unwrap();
                enum_variant.enum_().into()
            }
            _ => unreachable!(),
        }
    }
}
