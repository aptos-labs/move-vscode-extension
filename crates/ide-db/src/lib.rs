#![allow(dead_code)]

pub mod apply_change;
mod assists;
pub mod defs;
pub mod helpers;
mod label;
mod source_change;
mod syntax_helpers;
pub mod text_edit;

use base_db::input::CrateId;
use base_db::{FileLoader, FileLoaderDelegate, SourceDatabase, SourceRootDatabase, Upcast};
use lang::db::HirDatabase;
use line_index::LineIndex;
use std::fmt;
use std::mem::ManuallyDrop;
use syntax::{SyntaxKind, SyntaxKind::*};
use triomphe::Arc;
use vfs::{AnchoredPath, FileId};

pub type FxIndexSet<T> = indexmap::IndexSet<T, std::hash::BuildHasherDefault<rustc_hash::FxHasher>>;
pub type FxIndexMap<K, V> =
    indexmap::IndexMap<K, V, std::hash::BuildHasherDefault<rustc_hash::FxHasher>>;

#[ra_salsa::database(
    base_db::SourceDatabaseStorage,
    base_db::SourceRootDatabaseStorage,
    lang::db::HirDatabaseStorage,
    LineIndexDatabaseStorage
)]
pub struct RootDatabase {
    // We use `ManuallyDrop` here because every codegen unit that contains a
    // `&RootDatabase -> &dyn OtherDatabase` cast will instantiate its drop glue in the vtable,
    // which duplicates `Weak::drop` and `Arc::drop` tens of thousands of times, which makes
    // compile times of all `ide_*` and downstream crates suffer greatly.
    storage: ManuallyDrop<ra_salsa::Storage<RootDatabase>>,
}

impl Drop for RootDatabase {
    fn drop(&mut self) {
        unsafe { ManuallyDrop::drop(&mut self.storage) };
    }
}

impl fmt::Debug for RootDatabase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RootDatabase").finish()
    }
}

impl Upcast<dyn HirDatabase> for RootDatabase {
    #[inline]
    fn upcast(&self) -> &(dyn HirDatabase + 'static) {
        self
    }
}

impl Upcast<dyn SourceDatabase> for RootDatabase {
    #[inline]
    fn upcast(&self) -> &(dyn SourceDatabase + 'static) {
        self
    }
}

impl Upcast<dyn SourceRootDatabase> for RootDatabase {
    #[inline]
    fn upcast(&self) -> &(dyn SourceRootDatabase + 'static) {
        self
    }
}

impl FileLoader for RootDatabase {
    fn resolve_path(&self, path: AnchoredPath<'_>) -> Option<FileId> {
        FileLoaderDelegate(self).resolve_path(path)
    }
    fn relevant_crates(&self, file_id: FileId) -> Arc<[CrateId]> {
        FileLoaderDelegate(self).relevant_crates(file_id)
    }
}

impl ra_salsa::Database for RootDatabase {}

impl Default for RootDatabase {
    fn default() -> RootDatabase {
        RootDatabase::new(/*None*/)
    }
}

impl RootDatabase {
    pub fn new() -> RootDatabase {
        let db = RootDatabase {
            storage: ManuallyDrop::new(ra_salsa::Storage::default()),
        };
        // db.set_crate_graph_with_durability(Default::default(), Durability::HIGH);
        // db.set_local_roots_with_durability(Default::default(), Durability::HIGH);
        // db.set_library_roots_with_durability(Default::default(), Durability::HIGH);
        // db.setup_syntax_context_root();
        db
    }
}

impl ra_salsa::ParallelDatabase for RootDatabase {
    fn snapshot(&self) -> ra_salsa::Snapshot<RootDatabase> {
        ra_salsa::Snapshot::new(RootDatabase {
            storage: ManuallyDrop::new(self.storage.snapshot()),
        })
    }
}

#[ra_salsa::query_group(LineIndexDatabaseStorage)]
pub trait LineIndexDatabase: SourceDatabase {
    fn line_index(&self, file_id: FileId) -> Arc<LineIndex>;
}

fn line_index(db: &dyn LineIndexDatabase, file_id: FileId) -> Arc<LineIndex> {
    let text = db.file_text(file_id);
    Arc::new(LineIndex::new(&text))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SymbolKind {
    Attribute,
    Module,
    Const,
    Function,
    Method,
    Struct,
    Field,
    Enum,
    EnumVariant,
    TypeParam,
    ValueParam,
    Label,
    Local,
}

pub fn ast_kind_to_symbol_kind(kind: SyntaxKind) -> Option<SymbolKind> {
    match kind {
        MODULE => Some(SymbolKind::Module),
        FUN => Some(SymbolKind::Function),
        CONST => Some(SymbolKind::Const),
        STRUCT => Some(SymbolKind::Struct),
        ENUM => Some(SymbolKind::Enum),

        TYPE_PARAM => Some(SymbolKind::TypeParam),
        IDENT_PAT => Some(SymbolKind::Local),
        VARIANT => Some(SymbolKind::EnumVariant),

        NAMED_FIELD => Some(SymbolKind::Field),

        _ => unreachable!("unhandled ast kind {:?}", kind),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SnippetCap {
    _private: (),
}

impl SnippetCap {
    pub const fn new(allow_snippets: bool) -> Option<SnippetCap> {
        if allow_snippets {
            Some(SnippetCap { _private: () })
        } else {
            None
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Severity {
    Error,
    Warning,
    WeakWarning,
    Allow,
}
