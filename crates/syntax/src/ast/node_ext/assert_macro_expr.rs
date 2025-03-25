use crate::ast;

impl ast::AssertMacroExpr {
    pub fn args(&self) -> Vec<ast::Expr> {
        self.arg_list()
            .map(|it| it.arg_exprs().collect())
            .unwrap_or_default()
    }
}