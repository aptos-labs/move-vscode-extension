// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::ast;

impl ast::AnyCallExpr {
    #[inline]
    pub fn arg_exprs(&self) -> Vec<Option<ast::Expr>> {
        self.value_arg_list().map(|it| it.arg_exprs()).unwrap_or_default()
    }

    #[inline]
    pub fn n_provided_args(&self) -> usize {
        match self {
            ast::AnyCallExpr::MethodCallExpr(call_expr) => call_expr.arg_exprs().len() + 1,
            _ => self.arg_exprs().len(),
        }
    }
}
