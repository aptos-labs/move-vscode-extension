// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::{AstNode, ast};

impl ast::Pat {
    pub fn bindings(&self) -> Vec<ast::IdentPat> {
        self.syntax().descendants_of_type::<ast::IdentPat>().collect()
    }
}
