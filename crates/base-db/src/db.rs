use crate::inputs::{
    DepPackagesInput, FileIdSet, FilePackageIdInput, FileText, InternedFileId, PackageRootInput,
};
use crate::package_root::{PackageId, PackageRoot};
use salsa::Durability;
use std::cell::RefCell;
use std::panic;
use std::sync::{Arc, Once};
use syntax::{Parse, SyntaxError, ast};
use vfs::FileId;

#[salsa::db]
pub trait SourceDatabase: salsa::Database {
    /// Text of the file.
    fn file_text(&self, file_id: FileId) -> FileText;

    fn set_file_text(&mut self, file_id: FileId, text: &str);

    fn set_file_text_with_durability(&mut self, file_id: FileId, text: &str, durability: Durability);

    /// Contents of the source root.
    fn package_root(&self, package_id: PackageId) -> PackageRootInput;

    /// Source root of the file.
    fn set_package_root_with_durability(
        &mut self,
        package_id: PackageId,
        package_root: Arc<PackageRoot>,
        durability: Durability,
    );

    fn file_package_id(&self, id: FileId) -> FilePackageIdInput;

    fn set_file_package_id_with_durability(
        &mut self,
        file_id: FileId,
        package_id: PackageId,
        durability: Durability,
    );

    fn builtins_file_id(&self) -> Option<InternedFileId>;

    fn set_builtins_file_id(&mut self, id: Option<FileId>);

    fn dep_package_ids(&self, package_id: PackageId) -> DepPackagesInput;

    fn set_dep_package_ids(&mut self, package_id: PackageId, dep_ids: Vec<PackageId>);

    fn spec_related_files(&self, file_id: FileId) -> FileIdSet;

    fn set_spec_related_files(&mut self, file_id: FileId, file_set: Vec<FileId>);

    fn all_source_file_ids(&self, package_id: PackageId) -> FileIdSet;
}

#[query_group_macro::query_group]
pub trait ParseDatabase: SourceDatabase + salsa::Database {
    /// Parses the file into the syntax tree.
    #[salsa::invoke_actual(parse)]
    #[salsa::lru(128)]
    fn parse(&self, file_id: InternedFileId) -> Parse;

    /// Returns the set of errors obtained from parsing the file including validation errors.
    #[salsa::transparent]
    fn parse_errors(&self, file_id: InternedFileId) -> Option<&[SyntaxError]>;
}

fn parse(db: &dyn ParseDatabase, file_id: InternedFileId) -> Parse {
    let _p = tracing::info_span!("parse", ?file_id).entered();
    let text = db.file_text(file_id.data(db)).text(db);
    ast::SourceFile::parse(&text)
}

fn parse_errors(db: &dyn ParseDatabase, file_id: InternedFileId) -> Option<&[SyntaxError]> {
    #[salsa::tracked(return_ref)]
    fn parse_errors(db: &dyn ParseDatabase, file_id: InternedFileId) -> Option<Box<[SyntaxError]>> {
        let errors = db.parse(file_id).errors();
        match &*errors {
            [] => None,
            [..] => Some(errors.into()),
        }
    }
    parse_errors(db, file_id).as_ref().map(|it| &**it)
}

#[must_use]
#[non_exhaustive]
pub struct DbPanicContext;

impl Drop for DbPanicContext {
    fn drop(&mut self) {
        Self::with_ctx(|ctx| assert!(ctx.pop().is_some()));
    }
}

impl DbPanicContext {
    pub fn enter(frame: String) -> DbPanicContext {
        #[expect(clippy::print_stderr, reason = "already panicking anyway")]
        fn set_hook() {
            let default_hook = panic::take_hook();
            panic::set_hook(Box::new(move |panic_info| {
                default_hook(panic_info);
                if let Some(backtrace) = salsa::Backtrace::capture() {
                    eprintln!("{backtrace:#}");
                }
                DbPanicContext::with_ctx(|ctx| {
                    if !ctx.is_empty() {
                        eprintln!("additional context:");
                        for (idx, frame) in ctx.iter().enumerate() {
                            eprintln!("{idx:>4}: {frame}\n");
                        }
                    }
                });
            }));
        }

        static SET_HOOK: Once = Once::new();
        SET_HOOK.call_once(set_hook);

        Self::with_ctx(|ctx| ctx.push(frame));
        DbPanicContext
    }

    fn with_ctx(f: impl FnOnce(&mut Vec<String>)) {
        thread_local! {
            static CTX: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
        }
        CTX.with(|ctx| f(&mut ctx.borrow_mut()));
    }
}
