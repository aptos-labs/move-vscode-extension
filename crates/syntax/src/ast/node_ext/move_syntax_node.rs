// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::SyntaxKind::*;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::{AstNode, SyntaxElement, SyntaxNode, algo, ast};
use rowan::Direction;

pub trait MoveSyntaxElementExt {
    fn node(&self) -> &SyntaxNode;

    fn containing_module(&self) -> Option<ast::Module> {
        self.node().ancestor_strict::<ast::Module>()
    }

    fn containing_items_owner(&self) -> Option<ast::AnyHasItems> {
        self.node().ancestor_strict::<ast::AnyHasItems>()
    }

    fn containing_function(&self) -> Option<ast::Fun> {
        self.node().ancestor_strict::<ast::Fun>()
    }

    fn containing_script(&self) -> Option<ast::Script> {
        self.node().ancestor_strict::<ast::Script>()
    }

    fn containing_file(&self) -> Option<ast::SourceFile> {
        algo::containing_file_for_node(self.node().clone())
    }

    fn containing_item_spec(&self) -> Option<ast::ItemSpec> {
        self.node().ancestor_strict::<ast::ItemSpec>()
    }

    fn is<T: AstNode>(&self) -> bool {
        T::can_cast(self.node().kind())
    }

    fn parent_is<T: AstNode>(&self) -> bool {
        self.node().parent().is_some_and(|it| it.is::<T>())
    }

    fn is_msl_only_item(&self) -> bool {
        self.is::<ast::AnyMslOnly>()
    }

    fn is_msl_only_scope(&self) -> bool {
        matches!(
            self.node().kind(),
            SPEC_FUN | SPEC_INLINE_FUN | ITEM_SPEC | SPEC_BLOCK_EXPR | SCHEMA
        )
    }

    fn is_msl_context(&self) -> bool {
        for ancestor in self.node().ancestors() {
            if matches!(ancestor.kind(), MODULE | FUN | STRUCT | ENUM) {
                return false;
            }
            if ancestor.is_msl_only_item() {
                return true;
            }
        }
        false
    }

    fn cast<T: AstNode>(&self) -> Option<T> {
        let node = self.node();
        if T::can_cast(node.kind()) {
            T::cast(node.clone())
        } else {
            None
        }
    }

    fn inference_ctx_owner(&self) -> Option<ast::InferenceCtxOwner> {
        self.node().ancestor_or_self::<ast::InferenceCtxOwner>()
    }

    fn next_siblings_with_tokens(&self) -> impl Iterator<Item = SyntaxElement> {
        self.node().siblings_with_tokens(Direction::Next).skip(1)
    }

    fn prev_siblings_with_tokens(&self) -> impl Iterator<Item = SyntaxElement> {
        self.node().siblings_with_tokens(Direction::Prev).skip(1)
    }
}

impl MoveSyntaxElementExt for SyntaxNode {
    fn node(&self) -> &SyntaxNode {
        self
    }
}
