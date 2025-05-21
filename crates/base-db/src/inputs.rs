use crate::package_root::{PackageId, PackageRoot};
use crate::source_db::SourceDatabase;
use dashmap::{DashMap, Entry};
use salsa::Durability;
use salsa::Setter;
use std::sync::Arc;
use vfs::FileId;

#[salsa::interned(no_lifetime)]
#[derive(Debug)]
pub struct FileIdInput {
    pub data: FileId,
}

pub trait InternFileId {
    fn intern(self, db: &dyn SourceDatabase) -> FileIdInput;
}

impl InternFileId for FileId {
    fn intern(self, db: &dyn SourceDatabase) -> FileIdInput {
        FileIdInput::new(db, self)
    }
}

#[salsa::input]
pub struct FileIdSet {
    pub data: Vec<FileId>,
}

#[salsa::input]
pub struct PackageIdSet {
    pub data: Vec<PackageId>,
}

#[salsa::input]
pub struct FileText {
    pub text: Arc<str>,
    pub file_id: FileId,
}

#[salsa::input]
pub struct PackageRootInput {
    pub data: Arc<PackageRoot>,
}

#[salsa::input]
pub struct PackageData {
    // todo: add package name
    pub deps: Arc<Vec<PackageId>>,
}

#[derive(Default)]
pub struct Files {
    files: Arc<DashMap<FileId, FileText>>,
    file_package_ids: Arc<DashMap<FileId, PackageId>>,

    package_roots: Arc<DashMap<PackageId, PackageRootInput>>,
    package_deps: Arc<DashMap<PackageId, PackageData>>,

    spec_file_sets: Arc<DashMap<FileId, FileIdSet>>,
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
        db: &mut dyn SourceDatabase,
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
    pub fn package_root(&self, package_id: PackageId) -> PackageRootInput {
        let package_root = self
            .package_roots
            .get(&package_id)
            .expect("Unable to fetch source root id; this is a bug");

        *package_root
    }

    pub fn set_package_root_with_durability(
        &self,
        db: &mut dyn SourceDatabase,
        package_id: PackageId,
        package_root: Arc<PackageRoot>,
        durability: Durability,
    ) {
        match self.package_roots.entry(package_id) {
            Entry::Occupied(mut occupied) => {
                occupied
                    .get_mut()
                    .set_data(db)
                    .with_durability(durability)
                    .to(package_root);
            }
            Entry::Vacant(vacant) => {
                let package_root = PackageRootInput::builder(package_root)
                    .durability(durability)
                    .new(db);
                vacant.insert(package_root);
            }
        };
    }

    pub fn file_package_id(&self, id: FileId) -> PackageId {
        let file_package_id = self
            .file_package_ids
            .get(&id)
            .expect("Unable to fetch PackageId; this is a bug");
        *file_package_id
    }

    pub fn set_file_package_id(&self, file_id: FileId, package_id: PackageId) {
        self.file_package_ids.insert(file_id, package_id);
    }

    pub fn package_deps(&self, package_id: PackageId) -> PackageData {
        let package_deps = self
            .package_deps
            .get(&package_id)
            .expect("Unable to fetch package dependencies");
        *package_deps
    }

    pub fn set_package_deps(
        &self,
        db: &mut dyn SourceDatabase,
        package_id: PackageId,
        deps: Arc<Vec<PackageId>>,
    ) {
        match self.package_deps.entry(package_id) {
            Entry::Occupied(mut occupied) => {
                occupied.get_mut().set_deps(db).to(deps);
            }
            Entry::Vacant(vacant) => {
                let deps = PackageData::builder(deps).new(db);
                vacant.insert(deps);
            }
        };
    }

    pub fn spec_related_files(&self, file_id: FileId) -> FileIdSet {
        let spec_file_set = self
            .spec_file_sets
            .get(&file_id)
            .expect(&format!("Unable to fetch spec file set for {:?}", file_id));
        *spec_file_set
    }

    pub fn set_spec_related_files(
        &self,
        db: &mut dyn SourceDatabase,
        file_id: FileId,
        spec_related_files: Vec<FileId>,
    ) {
        match self.spec_file_sets.entry(file_id) {
            Entry::Occupied(mut occupied) => {
                occupied.get_mut().set_data(db).to(spec_related_files);
            }
            Entry::Vacant(vacant) => {
                let file_set = FileIdSet::new(db, vec![file_id]);
                vacant.insert(file_set);
            }
        };
    }

    pub fn package_ids(&self, db: &dyn SourceDatabase) -> PackageIdSet {
        let package_ids = self
            .package_roots
            .iter()
            .map(|it| it.key().clone())
            .collect::<Vec<_>>();
        PackageIdSet::new(db, package_ids)
    }
}
