// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast;

impl ast::ValueArgList {
    pub fn arg_exprs(&self) -> Vec<Option<ast::Expr>> {
        self.args().map(|it| it.expr()).collect()
    }
}
