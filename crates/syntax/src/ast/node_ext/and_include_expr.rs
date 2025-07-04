// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast::{SchemaLit, support};
use crate::{AstNode, ast};

impl ast::AndIncludeExpr {
    pub fn left_schema_lit(&self) -> Option<SchemaLit> {
        support::children(self.syntax()).nth(0)
    }

    pub fn right_schema_lit(&self) -> Option<SchemaLit> {
        support::children(self.syntax()).nth(1)
    }
}
