use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::{ast, AstNode};

impl ast::StructPatField {
    pub fn struct_pat(&self) -> ast::StructPat {
        self.syntax()
            .parent_of_type::<ast::StructPat>()
            .expect("required by parser")
    }
}
