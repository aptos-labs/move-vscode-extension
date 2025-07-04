// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::ast;

impl ast::ItemSpec {
    pub fn item_spec_params(&self) -> Vec<ast::ItemSpecParam> {
        self.item_spec_param_list()
            .map(|it| it.item_spec_params().collect())
            .unwrap_or_default()
    }

    pub fn param_ident_pats(&self) -> Vec<Option<ast::IdentPat>> {
        self.item_spec_params()
            .into_iter()
            .map(|it| it.ident_pat())
            .collect()
    }
}
