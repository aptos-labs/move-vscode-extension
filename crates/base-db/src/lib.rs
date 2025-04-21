#![allow(dead_code)]

pub mod change;
pub mod package_root;

use crate::package_root::{PackageRoot, PackageRootId};
use std::sync::Arc;
use syntax::{Parse, SourceFile, SyntaxError};
use vfs::FileId;

pub trait Upcast<T: ?Sized> {
    fn upcast(&self) -> &T;
}

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
