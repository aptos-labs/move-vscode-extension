// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use ide_db::{SymbolKind, ast_kind_to_symbol_kind};
use syntax::ast::node_ext::syntax_node::SyntaxNodeExt;
use syntax::{AstNode, NodeOrToken, SourceFile, SyntaxNode, TextRange, WalkEvent, ast};

#[derive(Debug, Clone)]
pub struct StructureNode {
    pub parent: Option<usize>,
    pub label: String,
    pub navigation_range: TextRange,
    pub node_range: TextRange,
    pub kind: SymbolKind,
}

pub(crate) fn file_structure(file: &SourceFile) -> Vec<StructureNode> {
    let mut res = Vec::new();
    let mut stack = Vec::new();

    for event in file.syntax().preorder_with_tokens() {
        match event {
            WalkEvent::Enter(NodeOrToken::Node(node)) => {
                if let Some(mut symbol) = structure_node(&node) {
                    symbol.parent = stack.last().copied();
                    stack.push(res.len());
                    res.push(symbol);
                }
            }
            WalkEvent::Leave(NodeOrToken::Node(node)) => {
                if structure_node(&node).is_some() {
                    stack.pop().unwrap();
                }
            }
            _ => (),
        }
    }
    res
}

fn structure_node(node: &SyntaxNode) -> Option<StructureNode> {
    let named_element = node.cast::<ast::NamedElement>()?;
    let name = named_element.name()?;
    let symbol_kind = ast_kind_to_symbol_kind(&named_element);
    Some(StructureNode {
        parent: None,
        label: name.text().to_string(),
        navigation_range: name.syntax().text_range(),
        node_range: named_element.syntax().text_range(),
        kind: symbol_kind,
    })
}
