use crate::ast;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;

impl ast::Const {
    pub fn module(&self) -> Option<ast::Module> {
        self.syntax.parent_of_type::<ast::Module>()
    }
}
