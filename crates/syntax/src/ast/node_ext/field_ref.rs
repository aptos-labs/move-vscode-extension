use crate::ast;
use crate::ast::node_ext::move_syntax_node::MoveSyntaxNodeExt;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;

impl ast::FieldRef {
    pub fn dot_expr(&self) -> ast::DotExpr {
        self.syntax.parent_of_type::<ast::DotExpr>().expect("required")
    }

    pub fn containing_module(&self) -> Option<ast::Module> {
        self.syntax.containing_module()
    }
}
