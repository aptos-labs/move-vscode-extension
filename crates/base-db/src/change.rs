use crate::SourceDatabase;
use crate::package_root::{PackageId, PackageRoot};
use salsa::Durability;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use vfs::FileId;

pub type PackageFileId = FileId;
pub type PackageGraph = HashMap<PackageFileId, Vec<PackageFileId>>;

/// Encapsulate a bunch of raw `.set` calls on the database.
#[derive(Default)]
pub struct FileChanges {
    pub builtins_file: Option<(FileId, String)>,
    pub files_changed: Vec<(FileId, Option<String>)>,
    pub package_roots: Option<Vec<PackageRoot>>,
    pub package_graph: Option<HashMap<PackageFileId, Vec<PackageFileId>>>,
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
                let package_id = PackageId::new(db, idx as u32, root.root_dir.clone());
                let durability = package_root_durability(&root);
                for file_id in root.file_set.iter() {
                    db.set_file_package_id(file_id, package_id);
                    db.set_spec_related_files(
                        file_id,
                        find_spec_file_set(file_id, root.clone()).unwrap_or(vec![file_id]),
                    );
                }
                db.set_package_root_with_durability(package_id, Arc::from(root), durability);
                db.set_dep_package_ids(package_id, Default::default());
            }
        }

        if let Some((builtins_file_id, builtins_text)) = self.builtins_file {
            tracing::info!(?builtins_file_id, "set builtins file");
            db.set_builtins_file_id(Some(builtins_file_id));
            db.set_file_text_with_durability(builtins_file_id, builtins_text.as_str(), Durability::HIGH);
            db.set_spec_related_files(builtins_file_id, vec![builtins_file_id]);
        }

        if let Some(package_graph) = self.package_graph {
            let _p = tracing::info_span!("set package dependencies").entered();
            for (manifest_file_id, dep_manifest_ids) in package_graph.into_iter() {
                let main_package_id = db.file_package_id(manifest_file_id);
                let deps_package_ids = dep_manifest_ids
                    .into_iter()
                    .map(|it| db.file_package_id(it))
                    .collect::<Vec<_>>();
                tracing::info!(
                    main_package = main_package_id.root_dir(db),
                    dep_package_ids = ?package_ids_to_names(db, &deps_package_ids),
                );
                db.set_dep_package_ids(main_package_id, deps_package_ids);
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

pub fn package_ids_to_names(db: &dyn SourceDatabase, ids: &[PackageId]) -> Vec<Option<String>> {
    ids.iter().map(|it| it.root_dir(db)).collect::<Vec<_>>()
}

fn find_spec_file_set(file_id: FileId, root: PackageRoot) -> Option<Vec<FileId>> {
    // simplification for now: only use MODULE_NAME.spec.move in the immediate vicinity
    // todo: fix later, requires refactoring into one pass on the upper level
    let file_path = root.file_set.path_for_file(&file_id)?;
    let (file_name, ext) = file_path.name_and_extension()?;
    if ext != Some("move") {
        // shouldn't really happen
        return None;
    }
    let parent_dir = file_path.parent()?;
    let candidate = match file_name.strip_suffix(".spec") {
        None => {
            // MODULE_NAME.move, searching for MODULE_NAME.spec.move
            parent_dir.join(&format!("{file_name}.spec.move"))
        }
        Some(truncated_file_name) => {
            // MODULE_NAME.spec.move -> MODULE_NAME.move
            parent_dir.join(&format!("{truncated_file_name}.move"))
        }
    }?;
    let candidate_file_id = root.file_for_path(&candidate)?;
    Some(vec![file_id, *candidate_file_id])
}

fn package_root_durability(package_root: &PackageRoot) -> Durability {
    if package_root.is_library {
        Durability::MEDIUM
    } else {
        Durability::LOW
    }
}

fn file_text_durability(package_root: &PackageRoot) -> Durability {
    if package_root.is_library {
        Durability::HIGH
    } else {
        Durability::LOW
    }
}
