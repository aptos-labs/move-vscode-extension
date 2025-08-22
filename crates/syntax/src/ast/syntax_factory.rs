// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

//! Builds upon [`ast::make`] constructors to create ast fragments with
//! optional syntax mappings.
//!
//! Instead of forcing make constructors to perform syntax mapping, we instead
//! let [`SyntaxFactory`] handle constructing the mappings. Care must be taken
//! to remember to feed the syntax mappings into a [`SyntaxEditor`](crate::syntax_editor::SyntaxEditor),
//! if applicable.

mod constructors;
mod exprs;

use crate::syntax_editor::mapping::SyntaxMapping;
use crate::{AstNode, SourceFile, ast};
use std::cell::{RefCell, RefMut};
use std::thread::panicking;

pub struct SyntaxFactory {
    // Stored in a refcell so that the factory methods can be &self
    mappings: Option<RefCell<SyntaxMapping>>,
}

impl SyntaxFactory {
    /// Creates a new [`SyntaxFactory`], generating mappings between input nodes and generated nodes.
    pub fn new() -> Self {
        Self {
            mappings: Some(RefCell::new(SyntaxMapping::default())),
        }
    }

    /// Creates a [`SyntaxFactory`] without generating mappings.
    pub fn without_mappings() -> Self {
        Self { mappings: None }
    }

    /// Gets all of the tracked syntax mappings, if any.
    pub fn finish_with_mappings(self) -> SyntaxMapping {
        self.mappings.unwrap_or_default().into_inner()
    }

    /// Take all of the tracked syntax mappings, leaving `SyntaxMapping::default()` in its place, if any.
    pub fn take(&self) -> SyntaxMapping {
        self.mappings
            .as_ref()
            .map(|mappings| mappings.take())
            .unwrap_or_default()
    }

    pub(crate) fn mappings(&self) -> Option<RefMut<'_, SyntaxMapping>> {
        self.mappings.as_ref().map(|it| it.borrow_mut())
    }
}

impl Default for SyntaxFactory {
    fn default() -> Self {
        Self::without_mappings()
    }
}

fn type_from_text(text: &str) -> ast::Type {
    ast_from_text(&format!("module 0x1::m {{ const M: {}; }}", text))
}

fn module_item_from_text<T: AstNode>(text: &str) -> T {
    ast_from_text(&format!("module 0x1::m {{ {text} }}"))
}

fn expr_item_from_text<T: AstNode>(text: &str) -> T {
    ast_from_text(&format!("module 0x1::m {{ fun main() {{ let _ = {text} }} }}"))
}

#[track_caller]
fn ast_from_text<N: AstNode>(text: &str) -> N {
    let parse = SourceFile::parse(text);
    let node = match parse.tree().syntax().descendants().find_map(N::cast) {
        Some(it) => it,
        None => {
            let node = std::any::type_name::<N>();
            panic!("Failed to make ast node `{node}` from text `{text}`")
        }
    };
    let node = node.clone_subtree();
    assert_eq!(node.syntax().text_range().start(), 0.into());
    node
}
