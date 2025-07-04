// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast;

impl ast::VectorLitExpr {
    pub fn type_arg(&self) -> Option<ast::TypeArg> {
        self.type_arg_list()?.type_arguments().next()
    }
}
