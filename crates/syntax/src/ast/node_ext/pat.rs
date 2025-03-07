use crate::{ast, AstNode};
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;

impl ast::Pat {
    pub fn bindings(&self) -> Vec<ast::IdentPat> {
        self.syntax().descendants_of_type::<ast::IdentPat>().collect()
    }
}