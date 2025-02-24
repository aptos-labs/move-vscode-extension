use crate::ast::Type;
use crate::{ast, AstNode};

impl ast::Type {
    pub fn text(&self) -> String {
        match self {
            Type::PathType(t) => t.syntax(),
            Type::RefType(t) => t.syntax(),
        }
        .to_string()
    }
}
