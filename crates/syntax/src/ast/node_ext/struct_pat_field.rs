use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::{ast, AstNode};

impl ast::StructPatField {
    pub fn struct_pat(&self) -> ast::StructPat {
        self.syntax()
            .ancestor_of_type::<ast::StructPat>(true)
            .expect("required by parser")
    }
}
