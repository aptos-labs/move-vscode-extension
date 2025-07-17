// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast;

impl ast::MethodOrPath {
    pub fn reference(&self) -> ast::ReferenceElement {
        match self {
            ast::MethodOrPath::MethodCallExpr(method_call_expr) => method_call_expr.clone().into(),
            ast::MethodOrPath::Path(path) => path.clone().into(),
        }
    }

    pub fn name_ref(&self) -> Option<ast::NameRef> {
        match self {
            ast::MethodOrPath::MethodCallExpr(method_call_expr) => method_call_expr.name_ref(),
            ast::MethodOrPath::Path(path) => path.segment().and_then(|it| it.name_ref()),
        }
    }

    pub fn type_arg_list(&self) -> Option<ast::TypeArgList> {
        match self {
            ast::MethodOrPath::MethodCallExpr(method_call_expr) => method_call_expr.type_arg_list(),
            ast::MethodOrPath::Path(path) => path.segment()?.type_arg_list(),
        }
    }

    pub fn type_args(&self) -> Vec<ast::TypeArg> {
        self.type_arg_list().map(|it| it.type_args()).unwrap_or_default()
    }
}
