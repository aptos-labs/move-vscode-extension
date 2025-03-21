use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::{ast, AstNode};

impl ast::StructOrEnum {
    pub fn module(&self) -> ast::Module {
        self.syntax()
            .parent_of_type::<ast::Module>()
            .expect("required by the parser")
    }
}
