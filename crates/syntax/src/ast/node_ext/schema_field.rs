// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast;

impl ast::SchemaField {
    pub fn name(&self) -> Option<ast::Name> {
        self.ident_pat().and_then(|it| it.name())
    }
}
