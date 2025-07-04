// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast;

impl ast::SourceFile {
    pub fn all_modules(&self) -> impl Iterator<Item = ast::Module> {
        self.modules()
            .chain(self.address_defs().flat_map(|ad| ad.modules()))
    }
}
