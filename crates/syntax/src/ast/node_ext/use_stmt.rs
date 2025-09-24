// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::ast;
use crate::ast::node_ext::syntax_element::SyntaxElementExt;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::ast::node_ext::use_speck::UseSpeckKind;

impl ast::UseStmt {
    pub fn owner(&self) -> Option<ast::AnyUseStmtsOwner> {
        self.syntax.parent_of_type()
    }

    pub fn module_path(&self) -> Option<ast::Path> {
        let root_use_speck_kind = self.use_speck()?.kind()?;
        match root_use_speck_kind {
            UseSpeckKind::Module { path, .. } => Some(path),
            UseSpeckKind::Item { module_path, .. } => Some(module_path),
            UseSpeckKind::Group { module_path, .. } => Some(module_path),
            UseSpeckKind::GroupNameRef { .. } => unreachable!(),
        }
    }

    pub fn path(&self) -> Option<ast::Path> {
        self.use_speck().and_then(|it| it.path())
    }

    pub fn use_group(&self) -> Option<ast::UseGroup> {
        self.use_speck().and_then(|it| it.use_group())
    }

    pub fn group_use_specks(&self) -> Vec<ast::UseSpeck> {
        self.use_speck()
            .and_then(|it| it.use_group())
            .map(|it| it.use_specks().collect())
            .unwrap_or_default()
    }
}
