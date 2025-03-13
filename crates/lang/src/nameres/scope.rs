use crate::db::HirDatabase;
use crate::loc::{SyntaxLoc, SyntaxLocExt};
use crate::nameres::is_visible::is_visible_in_context;
use crate::nameres::namespaces::{named_item_ns, NsSet, NsSetExt};
use crate::{AsName, InFile, Name};
use std::fmt;
use std::fmt::{Formatter, Pointer};
use syntax::ast::{HasReference, NamedItemScope};
use syntax::{ast, AstNode, SyntaxNode};

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct ScopeEntry {
    pub name: Name,
    pub named_node_loc: SyntaxLoc,
    pub ns: NsSet,
    pub scope_adjustment: Option<NamedItemScope>,
}

impl ScopeEntry {
    pub fn from_named(item: InFile<impl ast::HasName>, ns: NsSet) -> Option<Self> {
        let name = item.value.name()?;
        let loc = item.loc();
        Some(ScopeEntry {
            name: name.as_name(),
            named_node_loc: loc,
            ns,
            scope_adjustment: None,
        })
    }

    pub fn copy_with_ns(&self, ns: NsSet) -> Self {
        let mut entry = self.clone();
        entry.ns = ns;
        entry
    }
}

impl fmt::Debug for ScopeEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ScopeEntry")
            .field(&self.name.as_str().to_string())
            .field(&self.ns)
            .field(&self.named_node_loc)
            .finish()
    }
}

pub trait ScopeEntryExt {
    fn to_entry(self) -> Option<ScopeEntry>;
}

impl<T: ast::HasName> ScopeEntryExt for InFile<T> {
    fn to_entry(self) -> Option<ScopeEntry> {
        let name = self.value.name()?;
        let item_loc = self.loc();
        let item_ns = NsSet::from(named_item_ns(item_loc.kind()));
        let entry = ScopeEntry {
            name: name.as_name(),
            named_node_loc: item_loc,
            ns: item_ns,
            scope_adjustment: None,
        };
        Some(entry)
    }
}

pub trait NamedItemsExt {
    fn to_entries(self) -> Vec<ScopeEntry>;
}

impl<T: ast::HasName> NamedItemsExt for Vec<InFile<T>> {
    fn to_entries(self) -> Vec<ScopeEntry> {
        self.into_iter().filter_map(|item| item.to_entry()).collect()
    }
}

pub trait ScopeEntryListExt {
    fn filter_by_ns(self, ns: NsSet) -> impl Iterator<Item = ScopeEntry>;
    fn filter_by_name(self, name: &str) -> impl Iterator<Item = ScopeEntry>;
    fn filter_by_visibility(
        self,
        db: &dyn HirDatabase,
        context: impl HasReference,
    ) -> impl Iterator<Item = ScopeEntry>;
}

impl<T: Iterator<Item = ScopeEntry>> ScopeEntryListExt for T {
    fn filter_by_ns(self, ns: NsSet) -> impl Iterator<Item = ScopeEntry> {
        self.filter(move |entry| entry.ns.contains_any_of(ns))
    }

    fn filter_by_name(self, name: &str) -> impl Iterator<Item = ScopeEntry> {
        self.filter(move |entry| entry.name.as_str() == name)
    }

    fn filter_by_visibility(
        self,
        db: &dyn HirDatabase,
        context: impl HasReference,
    ) -> impl Iterator<Item = ScopeEntry> {
        self.filter(move |entry| is_visible_in_context(db, entry, &context))
    }
}
