use crate::ast;

impl ast::RefType {
    pub fn is_mut(&self) -> bool {
        self.mut_token().is_some()
    }
}
