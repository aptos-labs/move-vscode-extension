// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast;
use crate::ast::node_ext::syntax_element::SyntaxElementExt;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;

impl ast::LambdaParam {
    pub fn name_as_string(&self) -> Option<String> {
        let ident_pat = self.ident_pat()?;
        let name = ident_pat.name()?;
        Some(name.as_string())
    }

    pub fn lambda_expr(&self) -> ast::LambdaExpr {
        let param_list = self.syntax.parent().unwrap();
        param_list.parent_of_type::<ast::LambdaExpr>().unwrap()
    }
}
