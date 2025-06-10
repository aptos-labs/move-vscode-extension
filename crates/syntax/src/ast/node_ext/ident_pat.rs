use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::ast::NamedElement;
use crate::{ast, AstNode};

impl ast::IdentPat {
    pub fn reference(&self) -> ast::ReferenceElement {
        self.clone().into()
    }

    pub fn owner(&self) -> Option<ast::IdentPatKind> {
        self.syntax().ancestor_strict::<ast::IdentPatKind>()
    }
}
