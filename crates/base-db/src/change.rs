// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::SourceDatabase;
use crate::inputs::PackageMetadata;
use crate::package_root::{PackageId, PackageKind, PackageRoot};
use salsa::Durability;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::Arc;
use vfs::FileId;

pub type ManifestFileId = FileId;
pub type PackageGraph = HashMap<ManifestFileId, PackageMetadata>;

/// Encapsulate a bunch of raw `.set` calls on the database.
#[derive(Default)]
pub struct FileChanges {
    pub builtins_file: Option<(FileId, String)>,
    pub files_changed: Vec<(FileId, Option<String>)>,
    pub package_roots: Option<Vec<PackageRoot>>,
    pub package_graph: Option<PackageGraph>,
}

impl fmt::Debug for FileChanges {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = fmt.debug_struct("Change");
        if let Some(packages) = &self.package_roots {
            d.field("packages", packages);
        }
        if !self.files_changed.is_empty() {
            d.field("files_changed", &self.files_changed.len());
        }
        d.finish()
    }
}

impl FileChanges {
    pub fn new() -> Self {
        FileChanges::default()
    }

    pub fn set_package_roots(&mut self, package_roots: Vec<PackageRoot>) {
        self.package_roots = Some(package_roots);
    }

    pub fn set_package_graph(&mut self, package_graph: PackageGraph) {
        self.package_graph = Some(package_graph);
    }

    pub fn change_file(&mut self, file_id: FileId, new_text: Option<String>) {
        self.files_changed.push((file_id, new_text))
    }

    pub fn add_builtins_file(&mut self, file_id: FileId, builtins_text: String) {
        self.builtins_file = Some((file_id, builtins_text));
    }

    pub fn apply(self, db: &mut dyn SourceDatabase) {
        let _p = tracing::info_span!("FileChange::apply").entered();

        if let Some(package_roots) = self.package_roots.clone() {
            for (idx, root) in package_roots.into_iter().enumerate() {
                let package_id = PackageId::new(db, idx as u32);
                let durability = package_root_durability(&root);
                for file_id in root.file_set.iter() {
                    db.set_file_package_id(file_id, package_id);
                    db.set_spec_related_files(
                        file_id,
                        find_spec_file_set(file_id, root.clone()).unwrap_or(vec![]),
                    );
                }
                db.set_package_root_with_durability(package_id, Arc::from(root), durability);
            }
        }

        if let Some((builtins_file_id, builtins_text)) = self.builtins_file {
            db.set_builtins_file_id(Some(builtins_file_id));
            db.set_file_text_with_durability(builtins_file_id, builtins_text.as_str(), Durability::HIGH);
            db.set_spec_related_files(builtins_file_id, vec![]);
        }

        if let Some(package_graph) = self.package_graph {
            let _p = tracing::info_span!("set package dependencies").entered();
            for (package_manifest_id, package_metadata) in package_graph.into_iter() {
                db.set_package_metadata(package_manifest_id, package_metadata);
            }
        }

        let package_roots = self.package_roots;
        for (file_id, text) in self.files_changed {
            let text = text.unwrap_or_default();
            // only use durability if roots are explicitly provided
            if package_roots.is_some() {
                let package_id = db.file_package_id(file_id);
                let package_root = db.package_root(package_id).data(db);
                let durability = file_text_durability(&package_root);
                db.set_file_text_with_durability(file_id, text.as_str(), durability);
                continue;
            }
            // XXX: can't actually remove the file, just reset the text
            db.set_file_text(file_id, text.as_str())
        }
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

fn file_text_durability(package_root: &PackageRoot) -> Durability {
    match package_root.kind {
        PackageKind::Local => Durability::LOW,
        PackageKind::Library => Durability::HIGH,
    }
}
