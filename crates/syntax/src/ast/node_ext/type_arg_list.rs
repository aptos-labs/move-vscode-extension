use crate::ast::AstNode;
use crate::{ast, match_ast};

impl ast::TypeArgList {
    pub fn method_or_path(&self) -> Option<ast::MethodOrPath> {
        let parent = self.syntax.parent()?;
        match_ast! {
            match parent {
                ast::PathSegment(it) => Some(it.parent_path().into()),
                ast::MethodCallExpr(it) => Some(it.into()),
                _ => None
            }
        }
    }
}
