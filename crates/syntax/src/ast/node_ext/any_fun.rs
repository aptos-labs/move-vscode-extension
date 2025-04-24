use crate::ast;

impl ast::AnyFun {
    pub fn params(&self) -> Vec<ast::Param> {
        self.param_list()
            .map(|list| list.params().collect())
            .unwrap_or_default()
    }

    pub fn params_as_bindings(&self) -> Vec<ast::IdentPat> {
        self.params()
            .into_iter()
            .filter_map(|param| param.ident_pat())
            .collect()
    }

    pub fn return_type(&self) -> Option<ast::Type> {
        self.ret_type()?.type_()
    }

    pub fn into_generic_element(self) -> ast::AnyGenericElement {
        match self {
            ast::AnyFun::Fun(it) => it.into(),
            ast::AnyFun::SpecFun(it) => it.into(),
            ast::AnyFun::SpecInlineFun(it) => it.into(),
        }
    }
}

impl ast::Fun {
    pub fn to_any_fun(&self) -> ast::AnyFun {
        ast::AnyFun::Fun(self.clone())
    }
}

impl ast::SpecFun {
    pub fn to_any_fun(&self) -> ast::AnyFun {
        ast::AnyFun::SpecFun(self.clone())
    }
}

impl ast::SpecInlineFun {
    pub fn to_any_fun(&self) -> ast::AnyFun {
        ast::AnyFun::SpecInlineFun(self.clone())
    }
}
