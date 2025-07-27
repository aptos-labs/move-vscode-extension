// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::ast;

impl ast::AbortExpr {
    pub fn error_expr(&self) -> Option<ast::Expr> {
        self.expr()
    }
}
