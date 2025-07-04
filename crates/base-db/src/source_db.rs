// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::change::ManifestFileId;
use crate::inputs::{
    FileIdInput, FileIdSet, FileText, PackageIdSet, PackageMetadata, PackageMetadataInput,
    PackageRootInput,
};
use crate::package_root::{PackageId, PackageRoot};
use salsa::Durability;
use std::cell::RefCell;
use std::panic;
use std::sync::{Arc, Once};
use syntax::{Parse, SyntaxError, ast};
use vfs::FileId;

#[salsa_macros::db]
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

    fn file_package_id(&self, id: FileId) -> PackageId;

    fn set_file_package_id(&mut self, file_id: FileId, package_id: PackageId);

    fn builtins_file_id(&self) -> Option<FileIdInput>;

    fn set_builtins_file_id(&mut self, id: Option<FileId>);

    fn package_metadata(&self, package_file_id: ManifestFileId) -> PackageMetadataInput;

    fn set_package_metadata(
        &mut self,
        package_file_id: ManifestFileId,
        package_metadata: PackageMetadata,
    );

    fn spec_related_files(&self, file_id: FileId) -> FileIdSet;

    fn set_spec_related_files(&mut self, file_id: FileId, file_set: Vec<FileId>);

    fn all_package_ids(&self) -> PackageIdSet;
}

/// Parses the file into the syntax tree.
#[salsa::tracked]
pub fn parse(db: &dyn SourceDatabase, file_id: FileIdInput) -> Parse {
    let _p = tracing::info_span!("source_db::parse", ?file_id).entered();
    let text = db.file_text(file_id.data(db)).text(db);
    ast::SourceFile::parse(&text)
}

#[salsa::tracked(returns(ref))]
pub fn parse_errors(db: &dyn SourceDatabase, file_id: FileIdInput) -> Option<Box<[SyntaxError]>> {
    let errors = parse(db, file_id).errors();
    match &*errors {
        [] => None,
        [..] => Some(errors.into()),
    }
}

#[salsa_macros::tracked]
pub fn metadata_for_package_id(
    db: &dyn SourceDatabase,
    package_id: PackageId,
) -> Option<PackageMetadata> {
    let manifest_file_id = db.package_root(package_id).data(db).manifest_file_id?;
    let metadata = db.package_metadata(manifest_file_id).metadata(db);
    Some(metadata)
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
