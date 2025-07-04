// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash)]
pub enum NamedItemScope {
    Main,
    Test,
    Verify,
}

impl NamedItemScope {
    pub fn is_test(self) -> bool {
        self == NamedItemScope::Test
    }

    pub fn shrink_scope(self, adjustment_scope: NamedItemScope) -> NamedItemScope {
        if self == NamedItemScope::Main {
            return adjustment_scope;
        }
        self
    }
}
