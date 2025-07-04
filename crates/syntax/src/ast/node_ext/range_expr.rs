// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast::support;
use crate::{AstNode, ast};

impl ast::RangeExpr {
    pub fn start_expr(&self) -> Option<ast::Expr> {
        support::children(self.syntax()).nth(0)
    }

    pub fn end_expr(&self) -> Option<ast::Expr> {
        support::children(self.syntax()).nth(1)
    }
}
