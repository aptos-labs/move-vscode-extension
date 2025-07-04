// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast;

impl ast::LabelDecl {
    pub fn name_as_string(&self) -> String {
        self.quote_ident_token().to_string()
    }
}

impl ast::Label {
    pub fn name_as_string(&self) -> String {
        self.quote_ident_token().to_string()
    }
}
