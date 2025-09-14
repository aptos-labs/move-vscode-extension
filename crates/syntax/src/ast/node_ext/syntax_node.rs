// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast::node_ext::syntax_element::SyntaxElementExt;
use crate::parse::SyntaxKind;
use crate::syntax_editor::Element;
use crate::{AstNode, AstToken, SyntaxElement, SyntaxNode, SyntaxToken, TextRange, TextSize, ast};
use rowan::TokenAtOffset;
use std::cmp::Ordering;

impl SyntaxElementExt for SyntaxNode {
    fn to_syntax_element(&self) -> SyntaxElement {
        self.syntax_element()
    }
}

pub trait SyntaxNodeExt {
    fn token_at_offset_exact(&self, offset: TextSize) -> Option<SyntaxToken>;
    fn ident_at_offset(&self, offset: TextSize) -> Option<ast::Ident>;

    fn is_ancestor_of(&self, node: &SyntaxNode) -> bool;

    fn ancestors_of_type<N: AstNode>(&self) -> impl Iterator<Item = N>;

    fn ancestor_or_self<Ast: AstNode>(&self) -> Option<Ast>;
    fn ancestor_strict<Ast: AstNode>(&self) -> Option<Ast>;

    fn has_ancestor_strict<Ast: AstNode>(&self) -> bool;
    fn has_ancestor_or_self<Ast: AstNode>(&self) -> bool;

    fn parent_of_type<Ast: AstNode>(&self) -> Option<Ast>;

    fn descendants_of_type<Ast: AstNode>(&self) -> impl Iterator<Item = Ast>;
}

impl SyntaxNodeExt for SyntaxNode {
    fn token_at_offset_exact(&self, offset: TextSize) -> Option<SyntaxToken> {
        let token_at_offset = self.token_at_offset(offset);
        match token_at_offset {
            TokenAtOffset::None => None,
            TokenAtOffset::Single(token) => Some(token),
            TokenAtOffset::Between(_, _) => None,
        }
    }

    fn ident_at_offset(&self, offset: TextSize) -> Option<ast::Ident> {
        let token = self.token_at_offset_exact(offset)?;
        ast::Ident::cast(token)
    }

    fn is_ancestor_of(&self, node: &SyntaxNode) -> bool {
        node.ancestors().any(|it| &it == self)
    }

    fn ancestors_of_type<N: AstNode>(&self) -> impl Iterator<Item = N> {
        self.ancestors().filter_map(N::cast)
    }

    fn ancestor_or_self<Ast: AstNode>(&self) -> Option<Ast> {
        self.ancestors().find_map(Ast::cast)
    }

    fn ancestor_strict<Ast: AstNode>(&self) -> Option<Ast> {
        self.ancestors().skip(1).find_map(Ast::cast)
    }

    fn has_ancestor_strict<Ast: AstNode>(&self) -> bool {
        self.ancestor_strict::<Ast>().is_some()
    }

    fn has_ancestor_or_self<Ast: AstNode>(&self) -> bool {
        self.ancestor_or_self::<Ast>().is_some()
    }

    fn parent_of_type<Ast: AstNode>(&self) -> Option<Ast> {
        let parent_node = self.parent()?;
        Ast::cast(parent_node)
    }

    fn descendants_of_type<Ast: AstNode>(&self) -> impl Iterator<Item = Ast> {
        self.descendants().filter_map(Ast::cast)
    }
}

pub trait SyntaxTokenExt {
    fn is(&self, kind: SyntaxKind) -> bool;
}

impl SyntaxTokenExt for SyntaxToken {
    fn is(&self, kind: SyntaxKind) -> bool {
        self.kind() == kind
    }
}
