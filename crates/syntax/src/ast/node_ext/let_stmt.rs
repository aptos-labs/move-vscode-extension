use crate::ast;

impl ast::LetStmt {
    pub fn is_post(&self) -> bool {
        self.post_token().is_some()
    }
}
