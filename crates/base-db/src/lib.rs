#![allow(dead_code)]

pub mod change;
pub mod package_root;
pub mod inputs;
pub mod new_db;

pub use crate::new_db::SourceDatabase2;
pub use crate::new_db::ParseDatabase2;

use crate::package_root::{PackageRoot, PackageRootId};
use std::iter;
use std::ops::Deref;
use std::sync::Arc;
use syntax::{Parse, SourceFile, SyntaxError};
use vfs::FileId;

/// Database which stores all significant input facts: source code and project
/// model. Everything else in rust-analyzer is derived from these queries.
#[ra_salsa::query_group(SourceDatabaseStorage)]
pub trait SourceDatabase: std::fmt::Debug + std::panic::RefUnwindSafe {
    #[ra_salsa::input]
    fn file_text(&self, file_id: FileId) -> Arc<str>;

    /// Parses the file into the syntax tree.
    fn parse(&self, file_id: FileId) -> Parse;

    /// Returns the set of errors obtained from parsing the file including validation errors.
    fn parse_errors(&self, file_id: FileId) -> Option<Arc<[SyntaxError]>>;

    #[ra_salsa::input]
    fn file_package_root(&self, file_id: FileId) -> PackageRootId;

    #[ra_salsa::input]
    fn builtins_file_id(&self) -> Option<FileId>;

    #[ra_salsa::input]
    fn package_root(&self, id: PackageRootId) -> Arc<PackageRoot>;

    #[ra_salsa::input]
    fn package_deps(&self, manifest_file_id: PackageRootId) -> Arc<Vec<PackageRootId>>;

    fn source_file_ids(&self, package_root_id: PackageRootId) -> Vec<FileId>;
}

fn parse(db: &dyn SourceDatabase, file_id: FileId) -> Parse {
    let _p = tracing::debug_span!("parse", ?file_id).entered();
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

fn source_file_ids(db: &dyn SourceDatabase, package_root_id: PackageRootId) -> Vec<FileId> {
    let dep_ids = db.package_deps(package_root_id).deref().to_owned();
    tracing::debug!(?dep_ids);

    let file_sets = iter::once(package_root_id)
        .chain(dep_ids)
        .map(|id| db.package_root(id).file_set.clone())
        .collect::<Vec<_>>();

    let mut source_file_ids = vec![];
    for file_set in file_sets.clone() {
        for source_file_id in file_set.iter() {
            source_file_ids.push(source_file_id);
        }
    }
    source_file_ids
}
