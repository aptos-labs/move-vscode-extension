// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::{AstNode, ast};

impl ast::Attr {
    pub fn single_attr_item_or_none(&self) -> Option<ast::AttrItem> {
        let mut attr_items = self.attr_items().collect::<Vec<_>>();
        match attr_items.len() {
            1 => attr_items.pop(),
            _ => None,
        }
    }

    pub fn single_attr_item_name(&self) -> Option<String> {
        self.single_attr_item_or_none()?.no_qual_name()
    }
}
