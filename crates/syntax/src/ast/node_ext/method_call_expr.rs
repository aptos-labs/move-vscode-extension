use crate::ast;

impl ast::MethodCallExpr {
    pub fn reference_name(&self) -> String {
        self.name_ref().expect("required by the parser").as_string()
    }

    pub fn args(&self) -> Vec<ast::Expr> {
        self.arg_list()
            .map(|it| it.arg_exprs().collect())
            .unwrap_or_default()
    }
}
