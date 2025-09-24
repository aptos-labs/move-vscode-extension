use crate::SyntaxElement;
use crate::ast::edit::AstNodeEdit;
use crate::ast::syntax_factory::SyntaxFactory;
use crate::syntax_editor::{Element, Position, SyntaxEditor};

impl SyntaxEditor {
    pub fn insert_at_next_line_after(&mut self, relative_to: &impl AstNodeEdit, element: impl Element) {
        let make = SyntaxFactory::new();
        let indent_level = relative_to.indent_level();
        self.insert_all(
            Position::after(relative_to.syntax()),
            vec![
                make.newline().syntax_element(),
                make.whitespace(&indent_level.to_string()).syntax_element(),
                element.syntax_element(),
            ],
        );
        self.add_mappings(make.finish_with_mappings());
    }
}
