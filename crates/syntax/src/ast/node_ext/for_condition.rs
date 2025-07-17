use crate::ast;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;

impl ast::ForCondition {
    pub fn for_expr(&self) -> ast::ForExpr {
        self.syntax.parent_of_type::<ast::ForExpr>().unwrap()
    }
}
