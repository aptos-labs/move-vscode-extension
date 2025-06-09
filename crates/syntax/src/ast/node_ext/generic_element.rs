use crate::ast;

impl ast::GenericElement {
    pub fn type_params(&self) -> Vec<ast::TypeParam> {
        self.type_param_list()
            .map(|l| l.type_parameters().collect())
            .unwrap_or_default()
    }
}
