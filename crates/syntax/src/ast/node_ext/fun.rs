use crate::ast::NamedItem;
use crate::{ast, AstNode};

impl ast::Fun {
    pub fn params(&self) -> Vec<ast::Param> {
        self.param_list()
            .map(|list| list.params().collect())
            .unwrap_or_default()
    }

    pub fn params_as_bindings(&self) -> Vec<ast::IdentPat> {
        self.params().into_iter().map(|param| param.ident_pat()).collect()
    }

    pub fn return_type(&self) -> Option<ast::Type> {
        self.ret_type()?.type_()
    }

    pub fn is_native(&self) -> bool {
        self.native_token().is_some()
    }
    pub fn is_entry(&self) -> bool {
        self.entry_token().is_some()
    }
    pub fn is_inline(&self) -> bool {
        self.inline_token().is_some()
    }

    pub fn self_param(&self) -> Option<ast::Param> {
        self.params()
            .first()
            .map(|it| it.to_owned())
            .take_if(|param| param.ident_pat().name().is_some_and(|name| name.text() == "self"))
    }
}
