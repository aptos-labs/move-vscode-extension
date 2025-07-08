// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast;

impl ast::AssertMacroExpr {
    pub fn arg_exprs(&self) -> Vec<Option<ast::Expr>> {
        self.value_arg_list().map(|it| it.arg_exprs()).unwrap_or_default()
    }

    pub fn error_expr(&self) -> Option<ast::Expr> {
        let mut arg_exprs = self.arg_exprs().into_iter();
        let _ = arg_exprs.next();
        arg_exprs.next().flatten()
    }
}
