use crate::ast;

impl ast::AnyCallExpr {
    pub fn arg_exprs(&self) -> Vec<Option<ast::Expr>> {
        match self {
            ast::AnyCallExpr::CallExpr(call_expr) => call_expr.arg_exprs(),
            ast::AnyCallExpr::MethodCallExpr(call_expr) => call_expr.arg_exprs(),
            ast::AnyCallExpr::AssertMacroExpr(call_expr) => call_expr.arg_exprs(),
        }
    }
}
