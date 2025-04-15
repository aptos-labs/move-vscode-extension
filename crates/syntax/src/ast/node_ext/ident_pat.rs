use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::ast::NamedElement;
use crate::{ast, AstNode};

impl ast::IdentPat {
    pub fn owner(&self) -> Option<ast::IdentPatOwner> {
        self.syntax().ancestor_strict::<ast::IdentPatOwner>()
    }

    pub fn as_string(&self) -> String {
        self.name().expect("IdentPat.Name is required").as_string()
    }
}
