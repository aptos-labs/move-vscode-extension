// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::item_scope::ItemScope;
use crate::loc::{SyntaxLoc, SyntaxLocFileExt};
use crate::nameres::is_visible::ScopeEntryWithVis;
use crate::nameres::namespaces::{Ns, NsSet, named_item_ns};
use crate::types::ty::Ty;
use base_db::SourceDatabase;
use std::fmt;
use std::fmt::Formatter;
use std::vec::IntoIter;
use stdx::itertools::Itertools;
use syntax::SyntaxKind::{IDENT_PAT, NAMED_FIELD};
use syntax::files::{InFile, InFileExt};
use syntax::{AstNode, SyntaxKind, ast};
use vfs::FileId;

#[derive(Clone, Eq, Hash)]
pub struct ScopeEntry {
    pub name: String,
    pub node_loc: SyntaxLoc,
    pub ns: Ns,
    pub scope_adjustment: Option<ItemScope>,
}

impl PartialEq for ScopeEntry {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.node_loc == other.node_loc
    }
}

impl ScopeEntry {
    pub fn copy_with_ns(&self, ns: Ns) -> Self {
        let mut entry = self.clone();
        entry.ns = ns;
        entry
    }

    pub fn kind(&self) -> SyntaxKind {
        self.node_loc.kind()
    }

    pub fn cast_into<T: AstNode>(&self, db: &dyn SourceDatabase) -> Option<InFile<T>> {
        self.node_loc.to_ast(db)
    }
}

impl fmt::Debug for ScopeEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut t = f.debug_tuple("ScopeEntry");
        if self.node_loc.node_name().is_none_or(|it| it != &self.name) {
            return t.field(&self.name).field(&self.node_loc).finish();
        }
        t.field(&self.node_loc).finish()
    }
}

pub trait ScopeEntryExt {
    fn named_element(self) -> InFile<ast::NamedElement>;

    fn to_entry(self) -> Option<ScopeEntry>
    where
        Self: Sized,
    {
        let named_element = self.named_element();
        let name = named_element.value.name()?.as_string();
        let item_loc = named_element.loc();
        let item_ns = named_item_ns(item_loc.kind());
        let entry = ScopeEntry {
            name,
            node_loc: item_loc,
            ns: item_ns,
            scope_adjustment: None,
        };
        Some(entry)
    }
}

impl<Named: Into<ast::NamedElement>> ScopeEntryExt for InFile<Named> {
    fn named_element(self) -> InFile<ast::NamedElement> {
        self.map(|it| it.into())
    }
}

pub trait NamedItemsExt {
    fn to_entries(self) -> Vec<ScopeEntry>;
}

impl<Named: Into<ast::NamedElement>> NamedItemsExt for Vec<InFile<Named>> {
    fn to_entries(self) -> Vec<ScopeEntry> {
        let mut res = Vec::with_capacity(self.len());
        for item in self.into_iter() {
            if let Some(entry) = item.to_entry() {
                res.push(entry);
            }
        }
        res
    }
}

pub trait NamedItemsInFileExt {
    fn to_entries(self, file_id: FileId) -> Vec<ScopeEntry>;
}

impl<Named: Into<ast::NamedElement>> NamedItemsInFileExt for Vec<Named> {
    fn to_entries(self, file_id: FileId) -> Vec<ScopeEntry> {
        let mut entries = Vec::with_capacity(self.len());
        for item in self.into_iter() {
            let named_element = item.into().in_file(file_id);
            if let Some(entry) = named_element.to_entry() {
                entries.push(entry);
            }
        }
        entries
    }
}

pub trait VecExt {
    type Item;
    fn single_or_none(self) -> Option<Self::Item>;
}

impl<T> VecExt for Vec<T> {
    type Item = T;
    fn single_or_none(self) -> Option<T> {
        self.into_iter().exactly_one().ok()
    }
}

pub trait ScopeEntryListExt {
    fn into_iterator(self) -> IntoIter<ScopeEntry>
    where
        Self: Sized;

    fn filter_by_ns(self, ns: NsSet) -> Vec<ScopeEntry>
    where
        Self: Sized,
    {
        self.into_iterator()
            .filter(|entry| ns.contains(entry.ns))
            .collect()
    }

    fn filter_by_name(self, name: String) -> Vec<ScopeEntry>
    where
        Self: Sized,
    {
        self.into_iterator().filter(|entry| entry.name == name).collect()
    }

    fn filter_by_expected_type(
        self,
        db: &dyn SourceDatabase,
        expected_type: Option<Ty>,
    ) -> Vec<ScopeEntry>
    where
        Self: Sized,
    {
        self.into_iterator()
            .filter_map(|entry| {
                let item = entry.cast_into::<ast::NamedElement>(db)?;
                let Some(variant_item) = item.cast_into::<ast::Variant>() else {
                    return Some(entry);
                };
                // if expected type is unknown, or not a enum, then we cannot infer enum variants
                let ty_adt = expected_type.clone()?.unwrap_all_refs().into_ty_adt()?;
                let expected_enum = ty_adt.adt_item(db)?.value.enum_()?;

                let is_valid_item = expected_enum.variants().contains(&variant_item.value);
                is_valid_item.then_some(entry)
            })
            .collect()
    }

    fn into_always_visible_entries(self) -> Vec<ScopeEntryWithVis>
    where
        Self: Sized,
    {
        self.into_iterator().map(|it| it.into_always_visible()).collect()
    }
}

impl ScopeEntryListExt for Vec<ScopeEntry> {
    fn into_iterator(self) -> IntoIter<ScopeEntry> {
        self.into_iter()
    }
}

pub fn into_field_shorthand_items(
    db: &dyn SourceDatabase,
    mut entries: Vec<ScopeEntry>,
) -> Option<(InFile<ast::NamedField>, InFile<ast::IdentPat>)> {
    if entries.len() != 2 {
        return None;
    }
    let named_field = entries
        .remove(entries.iter().position(|it| it.kind() == NAMED_FIELD)?)
        .cast_into::<ast::NamedField>(db)?;
    let ident_pat = entries
        .remove(entries.iter().position(|it| it.kind() == IDENT_PAT)?)
        .cast_into::<ast::IdentPat>(db)?;
    Some((named_field, ident_pat))
}
