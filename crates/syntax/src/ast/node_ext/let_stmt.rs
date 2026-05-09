// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast::node_ext::syntax_element::SyntaxElementExt;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::{AstNode, ast};

impl ast::LetStmt {
    pub fn is_post(&self) -> bool {
        self.post_token().is_some() || self.syntax().has_ancestor_strict::<ast::PostStmt>()
    }
}
