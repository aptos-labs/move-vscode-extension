// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use base_db::inputs::FileIdInput;
use base_db::{SourceDatabase, source_db};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use syntax::{AstNode, NodeOrToken, SyntaxNode, SyntaxNodePtr, WalkEvent, ast};

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash, Serialize, Deserialize)]
pub enum ItemScope {
    Main,
    Test,
    Verify,
}

impl Display for ItemScope {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ItemScope::Main => write!(f, "main"),
            ItemScope::Test => write!(f, "test_only"),
            ItemScope::Verify => write!(f, "verify_only"),
        }
    }
}

impl ItemScope {
    pub fn all() -> Vec<ItemScope> {
        vec![ItemScope::Main, ItemScope::Test, ItemScope::Verify]
    }

    pub fn is_test(self) -> bool {
        self == ItemScope::Test
    }

    pub fn is_covered_by_the_use_stmt_scope(&self, use_stmt_scope: ItemScope) -> bool {
        match self {
            ItemScope::Test => use_stmt_scope == ItemScope::Test,
            ItemScope::Main => use_stmt_scope == ItemScope::Main,
            ItemScope::Verify => {
                use_stmt_scope == ItemScope::Main || use_stmt_scope == ItemScope::Verify
            }
        }
    }

    pub fn shrink_scope(self, adjustment_scope: ItemScope) -> ItemScope {
        if self == ItemScope::Main {
            return adjustment_scope;
        }
        self
    }
}

#[salsa_macros::tracked(returns(ref))]
pub(crate) fn item_scopes(
    db: &dyn SourceDatabase,
    file_id: FileIdInput,
) -> HashMap<SyntaxNodePtr, ItemScope> {
    let mut map = HashMap::new();
    let mut scope_nodes = vec![];

    let file = source_db::parse(db, file_id).tree();
    for event in file.syntax().preorder_with_tokens() {
        match event {
            WalkEvent::Enter(NodeOrToken::Node(node)) => {
                let node_ptr = SyntaxNodePtr::new(&node);
                let explicit_item_scope = node_item_scope(node.clone());
                match explicit_item_scope {
                    ItemScope::Main => {
                        // gets its scope from the last scoped node
                        let last_scope =
                            scope_nodes.last().map(|(_, it)| *it).unwrap_or(ItemScope::Main);
                        map.insert(node_ptr, last_scope);
                    }
                    ItemScope::Test | ItemScope::Verify => {
                        scope_nodes.push((node.clone(), explicit_item_scope));
                        // gets its scope from its own attribute
                        map.insert(node_ptr, explicit_item_scope);
                    }
                }
            }
            WalkEvent::Leave(NodeOrToken::Node(node)) => {
                if let Some((last_scope_node, _)) = scope_nodes.last()
                    && last_scope_node == &node
                {
                    scope_nodes.pop();
                }
            }
            _ => (),
        }
    }
    map
}

fn node_item_scope(node: SyntaxNode) -> ItemScope {
    use syntax::SyntaxKind::*;
    if matches!(
        node.kind(),
        SCHEMA | SPEC_FUN | SPEC_INLINE_FUN | ITEM_SPEC | MODULE_SPEC | SPEC_BLOCK_EXPR
    ) {
        return ItemScope::Verify;
    }
    if let Some(has_attrs) = ast::AnyHasAttrs::cast(node) {
        if let Some(ancestor_scope) = item_scope_from_attributes(has_attrs)
            && ancestor_scope != ItemScope::Main
        {
            return ancestor_scope;
        }
    }
    ItemScope::Main
}

fn item_scope_from_attributes(attrs_owner: impl ast::HasAttrs) -> Option<ItemScope> {
    let atom_attr_items = attrs_owner
        .attr_items()
        .filter_map(|it| it.path_text())
        .collect::<HashSet<_>>();
    if atom_attr_items.is_empty() {
        return None;
    }
    if atom_attr_items.contains("test_only") || atom_attr_items.contains("test") {
        return Some(ItemScope::Test);
    }
    if atom_attr_items.contains("verify_only") {
        return Some(ItemScope::Verify);
    }
    None
}
