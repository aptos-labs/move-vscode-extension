// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::SyntaxKind::{COMMA, ERROR};
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::{SyntaxElement, SyntaxKind, SyntaxToken};
use itertools::Itertools;
use rowan::NodeOrToken;

pub trait SyntaxElementExt {
    fn to_syntax_element(&self) -> SyntaxElement;

    fn prev_sibling_or_token_no_trivia(&self) -> Option<SyntaxElement> {
        let prev = self.to_syntax_element().prev_sibling_or_token();
        if let Some(prev) = &prev
            && prev.kind().is_trivia()
        {
            return prev.prev_sibling_or_token_no_trivia();
        }
        prev
    }

    fn next_sibling_or_token_no_trivia(&self) -> Option<SyntaxElement> {
        let next = self.to_syntax_element().next_sibling_or_token();
        if let Some(next) = &next
            && next.kind().is_trivia()
        {
            return next.next_sibling_or_token_no_trivia();
        }
        next
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

    fn following_comma(&self) -> Option<SyntaxToken> {
        self.to_syntax_element()
            .next_sibling_or_token_no_trivia()
            .and_then(|it| it.into_token())
            .filter(|it| it.kind() == COMMA)
    }

    fn following_ws(&self) -> Option<SyntaxToken> {
        self.to_syntax_element()
            .next_sibling_or_token()
            .and_then(|it| it.into_token())
            .filter(|it| it.kind() == SyntaxKind::WHITESPACE)
    }

    fn preceding_comma(&self) -> Option<SyntaxToken> {
        self.to_syntax_element()
            .prev_sibling_or_token_no_trivia()
            .and_then(|it| it.into_token())
            .filter(|it| it.kind() == COMMA)
    }

    fn preceding_ws(&self) -> Option<SyntaxToken> {
        self.to_syntax_element()
            .prev_sibling_or_token()
            .and_then(|it| it.into_token())
            .filter(|it| it.kind() == SyntaxKind::WHITESPACE)
    }

    fn error_node_or_self(&self) -> SyntaxElement {
        let mut element = self.to_syntax_element();
        if let Some(parent) = element.parent()
            && parent.kind().is_error()
        {
            parent.into()
        } else {
            element
        }
    }
}

impl SyntaxElementExt for SyntaxElement {
    fn to_syntax_element(&self) -> SyntaxElement {
        self.clone()
    }
}
