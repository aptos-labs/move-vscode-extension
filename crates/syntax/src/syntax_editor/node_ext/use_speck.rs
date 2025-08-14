// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::ast::node_ext::syntax_element::SyntaxElementExt;
use crate::syntax_editor::{Element, SyntaxEditor};
use crate::{AstNode, ast};

impl ast::UseSpeck {
    pub fn delete(&self, editor: &mut SyntaxEditor) {
        // if it's a first use speck in a group, remove ws after the following comma
        if let Some(use_group) = self.parent_use_group()
            && use_group.use_specks().next().is_some_and(|it| &it == self)
        {
            if let Some(following_ws) = self
                .syntax()
                .following_comma()
                .and_then(|it| it.syntax_element().following_ws())
            {
                editor.delete(following_ws);
            }
        }
        editor.delete_comma_sep_list_element(self.syntax())
    }
}
