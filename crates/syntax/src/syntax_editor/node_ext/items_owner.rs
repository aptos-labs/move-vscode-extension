use crate::ast::syntax_factory::SyntaxFactory;
use crate::syntax_editor::{Element, SyntaxEditor};
use crate::{AstNode, ast};

impl ast::AnyHasItems {
    pub fn add_use_stmt(&self, use_stmt: ast::UseStmt, editor: &mut SyntaxEditor) -> Option<()> {
        let (anchor, needs_newline_at_the_end) = self.pos_after_last_use_stmt()?;

        let make = SyntaxFactory::new();
        let mut elements_to_add = vec![
            make.newline().into(),
            make.whitespace("    ").into(),
            use_stmt.syntax().syntax_element(),
        ];
        if needs_newline_at_the_end {
            elements_to_add.push(make.newline().into());
        }
        editor.insert_all(anchor, elements_to_add);

        Some(())
    }
}
