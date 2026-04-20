use crate::ast;

impl ast::DotExpr {
    pub fn reference(&self) -> ast::ReferenceElement {
        self.clone().into()
    }
}
