// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

mod edit_algo;
pub mod mapping;
mod node_ext;
mod syntax_editor_ext;

use crate::ast::node_ext::syntax_element::SyntaxElementExt;
use crate::ast::node_ext::syntax_node::SyntaxNodeExt;
use crate::syntax_editor::mapping::SyntaxMapping;
use crate::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken, TextRange};
use rowan::Direction;
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::ops::RangeInclusive;
use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Debug)]
pub struct SyntaxEditor {
    root: SyntaxNode,
    changes: Vec<Change>,
    mappings: SyntaxMapping,
}

impl SyntaxEditor {
    /// Creates a syntax editor to start editing from `root`
    pub fn new(root: SyntaxNode) -> Self {
        Self {
            root,
            changes: vec![],
            mappings: SyntaxMapping::new(),
        }
    }

    pub fn merge(&mut self, mut other: SyntaxEditor) {
        debug_assert!(
            self.root == other.root || other.root.ancestors().any(|node| node == self.root),
            "{:?} is not in the same tree as {:?}",
            other.root,
            self.root
        );

        self.changes.append(&mut other.changes);
        self.mappings.merge(other.mappings);
    }

    pub fn insert(&mut self, position: Position, element: impl Element) {
        debug_assert!(
            is_ancestor_or_self(&position.parent(), &self.root),
            "position {:?} is not a child of {:?}",
            position.place(),
            self.root
        );
        self.changes
            .push(Change::Insert(position, element.syntax_element()))
    }

    pub fn insert_all(&mut self, position: Position, elements: Vec<SyntaxElement>) {
        debug_assert!(is_ancestor_or_self(&position.parent(), &self.root));
        self.changes.push(Change::InsertAll(position, elements))
    }

    pub fn delete(&mut self, element: impl Element) {
        let element = element.syntax_element();
        debug_assert!(is_ancestor_or_self_of_element(&element, &self.root));
        debug_assert!(
            !matches!(&element, SyntaxElement::Node(node) if node == &self.root),
            "should not delete root node"
        );
        self.changes.push(Change::Replace(element.syntax_element(), None));
    }

    pub fn delete_comma_sep_list_element(&mut self, node: &SyntaxNode) {
        // delete surrounding trivia
        for trivia_sibling in node
            .next_siblings_with_tokens()
            .take_while(|it| it.kind().is_trivia())
            .chain(
                node.prev_siblings_with_tokens()
                    .take_while(|it| it.kind().is_trivia()),
            )
        {
            self.delete(trivia_sibling);
        }
        // delete surrounding comma
        let syntax_element = node.syntax_element();
        if let Some(following_comma) = &syntax_element.following_comma() {
            self.delete(following_comma);
        } else if let Some(preceding_comma) = &syntax_element.preceding_comma() {
            self.delete(preceding_comma);
        }
        self.delete(node)
    }

    pub fn replace(&mut self, old: impl Element, new: impl Element) {
        let old = old.syntax_element();
        debug_assert!(is_ancestor_or_self_of_element(&old, &self.root));
        self.changes
            .push(Change::Replace(old.syntax_element(), Some(new.syntax_element())));
    }

    pub fn replace_with_many(&mut self, old: impl Element, new: Vec<SyntaxElement>) {
        let old = old.syntax_element();
        debug_assert!(is_ancestor_or_self_of_element(&old, &self.root));
        debug_assert!(
            !(matches!(&old, SyntaxElement::Node(node) if node == &self.root) && new.len() > 1),
            "cannot replace root node with many elements"
        );
        self.changes
            .push(Change::ReplaceWithMany(old.syntax_element(), new));
    }

    pub fn replace_all(&mut self, range: RangeInclusive<SyntaxElement>, new: Vec<SyntaxElement>) {
        if range.start() == range.end() {
            self.replace_with_many(range.start(), new);
            return;
        }

        debug_assert!(is_ancestor_or_self_of_element(range.start(), &self.root));
        self.changes.push(Change::ReplaceAll(range, new))
    }

    pub fn finish(self) -> SyntaxEdit {
        edit_algo::apply_edits(self)
    }

    pub fn add_mappings(&mut self, other: SyntaxMapping) {
        self.mappings.merge(other);
    }
}

/// Represents a completed [`SyntaxEditor`] operation.
pub struct SyntaxEdit {
    old_root: SyntaxNode,
    new_root: SyntaxNode,
    changed_elements: Vec<SyntaxElement>,
}

impl SyntaxEdit {
    /// Root of the initial unmodified syntax tree.
    pub fn old_root(&self) -> &SyntaxNode {
        &self.old_root
    }

    /// Root of the modified syntax tree.
    pub fn new_root(&self) -> &SyntaxNode {
        &self.new_root
    }

    /// Which syntax elements in the modified syntax tree were inserted or
    /// modified as part of the edit.
    ///
    /// Note that for syntax nodes, only the upper-most parent of a set of
    /// changes is included, not any child elements that may have been modified.
    pub fn changed_elements(&self) -> &[SyntaxElement] {
        self.changed_elements.as_slice()
    }
}

