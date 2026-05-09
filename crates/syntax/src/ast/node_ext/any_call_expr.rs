// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::ast;

impl ast::AnyCallExpr {
    #[inline]
    pub fn arg_exprs(&self) -> Vec<Option<ast::Expr>> {
        self.value_arg_list().map(|it| it.arg_exprs()).unwrap_or_default()
    }

    // pub fn arg_exprs(&self) -> Vec<Option<ast::Expr>> {
    //     match self {
    //         ast::AnyCallExpr::CallExpr(call_expr) => call_expr.arg_exprs(),
    //         ast::AnyCallExpr::MethodCallExpr(call_expr) => call_expr.arg_exprs(),
    //         ast::AnyCallExpr::AssertMacroExpr(call_expr) => call_expr.arg_exprs(),
    //     }
    // }

    #[inline]
    pub fn n_provided_args(&self) -> usize {
        self.arg_exprs().len()
        // match self {
        //     ast::AnyCallExpr::CallExpr(call_expr) => call_expr.arg_exprs().len(),
        //     ast::AnyCallExpr::AssertMacroExpr(call_expr) => call_expr.arg_exprs().len(),
        //     ast::AnyCallExpr::MethodCallExpr(call_expr) => call_expr.arg_exprs().len() + 1,
        // }
    }
}
