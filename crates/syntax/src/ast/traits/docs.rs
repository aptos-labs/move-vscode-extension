// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::{AstNode, AstToken, ast};
use std::cmp::Ordering;

pub trait HoverDocsOwner: AstNode {
    fn doc_comments(&self) -> Vec<ast::Comment> {
        self.syntax()
            .children_with_tokens()
            .into_iter()
            .filter_map(|it| {
                it.into_token()
                    .and_then(ast::Comment::cast)
                    .filter(|it| it.is_doc() && it.is_outer())
            })
            .collect()
    }

    fn outer_doc_comments(&self, anchor_token: ast::SyntaxToken) -> Vec<ast::Comment> {
        self.doc_comments()
            .into_iter()
            .filter(|it| it.syntax.text_range().ordering(anchor_token.text_range()) == Ordering::Less)
            .collect()
    }
}
