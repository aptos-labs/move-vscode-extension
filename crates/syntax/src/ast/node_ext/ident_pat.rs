use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::{ast, AstNode};

impl ast::IdentPat {
    pub fn type_owner(&self) -> Option<ast::BindingTypeOwner> {
        self.syntax().ancestor_strict::<ast::BindingTypeOwner>()
    }
}
