// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast;
use crate::ast::TypeParam;

impl ast::InvariantStmt {
    pub fn type_params(&self) -> Vec<TypeParam> {
        self.spec_type_param_list()
            .map(|it| it.type_parameters().collect())
            .unwrap_or_default()
    }
}