/// Position describing where to insert elements
#[derive(Debug)]
pub struct Position {
    repr: PositionRepr,
}

impl Position {
    pub(crate) fn parent(&self) -> SyntaxNode {
        self.place().0
    }

    pub(crate) fn place(&self) -> (SyntaxNode, usize) {
        match &self.repr {
            PositionRepr::FirstChild(parent) => (parent.clone(), 0),
            PositionRepr::After(child) => (child.parent().unwrap(), child.index() + 1),
        }
    }
}

#[derive(Debug)]
enum PositionRepr {
    FirstChild(SyntaxNode),
    After(SyntaxElement),
}

impl Position {
    pub fn after(elem: impl Element) -> Position {
        let repr = PositionRepr::After(elem.syntax_element());
        Position { repr }
    }

    pub fn before(elem: impl Element) -> Position {
        let elem = elem.syntax_element();
        let repr = match elem.prev_sibling_or_token() {
            Some(it) => PositionRepr::After(it),
            None => PositionRepr::FirstChild(
                elem.parent()
                    .expect(&format!("{:?}.parent() is None", elem.kind())),
            ),
        };
        Position { repr }
    }

    pub fn first_child_of(node: &(impl Into<SyntaxNode> + Clone)) -> Position {
        let repr = PositionRepr::FirstChild(node.clone().into());
        Position { repr }
    }

    pub fn last_child_of(node: &(impl Into<SyntaxNode> + Clone)) -> Position {
        let node = node.clone().into();
        let repr = match node.last_child_or_token() {
            Some(it) => PositionRepr::After(it),
            None => PositionRepr::FirstChild(node),
        };
        Position { repr }
    }
}

#[derive(Debug)]
enum Change {
    /// Inserts a single element at the specified position.
    Insert(Position, SyntaxElement),
    /// Inserts many elements in-order at the specified position.
    InsertAll(Position, Vec<SyntaxElement>),
    /// Represents both a replace single element and a delete element operation.
    Replace(SyntaxElement, Option<SyntaxElement>),
    /// Replaces a single element with many elements.
    ReplaceWithMany(SyntaxElement, Vec<SyntaxElement>),
    /// Replaces a range of elements with another list of elements.
    /// Range will always have start != end.
    ReplaceAll(RangeInclusive<SyntaxElement>, Vec<SyntaxElement>),
}

impl Change {
    fn target_range(&self) -> TextRange {
        match self {
            Change::Insert(target, _) | Change::InsertAll(target, _) => match &target.repr {
                PositionRepr::FirstChild(parent) => TextRange::at(
                    parent.first_child_or_token().unwrap().text_range().start(),
                    0.into(),
                ),
                PositionRepr::After(child) => TextRange::at(child.text_range().end(), 0.into()),
            },
            Change::Replace(target, _) | Change::ReplaceWithMany(target, _) => target.text_range(),
            Change::ReplaceAll(range, _) => range.start().text_range().cover(range.end().text_range()),
        }
    }

    fn target_parent(&self) -> SyntaxNode {
        match self {
            Change::Insert(target, _) | Change::InsertAll(target, _) => target.parent(),
            Change::Replace(target, _) | Change::ReplaceWithMany(target, _) => match target {
                SyntaxElement::Node(target) => target.parent().unwrap_or_else(|| target.clone()),
                SyntaxElement::Token(target) => target.parent().unwrap(),
            },
            Change::ReplaceAll(target, _) => target.start().parent().unwrap(),
        }
    }

    fn change_kind(&self) -> ChangeKind {
        match self {
            Change::Insert(_, _) | Change::InsertAll(_, _) => ChangeKind::Insert,
            Change::Replace(_, _) | Change::ReplaceWithMany(_, _) => ChangeKind::Replace,
            Change::ReplaceAll(_, _) => ChangeKind::ReplaceRange,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum ChangeKind {
    Insert,
    ReplaceRange,
    Replace,
}

/// Utility trait to allow calling syntax editor functions with references or owned
/// nodes. Do not use outside of this module.
pub trait Element {
    fn syntax_element(self) -> SyntaxElement;
}

impl<E: Element + Clone> Element for &'_ E {
    fn syntax_element(self) -> SyntaxElement {
        self.clone().syntax_element()
    }
}

impl Element for SyntaxElement {
    fn syntax_element(self) -> SyntaxElement {
        self
    }
}

impl Element for SyntaxNode {
    fn syntax_element(self) -> SyntaxElement {
        self.into()
    }
}

impl Element for SyntaxToken {
    fn syntax_element(self) -> SyntaxElement {
        self.into()
    }
}

fn is_ancestor_or_self(node: &SyntaxNode, ancestor: &SyntaxNode) -> bool {
    node == ancestor || node.ancestors().any(|it| &it == ancestor)
}

fn is_ancestor_or_self_of_element(node: &SyntaxElement, ancestor: &SyntaxNode) -> bool {
    matches!(node, SyntaxElement::Node(node) if node == ancestor)
        || node.ancestors().any(|it| &it == ancestor)
}
