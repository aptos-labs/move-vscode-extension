use crate::ast;

impl ast::Stmt {
    pub fn post_let_stmt(&self) -> Option<ast::LetStmt> {
        let post_stmt = self.clone().post_stmt()?;
        post_stmt.stmt()?.let_stmt()
    }
}
