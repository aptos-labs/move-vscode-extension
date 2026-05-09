use crate::ast;

impl ast::Lemma {
    pub fn params_as_bindings(&self) -> Vec<ast::IdentPat> {
        self.param_list()
            .map(|list| list.params().collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|param| param.ident_pat())
            .collect()
    }
}
