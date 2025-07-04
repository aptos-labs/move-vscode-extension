// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast;
use crate::ast::WildcardPattern;

impl ast::ApplySchema {
    pub fn apply_to_patterns(&self) -> Vec<WildcardPattern> {
        self.apply_to()
            .map(|it| it.wildcards().collect())
            .unwrap_or_default()
    }

    pub fn apply_except_patterns(&self) -> Vec<WildcardPattern> {
        self.apply_except()
            .map(|it| it.wildcards().collect())
            .unwrap_or_default()
    }
}
