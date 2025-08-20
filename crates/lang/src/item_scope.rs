// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use base_db::inputs::FileIdInput;
use base_db::{SourceDatabase, source_db};
use std::collections::{HashMap, HashSet};
use syntax::{AstNode, NodeOrToken, SyntaxNode, SyntaxNodePtr, WalkEvent, ast};

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash)]
pub enum NamedItemScope {
    Main,
    Test,
    Verify,
}

impl NamedItemScope {
    pub fn all() -> Vec<NamedItemScope> {
        vec![NamedItemScope::Main, NamedItemScope::Test, NamedItemScope::Verify]
    }

    pub fn is_test(self) -> bool {
        self == NamedItemScope::Test
    }

    pub fn shrink_scope(self, adjustment_scope: NamedItemScope) -> NamedItemScope {
        if self == NamedItemScope::Main {
            return adjustment_scope;
        }
        self
    }
}

#[salsa_macros::tracked(returns(ref))]
pub(crate) fn item_scopes(
    db: &dyn SourceDatabase,
    file_id: FileIdInput,
) -> HashMap<SyntaxNodePtr, NamedItemScope> {
    let mut map = HashMap::new();
    let mut scope_nodes = vec![];

    let file = source_db::parse(db, file_id).tree();
    for event in file.syntax().preorder_with_tokens() {
        match event {
            WalkEvent::Enter(NodeOrToken::Node(node)) => {
                let node_ptr = SyntaxNodePtr::new(&node);
                let explicit_item_scope = node_item_scope(node.clone());
                match explicit_item_scope {
                    NamedItemScope::Main => {
                        // gets its scope from the last scoped node
                        let last_scope = scope_nodes
                            .last()
                            .map(|(_, it)| *it)
                            .unwrap_or(NamedItemScope::Main);
                        map.insert(node_ptr, last_scope);
                    }
                    NamedItemScope::Test | NamedItemScope::Verify => {
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

fn node_item_scope(node: SyntaxNode) -> NamedItemScope {
    use syntax::SyntaxKind::*;
    if matches!(
        node.kind(),
        SCHEMA | SPEC_FUN | SPEC_INLINE_FUN | ITEM_SPEC | MODULE_SPEC | SPEC_BLOCK_EXPR
    ) {
        return NamedItemScope::Verify;
    }
    if let Some(has_attrs) = ast::AnyHasAttrs::cast(node) {
        if let Some(ancestor_scope) = item_scope_from_attributes(has_attrs)
            && ancestor_scope != NamedItemScope::Main
        {
            return ancestor_scope;
        }
    }
    NamedItemScope::Main
}

fn item_scope_from_attributes(attrs_owner: impl ast::HasAttrs) -> Option<NamedItemScope> {
    let atom_attr_items = attrs_owner
        .attr_items()
        .filter_map(|it| it.path_text())
        .collect::<HashSet<_>>();
    if atom_attr_items.is_empty() {
        return None;
    }
    if atom_attr_items.contains("test_only") || atom_attr_items.contains("test") {
        return Some(NamedItemScope::Test);
    }
    if atom_attr_items.contains("verify_only") {
        return Some(NamedItemScope::Verify);
    }
    None
}
