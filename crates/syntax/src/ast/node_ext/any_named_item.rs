use crate::{ast, AstNode};

impl ast::AnyNamedElement {
    pub fn cast_into<T: AstNode>(self) -> Option<T> {
        T::cast(self.syntax().to_owned())
    }
}
