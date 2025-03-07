use crate::{ast, AstNode};

impl ast::Type {
    pub fn text(&self) -> String {
        match self {
            ast::Type::PathType(t) => t.syntax(),
            ast::Type::RefType(t) => t.syntax(),
        }
        .to_string()
    }
}
