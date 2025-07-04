// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast;

impl ast::MethodOrPath {
    pub fn type_arg_list(&self) -> Option<ast::TypeArgList> {
        match self {
            ast::MethodOrPath::MethodCallExpr(method_call_expr) => method_call_expr.type_arg_list(),
            ast::MethodOrPath::Path(path) => path.segment()?.type_arg_list(),
        }
    }
}
