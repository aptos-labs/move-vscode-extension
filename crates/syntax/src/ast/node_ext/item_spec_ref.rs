// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast;
use crate::ast::node_ext::syntax_element::SyntaxElementExt;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;

impl ast::ItemSpecRef {
    pub fn reference(&self) -> ast::ReferenceElement {
        self.clone().into()
    }

    pub fn item_spec(&self) -> ast::ItemSpec {
        self.syntax
            .parent_of_type::<ast::ItemSpec>()
            .expect("unreachable")
    }
}
