// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::ast;

impl ast::UseSpeck {
    pub fn path_name(&self) -> Option<String> {
        self.path()?.reference_name()
    }
}
