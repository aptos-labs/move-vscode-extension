// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast;

impl ast::IdentPatOwner {
    pub fn kind(&self) -> String {
        let kind = match self {
            ast::IdentPatOwner::Param(_) => "parameter",
            ast::IdentPatOwner::LambdaParam(_) => "parameter",
            ast::IdentPatOwner::ItemSpecParam(_) => "parameter",
            ast::IdentPatOwner::LetStmt(_) => "variable",
            ast::IdentPatOwner::ForCondition(_) => "variable",
            ast::IdentPatOwner::SchemaField(_) => "field",
            ast::IdentPatOwner::QuantBinding(_) => "variable",
        };
        kind.to_string()
    }
}
