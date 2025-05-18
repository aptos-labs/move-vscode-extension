use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::{SyntaxElement, SyntaxToken};
use itertools::Itertools;
use rowan::NodeOrToken;

pub trait SyntaxElementExt {
    fn to_syntax_element(&self) -> SyntaxElement;

    fn prev_sibling_or_token_no_trivia(&self) -> Option<SyntaxElement> {
        let prev = self.to_syntax_element().prev_sibling_or_token();
        if let Some(prev) = prev {
            if prev.kind().is_trivia() {
                return prev.prev_sibling_or_token();
            }
        }
        None
    }

    fn next_sibling_or_token_no_trivia(&self) -> Option<SyntaxElement> {
        let next = self.to_syntax_element().next_sibling_or_token();
        if let Some(next) = next {
            if next.kind().is_trivia() {
                return next.next_sibling_or_token();
            }
        }
        None
    }

    /// walks up over the tree if needed
    fn next_token(&self) -> Option<SyntaxToken> {
        let syntax_element = self.to_syntax_element();
        let sibling_or_token = match syntax_element.next_sibling_or_token() {
            Some(it) => it,
            None => {
                return syntax_element.parent()?.next_token();
            }
        };
        match sibling_or_token {
            NodeOrToken::Token(token) => Some(token),
            NodeOrToken::Node(node) => node.first_token(),
        }
    }

    fn next_token_no_trivia(&self) -> Option<SyntaxToken> {
        let next_token = self.next_token();
        if let Some(next_token) = next_token {
            if next_token.kind().is_trivia() {
                return next_token.next_token();
            }
        }
        None
    }
}

impl SyntaxElementExt for SyntaxElement {
    fn to_syntax_element(&self) -> SyntaxElement {
        self.clone()
    }
}
