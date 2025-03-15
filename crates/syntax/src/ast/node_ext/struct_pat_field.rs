use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::{ast, AstNode};

impl ast::StructPatField {
    pub fn struct_pat(&self) -> ast::StructPat {
        self.syntax()
            .ancestor_of_type::<ast::StructPat>(true)
            .expect("required by parser")
    }
}

impl ast::StructLitField {
    pub fn struct_lit(&self) -> ast::StructLit {
        self.syntax()
            .ancestor_of_type::<ast::StructLit>(true)
            .expect("required by parser")
    }
}
