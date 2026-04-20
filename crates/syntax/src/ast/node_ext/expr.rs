// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::{AstNode, ast};

impl ast::Expr {
    pub fn is_block_like(&self) -> bool {
        matches!(
            self,
            ast::Expr::IfExpr(_)
                | ast::Expr::LoopExpr(_)
                | ast::Expr::ForExpr(_)
                | ast::Expr::WhileExpr(_)
                | ast::Expr::BlockExpr(_)
                | ast::Expr::MatchExpr(_)
        )
    }

    pub fn borrow_global_index_expr(self) -> Option<(ast::PathExpr, Option<ast::Expr>)> {
        let borrow_expr = self.borrow_expr()?;
        let index_expr = borrow_expr.expr()?.index_expr()?;
        let base_path_expr = index_expr.base_expr().path_expr()?;
        Some((base_path_expr, index_expr.arg_expr()))
    }
}
