// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

mod docs;
pub mod has_item_list;
pub mod has_use_stmts;

use crate::ast::{AstChildren, Stmt, support};
use crate::{AstNode, ast};
pub use docs::HoverDocsOwner;
pub use has_item_list::HasItems;
pub use has_use_stmts::UseStmtsOwner;
use itertools::Itertools;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::io::Read;

// pub trait NamedElement: AstNode {
//     fn name(&self) -> Option<ast::Name> {
//         support::child(self.syntax())
//     }
// }

pub(crate) fn into_named_elements(items: Vec<impl Into<ast::NamedElement>>) -> Vec<ast::NamedElement> {
    items.into_iter().map(|it| it.into()).collect()
}

impl ast::Name {
    pub fn as_string(&self) -> String {
        self.ident_token().to_string()
    }
}

pub trait HasStmts: AstNode {
    fn stmts(&self) -> AstChildren<Stmt> {
        support::children(&self.syntax())
    }

    fn let_stmts(&self) -> impl Iterator<Item = ast::LetStmt> {
        self.stmts().filter_map(|it| it.let_stmt())
    }

    fn global_variables(&self) -> Vec<ast::GlobalVariableDecl> {
        self.stmts().filter_map(|it| it.global_variable_decl()).collect()
    }
}

pub trait HasAttrs: AstNode {
    fn attrs(&self) -> AstChildren<ast::Attr> {
        support::children(self.syntax())
    }

    fn attr_items(&self) -> impl Iterator<Item = ast::AttrItem> {
        self.attrs().flat_map(|it| it.attr_items())
    }

    fn has_attr_item(&self, path_text: &str) -> bool {
        self.attr_items()
            .any(|it| it.path_text().is_some_and(|text| text == path_text))
    }

    fn has_attr_item_with_name_only(&self, path_text: &str) -> bool {
        self.attr_items()
            .filter(|it| it.is_name_only())
            .any(|it| it.path_text().is_some_and(|text| text == path_text))
    }

    fn is_test_only(&self) -> bool {
        self.has_attr_item_with_name_only("test_only")
    }

    fn is_verify_only(&self) -> bool {
        self.has_attr_item_with_name_only("verify_only")
    }
}

pub trait MslOnly: AstNode {}
