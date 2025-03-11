use crate::nameres::namespaces::{named_item_ns, NsSet, NsSetExt};
use crate::{AsName, Name};
use syntax::{ast, AstNode, SyntaxNode};

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ScopeEntry {
    pub name: Name,
    pub named_node: SyntaxNode,
    pub ns: NsSet,
}

impl ScopeEntry {
    pub fn from_named<Item>(item: Item, ns: NsSet) -> Option<Self>
    where
        Item: ast::HasName,
    {
        let name = item.name()?;
        Some(ScopeEntry {
            name: name.as_name(),
            named_node: item.syntax().clone(),
            ns,
        })
    }

    pub fn from_name_ref(name_ref: ast::NameRef, ns: NsSet) -> Self {
        ScopeEntry {
            name: name_ref.as_name(),
            named_node: name_ref.syntax().to_owned(),
            ns,
        }
    }

    pub fn copy_with_ns(&self, ns: NsSet) -> Self {
        let mut entry = self.clone();
        entry.ns = ns;
        entry
    }
}

pub trait ScopeEntryExt {
    fn to_entry(self) -> Option<ScopeEntry>;
}

impl<T: ast::HasName> ScopeEntryExt for T {
    fn to_entry(self) -> Option<ScopeEntry> {
        let name = self.name()?;
        let named_item = ast::AnyHasName::cast(self.syntax().to_owned())?;
        let entry = ScopeEntry {
            name: name.as_name(),
            named_node: self.syntax().to_owned(),
            ns: NsSet::from(named_item_ns(named_item)),
        };
        Some(entry)
    }
}

pub trait NamedItemsExt {
    fn to_entries(self) -> Vec<ScopeEntry>;
}

impl<T: ast::HasName> NamedItemsExt for Vec<T> {
    fn to_entries(self) -> Vec<ScopeEntry> {
        self.into_iter().filter_map(|item| item.to_entry()).collect()
    }
}

pub trait ScopeEntryListExt {
    fn filter_by_ns(self, ns: NsSet) -> impl Iterator<Item=ScopeEntry>;
    fn filter_by_name(self, name: &str) -> impl Iterator<Item=ScopeEntry>;
}

impl<T: Iterator<Item=ScopeEntry>> ScopeEntryListExt for T {
    fn filter_by_ns(self, ns: NsSet) -> impl Iterator<Item=ScopeEntry> {
        self.filter(move |entry| entry.ns.contains_any_of(ns))
    }

    fn filter_by_name(self, name: &str) -> impl Iterator<Item=ScopeEntry> {
        self.filter(move |entry| entry.name.as_str() == name)
    }
}
