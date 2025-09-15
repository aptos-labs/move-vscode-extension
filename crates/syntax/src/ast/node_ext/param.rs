// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast;
use crate::ast::NamedElement;
use crate::ast::node_ext::syntax_element::SyntaxElementExt;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;

impl ast::Param {
    pub fn param_list(&self) -> Option<ast::ParamList> {
        self.syntax.parent_of_type::<ast::ParamList>()
    }

    pub fn any_fun(&self) -> Option<ast::AnyFun> {
        self.param_list()?.syntax.parent_of_type::<ast::AnyFun>()
    }

    pub fn ident_name(&self) -> String {
        if self.wildcard_pat().is_some() {
            return "_".to_string();
        }
        // todo: ident_pat can be none
        let ident_pat = self.ident_pat().unwrap();
        ident_pat.name().unwrap().as_string()
    }

    pub fn is_self(&self) -> bool {
        if self.ident_name() != "self" {
            return false;
        }
        if let Some(param_list) = self.param_list() {
            if param_list
                .params()
                .position(|it| &it == self)
                .is_some_and(|pos| pos == 0)
            {
                return true;
            }
        }
        false
    }
}
