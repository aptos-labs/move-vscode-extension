use crate::new_db::SourceDatabase2;
use crate::package_root::{PackageRoot, PackageRootId};
use dashmap::{DashMap, Entry};
use salsa::Durability;
use salsa::Setter;
use std::sync::Arc;
use vfs::FileId;

#[salsa::interned(no_lifetime)]
pub struct InternedFileId {
    pub data: FileId,
}

pub trait InternFileId {
    fn intern(self, db: &dyn SourceDatabase2) -> InternedFileId;
}

impl InternFileId for FileId {
    fn intern(self, db: &dyn SourceDatabase2) -> InternedFileId {
        InternedFileId::new(db, self)
    }
}

#[salsa::input]
pub struct FileIdSet {
    pub data: Vec<FileId>,
}

#[salsa::input]
pub struct FileText {
    pub text: Arc<str>,
    pub file_id: FileId,
}

#[salsa::input]
pub struct FilePackageRootInput {
    pub data: PackageRootId,
}

#[salsa::input]
pub struct PackageRootInput {
    pub data: Arc<PackageRoot>,
}

#[salsa::input]
pub struct PackageDepsInput {
    pub data: Arc<Vec<PackageRootId>>,
}

#[derive(Debug, Default)]
pub struct Files {
    files: Arc<DashMap<FileId, FileText>>,
    source_roots: Arc<DashMap<PackageRootId, PackageRootInput>>,
    file_source_roots: Arc<DashMap<FileId, FilePackageRootInput>>,
    package_deps: Arc<DashMap<PackageRootId, PackageDepsInput>>,
}

impl Files {
    pub fn file_text(&self, file_id: FileId) -> FileText {
        *self
            .files
            .get(&file_id)
            .expect("Unable to fetch file; this is a bug")
    }

    pub fn set_file_text_with_durability(
        &self,
        db: &mut dyn SourceDatabase2,
        file_id: FileId,
        text: &str,
        durability: Durability,
    ) {
        match self.files.entry(file_id) {
            Entry::Occupied(mut occupied) => {
                occupied
                    .get_mut()
                    .set_text(db)
                    .with_durability(durability)
                    .to(Arc::from(text));
            }
            Entry::Vacant(vacant) => {
                let text = FileText::builder(Arc::from(text), file_id)
                    .durability(durability)
                    .new(db);
                vacant.insert(text);
            }
        };
    }

    /// Source root of the file.
    pub fn package_root(&self, package_root_id: PackageRootId) -> PackageRootInput {
        let package_root = self
            .source_roots
            .get(&package_root_id)
            .expect("Unable to fetch source root id; this is a bug");

        *package_root
    }

    pub fn set_package_root_with_durability(
        &self,
        db: &mut dyn SourceDatabase2,
        package_root_id: PackageRootId,
        package_root: Arc<PackageRoot>,
        durability: Durability,
    ) {
        match self.source_roots.entry(package_root_id) {
            Entry::Occupied(mut occupied) => {
                occupied
                    .get_mut()
                    .set_data(db)
                    .with_durability(durability)
                    .to(package_root);
            }
            Entry::Vacant(vacant) => {
                let source_root = PackageRootInput::builder(package_root)
                    .durability(durability)
                    .new(db);
                vacant.insert(source_root);
            }
        };
    }

    pub fn file_package_root(&self, id: FileId) -> FilePackageRootInput {
        let file_source_root = self
            .file_source_roots
            .get(&id)
            .expect("Unable to fetch FileSourceRootInput; this is a bug");
        *file_source_root
    }

    pub fn set_file_package_root_with_durability(
        &self,
        db: &mut dyn SourceDatabase2,
        id: FileId,
        package_root_id: PackageRootId,
        durability: Durability,
    ) {
        match self.file_source_roots.entry(id) {
            Entry::Occupied(mut occupied) => {
                occupied
                    .get_mut()
                    .set_data(db)
                    .with_durability(durability)
                    .to(package_root_id);
            }
            Entry::Vacant(vacant) => {
                let file_package_root = FilePackageRootInput::builder(package_root_id)
                    .durability(durability)
                    .new(db);
                vacant.insert(file_package_root);
            }
        };
    }

    pub fn package_deps(&self, package_id: PackageRootId) -> PackageDepsInput {
        let package_deps = self
            .package_deps
            .get(&package_id)
            .expect("Unable to fetch package dependencies");
        *package_deps
    }

    pub fn set_package_deps(
        &self,
        db: &mut dyn SourceDatabase2,
        package_id: PackageRootId,
        deps: Arc<Vec<PackageRootId>>,
    ) {
        match self.package_deps.entry(package_id) {
            Entry::Occupied(mut occupied) => {
                occupied.get_mut().set_data(db).to(deps);
            }
            Entry::Vacant(vacant) => {
                let deps = PackageDepsInput::builder(deps).new(db);
                vacant.insert(deps);
            }
        };
    }
}
