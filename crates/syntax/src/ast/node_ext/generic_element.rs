// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast;

impl ast::GenericElement {
    pub fn type_params(&self) -> Vec<ast::TypeParam> {
        self.type_param_list()
            .map(|l| l.type_parameters().collect())
            .unwrap_or_default()
    }
}
