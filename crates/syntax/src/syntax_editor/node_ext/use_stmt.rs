use crate::ast::node_ext::syntax_element::SyntaxElementExt;
use crate::syntax_editor::SyntaxEditor;
use crate::{AstNode, ast};

impl ast::UseStmt {
    pub fn delete(&self, editor: &mut SyntaxEditor) {
        if let Some(following_ws) = self.syntax().following_ws() {
            editor.delete(following_ws);
        }
        editor.delete(self.syntax());
    }
}
