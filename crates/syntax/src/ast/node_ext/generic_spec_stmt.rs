// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast;
use crate::ast::TypeParam;

impl ast::GenericSpecStmt {
    pub fn type_params(&self) -> Vec<TypeParam> {
        match self {
            ast::GenericSpecStmt::AxiomStmt(stmt) => stmt.type_params(),
            ast::GenericSpecStmt::InvariantStmt(stmt) => stmt.type_params(),
        }
    }
}
