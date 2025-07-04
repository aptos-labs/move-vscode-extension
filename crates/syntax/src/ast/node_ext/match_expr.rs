// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast;

impl ast::MatchExpr {
    pub fn arms(&self) -> Vec<ast::MatchArm> {
        self.match_arm_list()
            .map(|it| it.match_arms().collect())
            .unwrap_or_default()
    }
}
