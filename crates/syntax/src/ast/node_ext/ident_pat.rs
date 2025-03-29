use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::ast::NamedElement;
use crate::{ast, AstNode};

impl ast::IdentPat {
    pub fn type_owner(&self) -> Option<ast::BindingTypeOwner> {
        self.syntax().ancestor_strict::<ast::BindingTypeOwner>()
    }

    pub fn as_string(&self) -> String {
        self.name().expect("IdentPat.Name is required").as_string()
    }
}
