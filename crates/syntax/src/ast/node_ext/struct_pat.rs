// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast;

impl ast::StructPat {
    pub fn fields(&self) -> Vec<ast::StructPatField> {
        self.struct_pat_field_list()
            .map(|it| it.fields().collect())
            .unwrap_or_default()
    }
}
