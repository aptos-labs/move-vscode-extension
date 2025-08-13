// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::ast;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;

impl ast::UseSpeck {
    pub fn path_name(&self) -> Option<String> {
        self.path()?.reference_name()
    }

    pub fn parent_use_group(&self) -> Option<ast::UseGroup> {
        self.syntax.parent_of_type()
    }
}
