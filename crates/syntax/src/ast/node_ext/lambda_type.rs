// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast;

impl ast::LambdaType {
    pub fn param_types(&self) -> Vec<ast::Type> {
        self.lambda_type_params().map(|it| it.type_()).collect()
    }
}
