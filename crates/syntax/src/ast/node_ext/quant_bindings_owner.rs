use crate::ast;

impl ast::QuantBindingsOwner {
    pub fn quant_bindings(&self) -> Vec<ast::QuantBinding> {
        self.quant_binding_list()
            .map(|it| it.bindings().collect())
            .unwrap_or_default()
    }

    pub fn quant_bindings_as_ident_pats(&self) -> Vec<ast::IdentPat> {
        self.quant_bindings()
            .into_iter()
            .filter_map(|it| it.ident_pat())
            .collect()
    }
}
