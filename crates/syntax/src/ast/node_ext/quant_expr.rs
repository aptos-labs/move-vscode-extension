use crate::ast;

impl ast::QuantExpr {
    pub fn quant_bindings(&self) -> Vec<ast::QuantBinding> {
        self.quant_binding_list()
            .map(|it| it.bindings().collect())
            .unwrap_or_default()
    }
}
