use crate::ast;

impl ast::LambdaType {
    pub fn param_types(&self) -> Vec<ast::Type> {
        self.lambda_type_params().map(|it| it.type_()).collect()
    }
}
