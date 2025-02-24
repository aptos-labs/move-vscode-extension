use crate::ast;

impl ast::Fun {
    pub fn params(&self) -> Vec<ast::Param> {
        self.param_list()
            .map(|list| list.params().collect())
            .unwrap_or_default()
    }

    pub fn params_as_bindings(&self) -> Vec<ast::IdentPat> {
        self.params()
            .into_iter()
            .map(|param| param.ident_pat().expect("always present"))
            .collect()
    }

    pub fn return_type(&self) -> Option<ast::Type> {
        self.ret_type()?.type_()
    }
}
