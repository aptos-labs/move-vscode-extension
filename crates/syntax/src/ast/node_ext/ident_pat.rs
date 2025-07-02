use crate::ast::NamedElement;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::{AstNode, ast};

impl ast::IdentPat {
    pub fn reference(&self) -> ast::ReferenceElement {
        self.clone().into()
    }

    pub fn ident_owner(&self) -> Option<ast::IdentPatOwner> {
        self.syntax().ancestor_strict::<ast::IdentPatOwner>()
    }
}
