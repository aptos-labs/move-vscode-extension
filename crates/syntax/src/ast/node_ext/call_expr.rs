use crate::ast;

impl ast::CallExpr {
    pub fn path(&self) -> Option<ast::Path> {
        let path = self.expr()?.path_expr()?.path();
        Some(path)
    }

    pub fn arg_exprs(&self) -> Vec<Option<ast::Expr>> {
        self.value_arg_list().map(|it| it.arg_exprs()).unwrap_or_default()
    }
}
