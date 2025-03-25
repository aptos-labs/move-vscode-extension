use crate::ast;

impl ast::BorrowExpr {
    pub fn is_mut(&self) -> bool {
        self.mut_token().is_some()
    }
}