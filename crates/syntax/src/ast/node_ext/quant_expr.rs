// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast;

impl ast::QuantExpr {
    pub fn quant_bindings(&self) -> Vec<ast::QuantBinding> {
        self.quant_binding_list()
            .map(|it| it.bindings().collect())
            .unwrap_or_default()
    }
}
