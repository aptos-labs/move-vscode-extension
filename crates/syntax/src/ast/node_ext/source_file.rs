// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::syntax_node::Aptos;
use crate::{AstNode, SyntaxToken, TextSize, algo, ast};
use rowan::TokenAtOffset;

impl ast::SourceFile {
    pub fn all_modules(&self) -> impl Iterator<Item = ast::Module> {
        self.modules()
            .chain(self.address_defs().flat_map(|ad| ad.modules()))
    }

    pub fn find_token_at_offset(&self, offset: TextSize) -> TokenAtOffset<SyntaxToken> {
        self.syntax.token_at_offset(offset)
    }

    pub fn find_node_at_offset<N: AstNode>(&self, offset: TextSize) -> Option<N> {
        algo::find_node_at_offset(self.syntax(), offset)
    }

    pub fn find_original_token(&self, fake_token: SyntaxToken) -> Option<SyntaxToken> {
        self.syntax
            .token_at_offset(fake_token.text_range().start())
            .right_biased()
    }

    pub fn find_original_node<N: AstNode>(&self, fake_node: N) -> Option<N> {
        algo::find_node_at_offset(self.syntax(), fake_node.syntax().text_range().start())
    }
}
