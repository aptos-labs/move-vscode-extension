use crate::db::HirDatabase;
use crate::loc::{SyntaxLoc, SyntaxLocFileExt};
use crate::nameres::is_visible::is_visible_in_context;
use crate::nameres::namespaces::{Ns, NsSet, named_item_ns};
use crate::types::ty::Ty;
use std::fmt;
use std::fmt::Formatter;
use stdx::itertools::Itertools;
use syntax::ast;
use syntax::ast::{NamedItemScope, ReferenceElement};
use syntax::files::{InFile, InFileVecExt};
use vfs::FileId;

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct ScopeEntry {
    pub name: String,
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

    pub fn cast_into<T: ast::AstNode>(self, db: &dyn HirDatabase) -> Option<InFile<T>> {
        self.node_loc.to_ast(db.upcast())
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
            name: name.as_string(),
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
    fn filter_by_ns(self, ns: NsSet) -> Vec<ScopeEntry>;
    fn filter_by_name(self, name: String) -> Vec<ScopeEntry>;
    fn filter_by_visibility(
        self,
        db: &dyn HirDatabase,
        context: &InFile<impl ReferenceElement>,
    ) -> Vec<ScopeEntry>;
    fn filter_by_expected_type(self, db: &dyn HirDatabase, expected_type: Option<Ty>)
    -> Vec<ScopeEntry>;
}

impl ScopeEntryListExt for Vec<ScopeEntry> {
    fn filter_by_ns(self, ns: NsSet) -> Vec<ScopeEntry> {
        self.into_iter()
            .filter(move |entry| ns.contains(entry.ns))
            .collect()
    }

    fn filter_by_name(self, name: String) -> Vec<ScopeEntry> {
        self.into_iter().filter(|entry| entry.name == name).collect()
    }

    fn filter_by_visibility(
        self,
        db: &dyn HirDatabase,
        context: &InFile<impl ReferenceElement>,
    ) -> Vec<ScopeEntry> {
        self.into_iter()
            .filter(|entry| is_visible_in_context(db, entry, context))
            .collect()
    }

    fn filter_by_expected_type(
        self,
        db: &dyn HirDatabase,
        expected_type: Option<Ty>,
    ) -> Vec<ScopeEntry> {
        self.into_iter()
            .filter_map(|entry| {
                let item = entry.clone().cast_into::<ast::AnyNamedElement>(db)?;
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
}
