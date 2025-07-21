// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;

impl ast::TypeParam {
    pub fn generic_element(&self) -> Option<ast::GenericElement> {
        let type_param_list = self.syntax.parent_of_type::<ast::TypeParamList>()?;
        type_param_list.syntax.parent_of_type()
    }

    pub fn ability_bounds(&self) -> Vec<ast::Ability> {
        self.ability_bound_list()
            .map(|it| it.abilities().collect())
            .unwrap_or_default()
    }
}
