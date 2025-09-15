// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::SyntaxKind::*;
use crate::ast::node_ext::syntax_element::SyntaxElementExt;
use crate::syntax_editor::Element;
use crate::{AstNode, AstToken, SyntaxElement, SyntaxNode, SyntaxToken, TextSize, ast};
use rowan::{Direction, TokenAtOffset};

pub trait SyntaxNodeExt {
    fn syntax_node(&self) -> &SyntaxNode;

    fn token_at_offset_exact(&self, offset: TextSize) -> Option<SyntaxToken> {
        let token_at_offset = self.syntax_node().token_at_offset(offset);
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
        let self_node = self.syntax_node();
        node.ancestors().any(|it| &it == self_node)
    }

    fn ancestor_or_self<Ast: AstNode>(&self) -> Option<Ast> {
        self.syntax_node().ancestors().find_map(Ast::cast)
    }

    fn has_ancestor_or_self<Ast: AstNode>(&self) -> bool {
        self.ancestor_or_self::<Ast>().is_some()
    }

    fn descendants_of_type<Ast: AstNode>(&self) -> impl Iterator<Item = Ast> {
        self.syntax_node().descendants().filter_map(Ast::cast)
    }

    fn is_msl_only_scope(&self) -> bool {
        matches!(
            self.syntax_node().kind(),
            SPEC_FUN | SPEC_INLINE_FUN | ITEM_SPEC | SPEC_BLOCK_EXPR | SCHEMA
        )
    }

    fn cast<T: AstNode>(&self) -> Option<T> {
        let node = self.syntax_node();
        if T::can_cast(node.kind()) {
            T::cast(node.clone())
        } else {
            None
        }
    }

    fn inference_ctx_owner(&self) -> Option<ast::InferenceCtxOwner> {
        self.ancestor_or_self::<ast::InferenceCtxOwner>()
    }

    fn next_siblings_with_tokens(&self) -> impl Iterator<Item = SyntaxElement> {
        self.syntax_node().siblings_with_tokens(Direction::Next).skip(1)
    }

    fn prev_siblings_with_tokens(&self) -> impl Iterator<Item = SyntaxElement> {
        self.syntax_node().siblings_with_tokens(Direction::Prev).skip(1)
    }
}

impl SyntaxNodeExt for SyntaxNode {
    fn syntax_node(&self) -> &SyntaxNode {
        self
    }
}

// pub trait SyntaxTokenExt {
//     fn is_kind(&self, kind: SyntaxKind) -> bool;
// }
//
// impl SyntaxTokenExt for SyntaxToken {
//     fn is_kind(&self, kind: SyntaxKind) -> bool {
//         self.kind() == kind
//     }
// }
