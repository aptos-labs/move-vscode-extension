// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::{TextRange, ast};

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

    pub fn signature_range(&self) -> TextRange {
        let Some(item_spec_ref) = self.item_spec_ref() else {
            return TextRange::empty(self.syntax.text_range().start());
        };
        let Some(start) = self.spec_token().map(|it| it.text_range().start()) else {
            return item_spec_ref.syntax.text_range();
        };
        let end = self
            .spec_block()
            .and_then(|block| block.l_curly_token())
            .map(|it| it.text_range().start())
            .unwrap_or(item_spec_ref.syntax.text_range().end());
        TextRange::new(start, end)
    }
}
