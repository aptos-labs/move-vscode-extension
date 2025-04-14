use crate::ast;

impl ast::LambdaExpr {
    pub fn params(&self) -> Vec<ast::LambdaParam> {
        self.lambda_param_list()
            .map(|it| it.lambda_params().collect())
            .unwrap_or_default()
    }

    pub fn params_with_ident_pats(&self) -> Vec<(ast::LambdaParam, ast::IdentPat)> {
        self.params()
            .into_iter()
            .filter_map(|it| {
                let ident_pat = it.ident_pat()?;
                Some((it, ident_pat))
            })
            .collect()
    }

    pub fn param_ident_pats(&self) -> Vec<ast::IdentPat> {
        self.params_with_ident_pats()
            .into_iter()
            .map(|(_, ident_pat)| ident_pat)
            .collect()
        // self.lambda_params()
        //     .into_iter()
        //     .filter_map(|it| it.ident_pat())
        //     .collect()
    }
}
