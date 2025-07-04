// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;

impl ast::PathExpr {
    // for liveness analysis
    pub fn is_reassignment(&self) -> bool {
        // todo: tuple?
        let assignment_expr = self
            .syntax
            .parent_of_type::<ast::BinExpr>()
            .take_if(|it| matches!(it.op_kind(), Some(ast::BinaryOp::Assignment { .. })));
        assignment_expr.is_some_and(|it| {
            it.lhs()
                .is_some_and(|lhs| lhs == ast::Expr::PathExpr(self.clone()))
        })
    }
}
