// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::ast;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;

impl ast::Acquires {
    pub fn fun(&self) -> ast::Fun {
        self.syntax.parent_of_type::<ast::Fun>().unwrap()
    }
}
