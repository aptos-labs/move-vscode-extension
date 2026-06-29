// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use base_db::SourceDatabase;
use base_db::change::ManifestFileId;
use base_db::inputs::{
    FileIdInput, FileIdSet, FileText, Files, InternFileId, PackageIdSet, PackageMetadata,
    PackageMetadataInput, PackageRootInput,
};
use base_db::package_root::{PackageId, PackageRoot};
use line_index::LineIndex;
use salsa::Durability;
use std::fmt;
use std::mem::ManuallyDrop;
use std::sync::Arc;
use vfs::FileId;

#[salsa_macros::db]
pub struct RootDatabase {
    // We use `ManuallyDrop` here because every codegen unit that contains a
    // `&RootDatabase -> &dyn OtherDatabase` cast will instantiate its drop glue in the vtable,
    // which duplicates `Weak::drop` and `Arc::drop` tens of thousands of times, which makes
    // compile times of all `ide_*` and downstream crates suffer greatly.
    storage: ManuallyDrop<salsa::Storage<Self>>,
    files: Arc<Files>,
    builtins_file_id: Option<FileIdInput>,
}

impl std::panic::RefUnwindSafe for RootDatabase {}

#[salsa_macros::db]
impl salsa::Database for RootDatabase {}

impl Drop for RootDatabase {
    fn drop(&mut self) {
        unsafe { ManuallyDrop::drop(&mut self.storage) };
    }
}

impl Clone for RootDatabase {
    fn clone(&self) -> Self {
        Self {
            storage: self.storage.clone(),
            files: self.files.clone(),
            builtins_file_id: self.builtins_file_id.clone(),
        }
    }
}

impl fmt::Debug for RootDatabase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RootDatabase").finish()
    }
}

#[salsa_macros::db]
impl SourceDatabase for RootDatabase {
    fn file_text(&self, file_id: FileId) -> FileText {
        self.files.file_text(file_id)
    }

    fn set_file_text(&mut self, file_id: FileId, text: &str) {
        let files = Arc::clone(&self.files);
        files.set_file_text_with_durability(self, file_id, text, Durability::LOW);
    }

    fn set_file_text_with_durability(&mut self, file_id: FileId, text: &str, durability: Durability) {
        let files = Arc::clone(&self.files);
        files.set_file_text_with_durability(self, file_id, text, durability);
    }

    /// Source root of the file.
    fn package_root(&self, package_id: PackageId) -> PackageRootInput {
        self.files.package_root(package_id)
    }

    fn replace_package_roots(&mut self, package_roots: Vec<PackageRoot>) {
        let files = Arc::clone(&self.files);
        files.replace_package_roots(self, package_roots);
    }

    fn file_package_id(&self, id: FileId) -> PackageId {
        self.files.file_package_id(id)
    }

    fn builtins_file_id(&self) -> Option<FileIdInput> {
        self.builtins_file_id
    }

    fn set_builtins_file_id(&mut self, file_id: Option<FileId>) {
        self.builtins_file_id = file_id.map(|it| it.intern(self));
    }

    fn package_metadata(&self, manifest_file_id: ManifestFileId) -> PackageMetadataInput {
        self.files.package_metadata(manifest_file_id)
    }

    fn set_package_metadata(
        &mut self,
        package_manifest_id: ManifestFileId,
        package_metadata: PackageMetadata,
    ) {
        let files = Arc::clone(&self.files);
        files.set_package_metadata(self, package_manifest_id, package_metadata)
    }

    fn spec_related_files(&self, file_id: FileId) -> FileIdSet {
        self.files.spec_related_files(file_id)
    }

    fn set_spec_related_files(&mut self, file_id: FileId, file_set: Vec<FileId>) {
        let files = Arc::clone(&self.files);
        files.set_spec_related_files(self, file_id, file_set)
    }

    fn all_package_ids(&self) -> PackageIdSet {
        self.files.package_ids()
    }
}

impl Default for RootDatabase {
    fn default() -> RootDatabase {
        RootDatabase::new()
    }
}

impl RootDatabase {
    pub fn new() -> RootDatabase {
        let mut db = RootDatabase {
            storage: ManuallyDrop::new(salsa::Storage::default()),
            files: Default::default(),
            builtins_file_id: None,
        };

        let package_ids = PackageIdSet::builder(vec![])
            .durability(Durability::MEDIUM)
            .new(&mut db);
        Arc::get_mut(&mut db.files)
            .expect("files should not be shared yet")
            .all_package_ids = Some(package_ids);

        db
    }

    pub fn snapshot(&self) -> Self {
        Self {
            storage: self.storage.clone(),
            files: self.files.clone(),
            builtins_file_id: self.builtins_file_id,
        }
    }
}

pub fn line_index(db: &dyn SourceDatabase, file_id: FileId) -> Arc<LineIndex> {
    #[salsa_macros::tracked]
    fn line_index(db: &dyn SourceDatabase, file_id: FileIdInput) -> Arc<LineIndex> {
        let text = db.file_text(file_id.data(db)).text(db);
        Arc::new(LineIndex::new(&text))
    }
    line_index(db, FileIdInput::new(db, file_id))
}
