use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::{ast, AstNode};

impl ast::Pat {
    pub fn bindings(&self) -> Vec<ast::IdentPat> {
        self.syntax().descendants_of_type::<ast::IdentPat>().collect()
    }
}
