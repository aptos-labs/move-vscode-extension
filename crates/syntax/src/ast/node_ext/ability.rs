use crate::{AstNode, ast};

impl ast::Ability {
    pub fn is_key(&self) -> bool {
        self.syntax().text() == "key"
    }
}
