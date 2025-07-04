// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast;

impl ast::MethodCallExpr {
    pub fn reference_name(&self) -> String {
        let name_ref = self.name_ref().expect("required by the parser");
        // methods always have non-integer reference name
        name_ref.as_string()
    }

    pub fn arg_exprs(&self) -> Vec<Option<ast::Expr>> {
        self.value_arg_list().map(|it| it.arg_exprs()).unwrap_or_default()
    }
}
