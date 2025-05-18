use crate::ast::node_ext::syntax_element::SyntaxElementExt;
use crate::syntax_editor::Element;
use crate::{SyntaxElement, SyntaxNode, SyntaxToken};

impl SyntaxElementExt for SyntaxToken {
    fn to_syntax_element(&self) -> SyntaxElement {
        self.syntax_element()
    }
}
