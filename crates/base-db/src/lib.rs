#![allow(dead_code)]

pub mod change;
pub mod input;

use crate::input::{CrateGraph, CrateId, SourceRoot, SourceRootId};
use syntax::{Parse, SourceFile, SyntaxError};
use triomphe::Arc;
use vfs::{AnchoredPath, FileId};

pub trait Upcast<T: ?Sized> {
    fn upcast(&self) -> &T;
}

pub const DEFAULT_FILE_TEXT_LRU_CAP: u16 = 16;
pub const DEFAULT_PARSE_LRU_CAP: u16 = 128;
pub const DEFAULT_BORROWCK_LRU_CAP: u16 = 2024;

pub trait FileLoader {
    fn resolve_path(&self, path: AnchoredPath<'_>) -> Option<FileId>;
    /// Crates whose root's source root is the same as the source root of `file_id`
    fn relevant_crates(&self, file_id: FileId) -> Arc<[CrateId]>;
}

/// Database which stores all significant input facts: source code and project
/// model. Everything else in rust-analyzer is derived from these queries.
#[ra_salsa::query_group(SourceDatabaseStorage)]
pub trait SourceDatabase: FileLoader + std::fmt::Debug {
    #[ra_salsa::input]
    fn file_text(&self, file_id: FileId) -> Arc<str>;

    /// Parses the file into the syntax tree.
    fn parse(&self, file_id: FileId) -> Parse;

    /// Returns the set of errors obtained from parsing the file including validation errors.
    fn parse_errors(&self, file_id: FileId) -> Option<Arc<[SyntaxError]>>;

    #[ra_salsa::input]
    fn crate_graph(&self) -> Arc<CrateGraph>;

    // #[ra_salsa::input]
    // fn crate_workspace_data(&self) -> Arc<FxHashMap<CrateId, Arc<CrateWorkspaceData>>>;
}

fn parse(db: &dyn SourceDatabase, file_id: FileId) -> Parse {
    let _p = tracing::info_span!("parse", ?file_id).entered();
    let text = db.file_text(file_id);
    SourceFile::parse(&text)
}

fn parse_errors(db: &dyn SourceDatabase, file_id: FileId) -> Option<Arc<[SyntaxError]>> {
    let errors = db.parse(file_id).errors();
    match &*errors {
        [] => None,
        [..] => Some(errors.into()),
    }
}

/// We don't want to give HIR knowledge of source roots, hence we extract these
/// methods into a separate DB.
#[ra_salsa::query_group(SourceRootDatabaseStorage)]
pub trait SourceRootDatabase: SourceDatabase + Upcast<dyn SourceDatabase> {
    /// Path to a file, relative to the root of its source root.
    /// Source root of the file.
    #[ra_salsa::input]
    fn file_source_root(&self, file_id: FileId) -> SourceRootId;

    /// Contents of the source root.
    #[ra_salsa::input]
    fn source_root(&self, id: SourceRootId) -> Arc<SourceRoot>;

    /// Crates whose root file is in `id`.
    fn source_root_crates(&self, id: SourceRootId) -> Arc<[CrateId]>;
}

fn source_root_crates(db: &dyn SourceRootDatabase, id: SourceRootId) -> Arc<[CrateId]> {
    let graph = db.crate_graph();
    let mut crates = graph
        .iter()
        .filter(|&crate_id| {
            let root_file = graph[crate_id].root_file_id;
            db.file_source_root(root_file) == id
        })
        .collect::<Vec<_>>();
    crates.sort();
    crates.dedup();
    crates.into_iter().collect()
}

// FIXME: Would be nice to get rid of this somehow
/// Silly workaround for cyclic deps due to the SourceRootDatabase and SourceDatabase split
/// regarding FileLoader
pub struct FileLoaderDelegate<T>(pub T);

impl<T: SourceRootDatabase> FileLoader for FileLoaderDelegate<&'_ T> {
    fn resolve_path(&self, path: AnchoredPath<'_>) -> Option<FileId> {
        // FIXME: this *somehow* should be platform agnostic...
        let source_root = self.0.file_source_root(path.anchor);
        let source_root = self.0.source_root(source_root);
        source_root.resolve_path(path)
    }

    fn relevant_crates(&self, file_id: FileId) -> Arc<[CrateId]> {
        let _p = tracing::info_span!("relevant_crates").entered();
        let source_root = self.0.file_source_root(file_id);
        self.0.source_root_crates(source_root)
    }
}
