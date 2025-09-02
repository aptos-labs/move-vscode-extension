use crate::ast::edit::IndentLevel;
use crate::ast::syntax_factory::SyntaxFactory;
use crate::syntax_editor::{Element, SyntaxEditor};
use crate::{AstNode, ast};

impl ast::AnyHasItems {
    pub fn add_use_stmt(&self, editor: &mut SyntaxEditor, use_stmt: &ast::UseStmt) -> Option<()> {
        let (anchor, needs_newline_at_the_end) = self.pos_after_last_use_stmt()?;

        let make = SyntaxFactory::new();
        let indent = IndentLevel::from_node(self.syntax()) + 1;
        let mut elements_to_add = vec![
            make.whitespace(&format!("\n{indent}")).syntax_element(),
            use_stmt.syntax().syntax_element(),
        ];
        if needs_newline_at_the_end {
            elements_to_add.push(make.newline().into());
        }
        editor.insert_all(anchor, elements_to_add);

        Some(())
    }
}
