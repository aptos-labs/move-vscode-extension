use crate::ast;

impl ast::CallExpr {
    pub fn path(&self) -> Option<ast::Path> {
        let path = self.expr()?.path_expr()?.path();
        Some(path)
    }

    pub fn args(&self) -> Vec<ast::Expr> {
        self.arg_list()
            .map(|it| it.arg_exprs().collect())
            .unwrap_or_default()
    }
}
