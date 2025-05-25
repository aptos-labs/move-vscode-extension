use crate::ast;
use crate::ast::TypeParam;

impl ast::InvariantStmt {
    pub fn type_params(&self) -> Vec<TypeParam> {
        self.spec_type_param_list()
            .map(|it| it.type_parameters().collect())
            .unwrap_or_default()
    }
}
