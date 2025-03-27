use crate::ast;

impl ast::LambdaExpr {
    pub fn lambda_params(&self) -> Vec<ast::LambdaParam> {
        self.lambda_param_list()
            .map(|it| it.lambda_params().collect())
            .unwrap_or_default()
    }

    pub fn lambda_params_as_bindings(&self) -> Vec<ast::IdentPat> {
        self.lambda_params()
            .into_iter()
            .map(|it| it.ident_pat())
            .collect()
    }
}
