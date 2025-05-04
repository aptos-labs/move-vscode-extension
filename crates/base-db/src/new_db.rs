use crate::inputs::{
    FileIdSet, FilePackageRootInput, FileText, InternedFileId, PackageDepsInput, PackageRootInput,
};
use crate::package_root::{PackageRoot, PackageRootId};
use salsa::Durability;
use std::sync::Arc;
use syntax::{ast, Parse, SyntaxError};
use vfs::FileId;

#[salsa::db]
pub trait SourceDatabase2: salsa::Database {
    /// Text of the file.
    fn file_text(&self, file_id: FileId) -> FileText;

    fn set_file_text(&mut self, file_id: FileId, text: &str);

    fn set_file_text_with_durability(&mut self, file_id: FileId, text: &str, durability: Durability);

    /// Contents of the source root.
    fn package_root(&self, id: PackageRootId) -> PackageRootInput;

    /// Source root of the file.
    fn set_package_root_with_durability(
        &mut self,
        source_root_id: PackageRootId,
        source_root: Arc<PackageRoot>,
        durability: Durability,
    );

    fn file_package_root(&self, id: FileId) -> FilePackageRootInput;

    fn set_file_package_root_with_durability(
        &mut self,
        id: FileId,
        source_root_id: PackageRootId,
        durability: Durability,
    );

    fn builtins_file_id(&self) -> Option<InternedFileId>;

    fn set_builtins_file_id(&mut self, id: Option<FileId>);

    fn package_deps(&self, package_id: PackageRootId) -> PackageDepsInput;

    fn set_package_deps(&mut self, package_id: PackageRootId, deps: Vec<PackageRootId>);

    fn source_file_ids(&self, package_root_id: PackageRootId) -> FileIdSet;
}

#[query_group_macro::query_group]
pub trait ParseDatabase2: SourceDatabase2 + salsa::Database {
    /// Parses the file into the syntax tree.
    #[salsa::invoke_actual(parse)]
    #[salsa::lru(128)]
    fn parse(&self, file_id: InternedFileId) -> Parse;

    /// Returns the set of errors obtained from parsing the file including validation errors.
    #[salsa::transparent]
    fn parse_errors(&self, file_id: InternedFileId) -> Option<&[SyntaxError]>;
}

fn parse(db: &dyn ParseDatabase2, file_id: InternedFileId) -> Parse {
    let _p = tracing::info_span!("parse", ?file_id).entered();
    let file_id = file_id.data(db);
    let text = db.file_text(file_id).text(db);
    ast::SourceFile::parse(&text)
}

fn parse_errors(db: &dyn ParseDatabase2, file_id: InternedFileId) -> Option<&[SyntaxError]> {
    #[salsa::tracked(return_ref)]
    fn parse_errors(db: &dyn ParseDatabase2, file_id: InternedFileId) -> Option<Box<[SyntaxError]>> {
        let errors = db.parse(file_id).errors();
        match &*errors {
            [] => None,
            [..] => Some(errors.into()),
        }
    }
    parse_errors(db, file_id).as_ref().map(|it| &**it)
}
