// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::SourceDatabase;
use crate::inputs::PackageMetadata;
use crate::package_root::{PackageKind, PackageRoot};
use salsa::Durability;
use std::collections::HashMap;
use std::fmt;
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
            db.replace_package_roots(package_roots);
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

        let is_replacing_package_roots = self.package_roots.is_some();
        for (file_id, text) in self.files_changed {
            let text = text.unwrap_or_default();
            // only use durability if roots are explicitly provided
            if is_replacing_package_roots {
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

fn file_text_durability(package_root: &PackageRoot) -> Durability {
    match package_root.kind {
        PackageKind::Local => Durability::LOW,
        PackageKind::Library => Durability::HIGH,
    }
}
