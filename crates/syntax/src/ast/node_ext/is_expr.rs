// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast;

impl ast::IsExpr {
    pub fn path_types(&self) -> Vec<ast::PathType> {
        self.types().filter_map(|t| t.path_type()).collect()
    }
}
