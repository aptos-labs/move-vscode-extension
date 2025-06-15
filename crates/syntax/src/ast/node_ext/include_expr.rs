use crate::ast;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use std::io::Read;
use std::iter;

impl ast::IncludeExpr {
    pub fn schema_lits(&self) -> Vec<ast::SchemaLit> {
        match self {
            ast::IncludeExpr::SchemaIncludeExpr(schema_include_expr) => {
                schema_include_expr.schema_lit().into_iter().collect()
            }
            ast::IncludeExpr::AndIncludeExpr(and_include_expr) => and_include_expr
                .left_schema_lit()
                .into_iter()
                .chain(and_include_expr.right_schema_lit().into_iter())
                .collect(),
            ast::IncludeExpr::IfElseIncludeExpr(if_expr_include_expr) => if_expr_include_expr
                .then_schema_lit()
                .into_iter()
                .chain(if_expr_include_expr.else_schema_lit().into_iter())
                .collect(),
            ast::IncludeExpr::ImplyIncludeExpr(imply_expr) => {
                // ignore for now
                vec![]
            }
        }
    }
}
