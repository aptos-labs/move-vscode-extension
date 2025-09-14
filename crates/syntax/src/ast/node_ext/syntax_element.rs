// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::SyntaxKind::*;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::syntax_editor::Element;
use crate::{AstNode, SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken, ast};
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
            .filter(|it| it.kind() == WHITESPACE)
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
            .filter(|it| it.kind() == WHITESPACE)
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

    fn ancestors_of_type<N: AstNode>(&self) -> impl Iterator<Item = N> {
        self.to_syntax_element().ancestors().filter_map(N::cast)
    }

    fn parent_of_type<Ast: AstNode>(&self) -> Option<Ast> {
        let parent_node = self.to_syntax_element().parent()?;
        Ast::cast(parent_node)
    }

    fn ancestor_strict<Ast: AstNode>(&self) -> Option<Ast> {
        self.to_syntax_element().ancestors().skip(1).find_map(Ast::cast)
    }

    fn has_ancestor_strict<Ast: AstNode>(&self) -> bool {
        self.to_syntax_element().ancestor_strict::<Ast>().is_some()
    }

    fn is<T: AstNode>(&self) -> bool {
        T::can_cast(self.to_syntax_element().kind())
    }

    fn parent_is<T: AstNode>(&self) -> bool {
        self.to_syntax_element().parent().is_some_and(|it| it.is::<T>())
    }

    fn is_msl_only_item(&self) -> bool {
        self.is::<ast::AnyMslOnly>()
    }

    fn is_kind(&self, kind: SyntaxKind) -> bool {
        self.to_syntax_element().kind() == kind
    }

    fn containing_module(&self) -> Option<ast::Module> {
        self.ancestor_strict::<ast::Module>()
    }

    fn containing_items_owner(&self) -> Option<ast::AnyHasItems> {
        self.ancestor_strict::<ast::AnyHasItems>()
    }

    fn containing_function(&self) -> Option<ast::Fun> {
        self.ancestor_strict::<ast::Fun>()
    }

    fn containing_script(&self) -> Option<ast::Script> {
        self.ancestor_strict::<ast::Script>()
    }

    fn containing_file(&self) -> Option<ast::SourceFile> {
        // let mut syntax_element = self.to_syntax_element();
        self.to_syntax_element().ancestor_strict::<ast::SourceFile>()
        // while syntax_element.kind() != SOURCE_FILE {
        //     syntax_element = syntax_element.parent()?.syntax_element();
        // }
        // ast::SourceFile::cast(syntax_element)
        //
        // algo::containing_file_for_node(self.to_syntax_element())
    }

    fn containing_item_spec(&self) -> Option<ast::ItemSpec> {
        self.ancestor_strict::<ast::ItemSpec>()
    }

    fn is_msl_context(&self) -> bool {
        for ancestor in self.to_syntax_element().ancestors() {
            if matches!(ancestor.kind(), MODULE | FUN | STRUCT | ENUM) {
                return false;
            }
            if ancestor.is_msl_only_item() {
                return true;
            }
        }
        false
    }

    fn loc_node(&self) -> SyntaxNode {
        match self.to_syntax_element() {
            NodeOrToken::Node(node) => node,
            NodeOrToken::Token(token) => token.parent().expect("should have at least one parent"),
        }
    }
}

impl SyntaxElementExt for SyntaxElement {
    fn to_syntax_element(&self) -> SyntaxElement {
        self.clone()
    }
}

impl SyntaxElementExt for SyntaxNode {
    fn to_syntax_element(&self) -> SyntaxElement {
        self.syntax_element()
    }
}

impl SyntaxElementExt for SyntaxToken {
    fn to_syntax_element(&self) -> SyntaxElement {
        self.syntax_element()
    }
}
