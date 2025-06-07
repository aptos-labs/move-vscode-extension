use crate::ast;
use crate::ast::node_ext::move_syntax_node::MoveSyntaxElementExt;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;

impl ast::LambdaParam {
    pub fn lambda_expr(&self) -> ast::LambdaExpr {
        let param_list = self.syntax.parent().unwrap();
        param_list.parent_of_type::<ast::LambdaExpr>().unwrap()
    }
}
