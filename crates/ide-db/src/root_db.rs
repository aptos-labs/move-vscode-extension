use base_db::inputs::{
    FileIdSet, FilePackageRootInput, FileText, Files, InternFileId, InternedFileId, PackageDepsInput,
    PackageRootInput,
};
use base_db::package_root::{PackageRoot, PackageRootId};
use base_db::{ParseDatabase, SourceDatabase};
use line_index::LineIndex;
use salsa::Durability;
use std::fs::File;
use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::sync::Arc;
use std::{fmt, iter};
use vfs::FileId;

#[salsa::db]
pub struct RootDatabase {
    // We use `ManuallyDrop` here because every codegen unit that contains a
    // `&RootDatabase -> &dyn OtherDatabase` cast will instantiate its drop glue in the vtable,
    // which duplicates `Weak::drop` and `Arc::drop` tens of thousands of times, which makes
    // compile times of all `ide_*` and downstream crates suffer greatly.
    storage: ManuallyDrop<salsa::Storage<Self>>,
    files: Arc<Files>,
    builtins_file_id: Option<InternedFileId>,
}

impl std::panic::RefUnwindSafe for RootDatabase {}

#[salsa::db]
impl salsa::Database for RootDatabase {
    fn salsa_event(&self, _event: &dyn Fn() -> salsa::Event) {}
}

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

#[salsa::db]
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
    fn package_root(&self, source_root_id: PackageRootId) -> PackageRootInput {
        self.files.package_root(source_root_id)
    }

    fn set_package_root_with_durability(
        &mut self,
        source_root_id: PackageRootId,
        source_root: Arc<PackageRoot>,
        durability: Durability,
    ) {
        let files = Arc::clone(&self.files);
        files.set_package_root_with_durability(self, source_root_id, source_root, durability);
    }

    fn file_package_root(&self, id: FileId) -> FilePackageRootInput {
        self.files.file_package_root(id)
    }

    fn set_file_package_root_with_durability(
        &mut self,
        id: FileId,
        source_root_id: PackageRootId,
        durability: Durability,
    ) {
        let files = Arc::clone(&self.files);
        files.set_file_package_root_with_durability(self, id, source_root_id, durability);
    }

    fn builtins_file_id(&self) -> Option<InternedFileId> {
        self.builtins_file_id
    }

    fn set_builtins_file_id(&mut self, file_id: Option<FileId>) {
        self.builtins_file_id = file_id.map(|it| it.intern(self));
    }

    fn package_deps(&self, package_id: PackageRootId) -> PackageDepsInput {
        self.files.package_deps(package_id)
    }

    fn set_package_deps(&mut self, package_id: PackageRootId, deps: Vec<PackageRootId>) {
        let files = Arc::clone(&self.files);
        files.set_package_deps(self, package_id, Arc::from(deps))
    }

    fn spec_file_sets(&self, file_id: FileId) -> FileIdSet {
        self.files.spec_file_set(file_id)
    }

    fn set_spec_file_sets(&mut self, file_id: FileId, file_set: Vec<FileId>) {
        let files = Arc::clone(&self.files);
        files.set_spec_file_set(self, file_id, file_set)
    }

    fn source_file_ids(&self, package_root_id: PackageRootId) -> FileIdSet {
        let dep_ids = self.package_deps(package_root_id).data(self).deref().to_owned();
        tracing::debug!(?dep_ids);

        let file_sets = iter::once(package_root_id)
            .chain(dep_ids)
            .map(|id| self.package_root(id).data(self).file_set.clone())
            .collect::<Vec<_>>();

        let mut source_file_ids = vec![];
        for file_set in file_sets.clone() {
            for source_file_id in file_set.iter() {
                source_file_ids.push(source_file_id);
            }
        }
        FileIdSet::new(self, source_file_ids)
    }
}

impl Default for RootDatabase {
    fn default() -> RootDatabase {
        RootDatabase::new()
    }
}

impl RootDatabase {
    pub fn new() -> RootDatabase {
        let db = RootDatabase {
            storage: ManuallyDrop::new(salsa::Storage::default()),
            files: Default::default(),
            builtins_file_id: None,
        };

        // This needs to be here otherwise `CrateGraphBuilder` will panic.
        // db.set_all_crates(Arc::new(Box::new([])));
        // CrateGraphBuilder::default().set_in_db(&mut db);
        // db.set_local_roots_with_durability(Default::default(), Durability::MEDIUM);
        // db.set_library_roots_with_durability(Default::default(), Durability::MEDIUM);

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

#[query_group_macro::query_group]
pub trait LineIndexDatabase: ParseDatabase {
    fn line_index(&self, file_id: FileId) -> Arc<LineIndex>;
}

fn line_index(db: &dyn LineIndexDatabase, file_id: FileId) -> Arc<LineIndex> {
    let text = db.file_text(file_id).text(db);
    Arc::new(LineIndex::new(&text))
}
