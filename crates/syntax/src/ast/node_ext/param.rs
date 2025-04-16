use crate::ast;
use crate::ast::NamedElement;

impl ast::Param {
    pub fn ident_name(&self) -> String {
        if self.wildcard_pat().is_some() {
            return "_".to_string();
        }
        let ident_pat = self.ident_pat().unwrap();
        ident_pat.name().unwrap().as_string()
    }
}
