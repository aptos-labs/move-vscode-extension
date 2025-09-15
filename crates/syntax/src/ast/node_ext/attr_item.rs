// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::ast::node_ext::syntax_element::SyntaxElementExt;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::{AstNode, ast};

impl ast::AttrItem {
    pub fn attr(&self) -> Option<ast::Attr> {
        self.syntax.parent_of_type::<ast::Attr>()
    }

    pub fn is_top_level(&self) -> bool {
        self.attr().is_some()
    }

    pub fn parent_attr_item(&self) -> Option<ast::AttrItem> {
        self.parent_attr_item_list()?
            .syntax
            .parent_of_type::<ast::AttrItem>()
    }

    pub fn parent_attr_item_list(&self) -> Option<ast::AttrItemList> {
        self.syntax.parent_of_type::<ast::AttrItemList>()
    }

    pub fn is_name_only(&self) -> bool {
        self.initializer().is_none() && self.attr_item_list().is_none()
    }

    pub fn path_text(&self) -> Option<String> {
        let path = self.path()?;
        Some(path.syntax().text().to_string())
    }

    pub fn is_abort_code(&self) -> bool {
        if self.path_text().is_none_or(|it| it != "abort_code") {
            return false;
        }
        // confirm that the position is correct
        if let Some(parent_attr_item) = self.parent_attr_item()
            && parent_attr_item.is_top_level()
        {
            return parent_attr_item
                .path_text()
                .is_some_and(|name| name == "expected_failure");
        };
        false
    }
}
