use crate::db::HirDatabase;
use crate::files::InFileVecExt;
use crate::loc::{SyntaxLoc, SyntaxLocExt};
use crate::nameres::is_visible::is_visible_in_context;
use crate::nameres::namespaces::{named_item_ns, Ns, NsSet};
use crate::{AsName, InFile, Name};
use std::fmt;
use std::fmt::Formatter;
use syntax::ast;
use syntax::ast::{NamedItemScope, Reference};
use vfs::FileId;

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct ScopeEntry {
    pub name: Name,
    pub node_loc: SyntaxLoc,
    pub ns: Ns,
    pub scope_adjustment: Option<NamedItemScope>,
}

impl ScopeEntry {
    pub fn copy_with_ns(&self, ns: Ns) -> Self {
        let mut entry = self.clone();
        entry.ns = ns;
        entry
    }
}

impl fmt::Debug for ScopeEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ScopeEntry")
            .field(&self.name.to_string())
            .field(&self.ns)
            .field(&self.node_loc)
            .finish()
    }
}

pub trait ScopeEntryExt {
    fn to_entry(self) -> Option<ScopeEntry>;
}

impl<T: ast::NamedElement> ScopeEntryExt for InFile<T> {
    fn to_entry(self) -> Option<ScopeEntry> {
        let name = self.value.name()?;
        let item_loc = self.loc();
        let item_ns = named_item_ns(item_loc.kind());
        let entry = ScopeEntry {
            name: name.as_name(),
            node_loc: item_loc,
            ns: item_ns,
            scope_adjustment: None,
        };
        Some(entry)
    }
}

pub trait NamedItemsExt {
    fn to_entries(self) -> Vec<ScopeEntry>;
}

impl<T: ast::NamedElement> NamedItemsExt for Vec<InFile<T>> {
    fn to_entries(self) -> Vec<ScopeEntry> {
        self.into_iter().filter_map(|item| item.to_entry()).collect()
    }
}

pub trait NamedItemsInFileExt {
    fn to_in_file_entries(self, file_id: FileId) -> Vec<ScopeEntry>;
}

impl<T: ast::NamedElement> NamedItemsInFileExt for Vec<T> {
    fn to_in_file_entries(self, file_id: FileId) -> Vec<ScopeEntry> {
        self.wrapped_in_file(file_id).to_entries()
    }
}

pub trait ScopeEntryListExt {
    fn filter_by_ns(self, ns: NsSet) -> Vec<ScopeEntry>;
    fn filter_by_name(self, name: Name) -> Vec<ScopeEntry>;
    fn filter_by_visibility(
        self,
        db: &dyn HirDatabase,
        context: InFile<impl Reference>,
    ) -> Vec<ScopeEntry>;
}

impl ScopeEntryListExt for Vec<ScopeEntry> {
    fn filter_by_ns(self, ns: NsSet) -> Vec<ScopeEntry> {
        self.into_iter()
            .filter(move |entry| ns.contains(entry.ns))
            .collect()
    }

    fn filter_by_name(self, name: Name) -> Vec<ScopeEntry> {
        self.into_iter().filter(move |entry| entry.name == name).collect()
    }

    fn filter_by_visibility(
        self,
        db: &dyn HirDatabase,
        context: InFile<impl Reference>,
    ) -> Vec<ScopeEntry> {
        self.into_iter()
            .filter(move |entry| is_visible_in_context(db, entry, &context))
            .collect()
    }
}
