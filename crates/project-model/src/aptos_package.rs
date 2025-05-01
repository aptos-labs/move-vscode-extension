use crate::manifest_path::ManifestPath;
use crate::move_toml::{MoveToml, MoveTomlDependency};
use anyhow::Context;
use base_db::change::{DepGraph, ManifestFileId};
use paths::{AbsPath, AbsPathBuf};
use std::fmt::Formatter;
use std::{fmt, fs};
use vfs::FileId;

pub type FileLoader<'a> = &'a mut dyn for<'b> FnMut(&'b AbsPath) -> Option<FileId>;

/// `PackageFolderRoot` describes a package root folder.
/// Which may be an external dependency, or a member of
/// the current workspace.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct PackageFolderRoot {
    pub content_root: AbsPathBuf,
    /// Is from the local filesystem and may be edited
    pub is_local: bool,
}

#[derive(Clone, Eq, PartialEq)]
pub struct AptosPackage {
    content_root: AbsPathBuf,
    move_toml: MoveToml,
    is_git: bool,
    // is_dep: bool,
    deps: Vec<AptosPackage>,
}

impl fmt::Debug for AptosPackage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("AptosPackage")
            .field("content_root", &self.content_root().to_string())
            .finish()
    }
}

impl AptosPackage {
    pub fn load(root_manifest: &ManifestPath) -> anyhow::Result<AptosPackage> {
        let _p =
            tracing::info_span!("load package at", "{:?}", root_manifest.canonical_root()).entered();
        AptosPackage::load_inner(root_manifest, false)
            .with_context(|| format!("Failed to load the project at {root_manifest}"))
    }

    pub fn load_dependency(root_manifest: &ManifestPath, is_git: bool) -> anyhow::Result<AptosPackage> {
        let _p =
            tracing::info_span!("load dep package at", "{:?}", root_manifest.canonical_root()).entered();
        AptosPackage::load_inner(root_manifest, is_git)
    }

    fn load_inner(manifest_path: &ManifestPath, is_git: bool) -> anyhow::Result<Self> {
        let file_contents = fs::read_to_string(&manifest_path)
            .with_context(|| format!("Failed to read Move.toml file {manifest_path}"))?;
        let move_toml = MoveToml::from_str(file_contents.as_str())
            .with_context(|| format!("Failed to deserialize Move.toml file {manifest_path}"))?;

        let package_root = manifest_path.root();

        let mut dep_roots = vec![];
        let mut dep_manifests = vec![];
        for toml_dep in move_toml.dependencies.clone() {
            if let Some(dep_root) = toml_dep.dep_root(&package_root) {
                let move_toml_path = dep_root.join("Move.toml");
                if fs::exists(&move_toml_path).is_ok_and(|it| it) {
                    let manifest_path = ManifestPath::from_manifest_file(move_toml_path).unwrap();
                    dep_roots.push(manifest_path.canonical_root());
                    let is_git = matches!(toml_dep, MoveTomlDependency::Git(_));
                    dep_manifests.push((manifest_path, is_git));
                } else {
                    tracing::warn!(?move_toml_path, "invalid dependency: manifest does not exist");
                }
            }
        }
        tracing::info!("dep_roots = {:#?}", dep_roots);

        let deps = dep_manifests
            .into_iter()
            .filter_map(|(it, is_git)| AptosPackage::load_dependency(&it, is_git).ok())
            .collect();

        Ok(AptosPackage {
            content_root: package_root,
            move_toml,
            is_git,
            deps,
        })
    }

    pub fn content_root(&self) -> &AbsPath {
        self.content_root.as_path()
    }

    pub fn deps(&self) -> impl Iterator<Item = &AptosPackage> {
        self.deps.iter()
    }

    pub fn manifest_path(&self) -> ManifestPath {
        let file = self.content_root.join("Move.toml");
        ManifestPath { file }
    }

    pub fn package_and_deps(&self) -> Vec<&AptosPackage> {
        let mut refs = vec![self];
        for dep in self.deps() {
            refs.extend(dep.package_and_deps());
        }
        refs
    }

    pub fn to_dep_graph(&self, load: FileLoader<'_>) -> Option<DepGraph> {
        tracing::info!("reloading package at {}", self.content_root());

        let mut package_graph = DepGraph::default();
        for pkg in self.package_and_deps() {
            let package_file_id = pkg.load_manifest_file_id(load)?;

            let mut dep_ids = vec![];
            self.collect_dep_ids(&mut dep_ids, pkg, load);
            dep_ids.sort();
            dep_ids.dedup();

            package_graph.insert(package_file_id, dep_ids);
        }

        Some(package_graph)
    }

    fn collect_dep_ids(
        &self,
        dep_ids: &mut Vec<ManifestFileId>,
        package_ref: &AptosPackage,
        load: FileLoader<'_>,
    ) {
        for dep_package in package_ref.deps() {
            if let Some(dep_file_id) = dep_package.load_manifest_file_id(load) {
                dep_ids.push(dep_file_id);
                self.collect_dep_ids(dep_ids, dep_package, load);
            }
        }
    }

    /// Returns the roots for the current `AptosPackage`
    /// The return type contains the path and whether or not
    /// the root is a member of the current workspace
    pub fn to_folder_roots(&self) -> Vec<PackageFolderRoot> {
        self.package_and_deps()
            .into_iter()
            .map(|it| it.to_folder_root())
            .collect()
    }

    pub fn to_folder_root(&self) -> PackageFolderRoot {
        PackageFolderRoot {
            content_root: self.content_root.to_path_buf(),
            is_local: !self.is_git,
        }
    }

    pub fn dependency_manifests(&self) -> Vec<ManifestPath> {
        let mut manifests = vec![];
        for toml_dep in self.move_toml.dependencies.clone() {
            if let Some(dep_root) = toml_dep.dep_root(&self.content_root) {
                let move_toml_path = dep_root.join("Move.toml");
                if fs::exists(&move_toml_path).is_ok() {
                    let manifest_path = ManifestPath::from_manifest_file(move_toml_path).unwrap();
                    manifests.push(manifest_path);
                }
            }
        }
        manifests
    }

    pub fn contains_file(&self, file_path: &AbsPath) -> bool {
        file_path.starts_with(self.content_root())
        // self.to_folder_root()
        //     .content_root
        //     .iter()
        //     .any(|source_folder| file_path.starts_with(source_folder))
    }

    pub(crate) fn load_manifest_file_id(&self, load: FileLoader<'_>) -> Option<ManifestFileId> {
        let manifest_file = self.manifest_path().file;
        match load(manifest_file.as_path()) {
            Some(file_id) => Some(file_id),
            None => {
                tracing::info!("cannot load {:?} from the filesystem", manifest_file.as_path());
                None
            }
        }
    }
}
