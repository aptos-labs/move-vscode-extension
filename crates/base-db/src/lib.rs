#![allow(dead_code)]

pub mod change;
pub mod package_root;

use crate::package_root::{PackageRoot, PackageRootId};
use syntax::{Parse, SourceFile, SyntaxError};
use triomphe::Arc;
use vfs::FileId;

pub trait Upcast<T: ?Sized> {
    fn upcast(&self) -> &T;
}

/// Database which stores all significant input facts: source code and project
/// model. Everything else in rust-analyzer is derived from these queries.
#[ra_salsa::query_group(SourceDatabaseStorage)]
pub trait SourceDatabase: std::fmt::Debug {
    #[ra_salsa::input]
    fn file_text(&self, file_id: FileId) -> Arc<str>;

    /// Parses the file into the syntax tree.
    fn parse(&self, file_id: FileId) -> Parse;

    /// Returns the set of errors obtained from parsing the file including validation errors.
    fn parse_errors(&self, file_id: FileId) -> Option<Arc<[SyntaxError]>>;
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
#[ra_salsa::query_group(PackageRootDatabaseStorage)]
pub trait PackageRootDatabase: SourceDatabase + Upcast<dyn SourceDatabase> {
    // /// Path to a file, relative to the root of its source root.
    // /// Source root of the file.
    // #[ra_salsa::input]
    // fn file_source_root(&self, file_id: FileId) -> SourceRootId;

    #[ra_salsa::input]
    fn file_package_root_id(&self, file_id: FileId) -> PackageRootId;

    #[ra_salsa::input]
    fn builtins_file_id(&self) -> FileId;

    #[ra_salsa::input]
    fn package_root(&self, id: PackageRootId) -> Arc<PackageRoot>;

    #[ra_salsa::input]
    fn package_deps(&self, manifest_file_id: PackageRootId) -> Arc<Vec<PackageRootId>>;
}
