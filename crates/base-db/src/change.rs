use crate::SourceDatabase;
use crate::package_root::{PackageRoot, PackageRootId};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use vfs::FileId;

pub type ManifestFileId = FileId;
pub type PackageGraph = HashMap<ManifestFileId, Vec<ManifestFileId>>;

/// Encapsulate a bunch of raw `.set` calls on the database.
#[derive(Default)]
pub struct FileChanges {
    pub builtins_file: Option<(FileId, String)>,
    pub files_changed: Vec<(FileId, Option<String>)>,
    pub package_roots: Option<Vec<PackageRoot>>,
    pub package_graph: Option<HashMap<ManifestFileId, Vec<ManifestFileId>>>,
}

impl FileChanges {
    pub fn new() -> Self {
        FileChanges::default()
    }

    pub fn set_package_roots(&mut self, packages: Vec<PackageRoot>) {
        self.package_roots = Some(packages);
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

        if let Some(package_roots) = self.package_roots {
            for (idx, root) in package_roots.into_iter().enumerate() {
                let root_id = PackageRootId(idx as u32);
                let root_file_set = &root.file_set;
                for file_id in root_file_set.iter() {
                    db.set_file_package_root(file_id, root_id);
                }
                db.set_package_root(root_id, Arc::from(root));
                db.set_package_deps(root_id, Default::default());
            }
        }

        if let Some((builtins_file_id, builtins_text)) = self.builtins_file {
            tracing::info!(?builtins_file_id);
            db.set_builtins_file_id(Some(builtins_file_id));
            db.set_file_text(builtins_file_id, Arc::from(builtins_text));
        }

        if let Some(package_graph) = self.package_graph {
            let _p = tracing::info_span!("set package graph").entered();
            for (manifest_file_id, dep_manifest_ids) in package_graph.into_iter() {
                let main_package_id = db.file_package_root(manifest_file_id);
                let deps_package_ids = dep_manifest_ids
                    .into_iter()
                    .map(|it| db.file_package_root(it))
                    .collect::<Vec<_>>();
                tracing::info!(?main_package_id, ?deps_package_ids);
                db.set_package_deps(main_package_id, Arc::from(deps_package_ids));
            }
        }

        for (file_id, text) in self.files_changed {
            // XXX: can't actually remove the file, just reset the text
            let text = text.unwrap_or_default();
            db.set_file_text(file_id, Arc::from(text))
        }
    }
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
