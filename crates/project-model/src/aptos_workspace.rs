use crate::aptos_package::AptosPackage;
use crate::manifest_path::ManifestPath;
use anyhow::Context;
use base_db::change::{ManifestFileId, PackageGraph};
use paths::{AbsPath, AbsPathBuf};
use std::iter;
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

// todo: rename to AptosProject
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AptosWorkspace {
    main_package: AptosPackage,
    deps: Vec<AptosPackage>,
}

impl AptosWorkspace {
    pub fn load(manifest: ManifestPath) -> anyhow::Result<AptosWorkspace> {
        AptosWorkspace::load_inner(manifest.clone())
            .with_context(|| format!("Failed to load the project at {manifest}"))
    }

    fn load_inner(manifest: ManifestPath) -> anyhow::Result<AptosWorkspace> {
        // todo: run `aptos metadata` (see rust-analyzer for error handling and progress reporting)

        let main_package = AptosPackage::load(manifest, false)?;
        let mut deps = vec![];
        for dep_manifest_file in main_package.deps() {
            let dep_package = AptosPackage::load(dep_manifest_file, true);
            match dep_package {
                Ok(dep_package) => {
                    deps.push(dep_package);
                }
                Err(err) => {
                    tracing::error!("cannot load dependency: {:?}", err.to_string());
                }
            }
        }
        // todo: fetch package dependencies
        // todo: fetch declared named addresses

        Ok(AptosWorkspace { main_package, deps })
    }

    pub fn workspace_root(&self) -> &AbsPath {
        self.main_package.content_root()
    }

    pub fn manifest_path(&self) -> ManifestPath {
        self.main_package.manifest()
    }

    pub fn manifest(&self) -> Option<ManifestPath> {
        self.main_package.manifest().into()
    }

    /// Returns the roots for the current `AptosWorkspace`
    /// The return type contains the path and whether or not
    /// the root is a member of the current workspace
    pub fn to_folder_roots(&self) -> Vec<PackageFolderRoot> {
        self.iter_packages().map(|it| it.to_folder_root()).collect()
    }

    pub fn to_package_graph(&self, load: FileLoader<'_>) -> Option<PackageGraph> {
        tracing::info!(
            "aptos package at {:?} into PackageGraph",
            self.main_package.content_root()
        );

        let manifest_file_id = self.main_package.load_manifest_file_id(load)?;

        let mut package_graph = PackageGraph::default();
        let mut deps = vec![];
        for dep in self.deps.iter() {
            let dep_manifest_file_id = dep.load_manifest_file_id(load)?;
            deps.push(dep_manifest_file_id);
        }
        package_graph.insert(manifest_file_id, deps);

        Some(package_graph)
    }

    fn load_manifest_file_id(package: &AptosPackage, load: FileLoader<'_>) -> Option<ManifestFileId> {
        let manifest_file = package.manifest().file;
        match load(manifest_file.as_path()) {
            Some(file_id) => Some(file_id),
            None => {
                tracing::info!("cannot load FileId for {:?}", manifest_file.as_path());
                None
            }
        }
    }

    pub fn iter_packages(&self) -> impl Iterator<Item = &AptosPackage> {
        iter::once(&self.main_package).chain(self.deps.iter())
    }

    pub fn contains_file(&self, file_path: &AbsPath) -> bool {
        self.iter_packages().any(|pkg| pkg.contains_file(file_path))
    }
}
