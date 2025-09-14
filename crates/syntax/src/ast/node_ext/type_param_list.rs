// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::ast;
use crate::ast::node_ext::syntax_element::SyntaxElementExt;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;

impl ast::TypeParamList {
    pub fn generic_element(&self) -> Option<ast::GenericElement> {
        self.syntax.parent_of_type()
    }
}
