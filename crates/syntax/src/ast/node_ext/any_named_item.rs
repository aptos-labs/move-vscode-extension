use crate::{ast, AstNode};

impl ast::AnyNamedItem {
    pub fn cast<T: AstNode>(self) -> Option<T> {
        T::cast(self.syntax().to_owned())
    }
}
