use crate::manifest_path::ManifestPath;
use crate::move_toml::MoveToml;
use anyhow::Context;
use base_db::change::{ManifestFileId, PackageGraph};
use paths::{AbsPath, AbsPathBuf};
use std::fmt::Formatter;
use std::{fmt, fs};
use vfs::FileId;

pub type FileLoader<'a> = &'a mut dyn for<'b> FnMut(&'b AbsPath) -> Option<FileId>;

/// `PackageRoot` describes a package root folder.
/// Which may be an external dependency, or a member of
/// the current workspace.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct PackageFolderRoot {
    /// Is from the local filesystem and may be edited
    pub is_local: bool,
    /// Directories to include
    pub include: Vec<AbsPathBuf>,
    /// Directories to exclude
    pub exclude: Vec<AbsPathBuf>,
}

#[derive(Clone, Eq, PartialEq)]
pub struct AptosPackage {
    content_root: AbsPathBuf,
    move_toml: MoveToml,
    is_dep: bool,
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

    pub fn load_dependency(root_manifest: &ManifestPath) -> anyhow::Result<AptosPackage> {
        let _p =
            tracing::info_span!("load dep package at", "{:?}", root_manifest.canonical_root()).entered();
        AptosPackage::load_inner(root_manifest, true)
    }

    fn load_inner(manifest_path: &ManifestPath, is_dep: bool) -> anyhow::Result<Self> {
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
                    dep_manifests.push(manifest_path);
                } else {
                    tracing::warn!(?move_toml_path, "invalid dependency: manifest does not exist");
                }
            }
        }
        tracing::info!("dep_roots = {:#?}", dep_roots);

        let deps = dep_manifests
            .into_iter()
            .filter_map(|it| AptosPackage::load_dependency(&it).ok())
            .collect();

        Ok(AptosPackage {
            content_root: package_root,
            move_toml,
            is_dep,
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

    pub fn all_reachable_packages(&self) -> Vec<&AptosPackage> {
        package_refs(&self)
    }

    pub fn to_package_graph(&self, load: FileLoader<'_>) -> Option<PackageGraph> {
        tracing::info!("reloading aptos package at {}", self.content_root());

        let mut package_graph = PackageGraph::default();
        for pkg in self.all_reachable_packages() {
            let main_file_id = pkg.load_manifest_file_id(load)?;
            let mut dep_ids = vec![];
            self.collect_dep_ids(&mut dep_ids, pkg, load);
            dep_ids.sort();
            dep_ids.dedup();
            package_graph.insert(main_file_id, dep_ids);
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
        self.all_reachable_packages()
            .into_iter()
            .map(|it| it.to_folder_root())
            .collect()
    }

    pub fn to_folder_root(&self) -> PackageFolderRoot {
        PackageFolderRoot {
            is_local: !self.is_dep,
            include: vec![self.content_root.clone()],
            exclude: vec![self.content_root.join("build")],
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
        self.to_folder_root()
            .include
            .iter()
            .any(|source_folder| file_path.starts_with(source_folder))
    }

    pub(crate) fn load_manifest_file_id(&self, load: FileLoader<'_>) -> Option<ManifestFileId> {
        let manifest_file = self.manifest_path().file;
        match load(manifest_file.as_path()) {
            Some(file_id) => Some(file_id),
            None => {
                tracing::info!("cannot load {:?} from filesystem", manifest_file.as_path());
                None
            }
        }
    }
}

fn package_refs(package: &AptosPackage) -> Vec<&AptosPackage> {
    let mut refs = vec![package];
    for dep in package.deps() {
        refs.extend(package_refs(dep));
    }
    refs
}
