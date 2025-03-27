use crate::aptos_package::AptosPackage;
use crate::manifest_path::ManifestPath;
use anyhow::Context;
use paths::{AbsPath, AbsPathBuf};
use vfs::FileId;

pub type FileLoader<'a> = &'a mut dyn for<'b> FnMut(&'b AbsPath) -> Option<FileId>;

/// `PackageRoot` describes a package root folder.
/// Which may be an external dependency, or a member of
/// the current workspace.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct PackageRoot {
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
}

impl AptosWorkspace {
    pub fn load(manifest: ManifestPath) -> anyhow::Result<AptosWorkspace> {
        AptosWorkspace::load_inner(manifest.clone())
            .with_context(|| format!("Failed to load the project at {manifest}"))
    }

    fn load_inner(manifest: ManifestPath) -> anyhow::Result<AptosWorkspace> {
        // todo: run `aptos metadata` (see rust-analyzer for error handling and progress reporting)

        let main_package = AptosPackage::load(manifest)?;

        // todo: fetch package dependencies
        // todo: fetch declared named addresses

        Ok(AptosWorkspace { main_package })
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

    pub fn packages(&self) -> Vec<&AptosPackage> {
        vec![&self.main_package]
    }

    /// Returns the roots for the current `AptosWorkspace`
    /// The return type contains the path and whether or not
    /// the root is a member of the current workspace
    pub fn to_roots(&self) -> Vec<PackageRoot> {
        vec![self.main_package.to_root()]
    }

    pub fn contains_file(&self, file_path: &AbsPath) -> bool {
        self.packages().iter().any(|pkg| pkg.contains_file(file_path))
    }
}
