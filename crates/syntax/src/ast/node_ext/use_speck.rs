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

    pub fn use_stmt(&self) -> Option<ast::UseStmt> {
        self.syntax.parent_of_type()
    }

    pub fn is_group_self(&self) -> bool {
        self.parent_use_group().is_some() && self.is_self_name()
    }

    pub fn is_root_self(&self) -> bool {
        self.parent_use_group().is_none()
            && self.path().is_some_and(|it| it.qualifier().is_some())
            && self.is_self_name()
    }

    fn is_self_name(&self) -> bool {
        self.path_name().is_some_and(|it| it == "Self")
    }
}
