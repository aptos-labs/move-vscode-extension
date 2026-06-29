// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::change::ManifestFileId;
use crate::package_root::{PackageId, PackageKind, PackageRoot};
use crate::source_db::SourceDatabase;
use dashmap::{DashMap, Entry};
use salsa::Durability;
use salsa::Setter;
use std::collections::HashSet;
use std::sync::Arc;
use vfs::FileId;

pub type AddressPair = (String, String);

#[salsa_macros::interned(no_lifetime)]
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

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PackageMetadata {
    pub package_name: Option<String>,
    pub dep_manifest_ids: Arc<Vec<ManifestFileId>>,
    pub resolve_deps: bool,
    pub named_addresses: Vec<AddressPair>,
    pub missing_dependencies: Vec<String>,
}

#[salsa::input]
pub struct PackageMetadataInput {
    pub metadata: PackageMetadata,
}

#[derive(Default)]
pub struct Files {
    files: Arc<DashMap<FileId, FileText>>,
    file_package_ids: Arc<DashMap<FileId, PackageId>>,

    package_roots: Arc<DashMap<PackageId, PackageRootInput>>,
    package_metadata: Arc<DashMap<ManifestFileId, PackageMetadataInput>>,

    spec_file_sets: Arc<DashMap<FileId, FileIdSet>>,

    pub all_package_ids: Option<PackageIdSet>,
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

    pub fn replace_package_roots(&self, db: &mut dyn SourceDatabase, package_roots: Vec<PackageRoot>) {
        self.file_package_ids.clear();
        self.package_roots.clear();

        self.spec_file_sets.clear();
        if let Some(builtins_file_id) = db.builtins_file_id() {
            db.set_spec_related_files(builtins_file_id.data(db), vec![])
        }

        let mut all_package_ids = vec![];
        for (idx, package_root) in package_roots.into_iter().enumerate() {
            let package_id = PackageId::new(db, idx as u32);
            for file_id in package_root.file_ids() {
                self.file_package_ids.insert(file_id, package_id);
                self.set_spec_related_files(
                    db,
                    file_id,
                    find_spec_file_set(file_id, package_root.clone()).unwrap_or(vec![]),
                );
            }
            let package_durability = package_root_durability(&package_root);
            self.package_roots.insert(
                package_id,
                PackageRootInput::builder(Arc::from(package_root))
                    .durability(package_durability)
                    .new(db),
            );
            all_package_ids.push(package_id);
        }

        self.all_package_ids
            .unwrap()
            .set_data(db)
            .with_durability(Durability::MEDIUM)
            .to(all_package_ids);
    }

    pub fn file_package_id(&self, file_id: FileId) -> PackageId {
        let file_package_id = self.file_package_ids.get(&file_id).expect(&format!(
            "Unable to fetch PackageId for {file_id:?}; this is a bug"
        ));
        *file_package_id
    }

    pub fn set_file_package_id(&self, file_id: FileId, package_id: PackageId) {
        self.file_package_ids.insert(file_id, package_id);
    }

    pub fn package_metadata(&self, manifest_file_id: ManifestFileId) -> PackageMetadataInput {
        let metadata = self.package_metadata.get(&manifest_file_id).unwrap_or_else(|| {
            panic!(
                "Unable to fetch package metadata for manifest_file_id = {}",
                manifest_file_id.index()
            );
        });
        *metadata
    }

    // NOTE: Durability::HIGH is critical here, it needs to be bigger than resolution data
    pub fn set_package_metadata(
        &self,
        db: &mut dyn SourceDatabase,
        package_manifest_id: ManifestFileId,
        metadata: PackageMetadata,
    ) {
        match self.package_metadata.entry(package_manifest_id) {
            Entry::Occupied(mut occupied) => {
                occupied
                    .get_mut()
                    .set_metadata(db)
                    .with_durability(Durability::MEDIUM)
                    .to(metadata);
            }
            Entry::Vacant(vacant) => {
                let input = PackageMetadataInput::builder(metadata)
                    .durability(Durability::MEDIUM)
                    .new(db);
                vacant.insert(input);
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
                let file_set = FileIdSet::new(db, spec_related_files);
                vacant.insert(file_set);
            }
        };
    }

    pub fn package_ids(&self) -> PackageIdSet {
        self.all_package_ids
            .expect("initialized during RootDatabase::new()")
    }
}

fn find_spec_file_set(file_id: FileId, root: PackageRoot) -> Option<Vec<FileId>> {
    // simplification for now: only use MODULE_NAME.spec.move files
    // todo: fix later, requires refactoring into one pass on the upper level
    let file_path = root.file_set.path_for_file(&file_id)?;
    let (file_name, ext) = file_path.name_and_extension()?;
    if ext != Some("move") {
        // shouldn't really happen
        return None;
    }

    let prefix_name = file_name.strip_suffix(".spec").unwrap_or(file_name);
    let expected_file_names =
        HashSet::from([format!("{prefix_name}.move"), format!("{prefix_name}.spec.move")]);

    let mut spec_file_ids = vec![];
    // search through the package files for the files with
    for file_id in root.file_set.iter() {
        if let Some(file_path) = root.path_for_file(&file_id)
            && let Some(candidate_file_name) = file_path.as_path().and_then(|it| it.file_name())
        {
            if expected_file_names.contains(candidate_file_name) {
                spec_file_ids.push(file_id);
            }
        }
    }

    Some(spec_file_ids)
}

fn package_root_durability(package_root: &PackageRoot) -> Durability {
    match package_root.kind {
        PackageKind::Local => Durability::LOW,
        PackageKind::Library => Durability::MEDIUM,
    }
}
