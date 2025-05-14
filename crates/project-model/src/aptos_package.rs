use crate::manifest_path::ManifestPath;
use anyhow::Context;
use paths::{AbsPath, AbsPathBuf};
use std::fmt;
use std::fmt::Formatter;
use vfs::FileId;

pub mod load_from_fs;
mod load_from_fs_2;

pub type VfsLoader<'a> = &'a mut dyn for<'b> FnMut(&'b AbsPath) -> Option<FileId>;

/// `PackageFolderRoot` describes a package root folder.
/// Which may be an external dependency, or a member of
/// the current workspace.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PackageFolderRoot {
    pub content_root: AbsPathBuf,
    /// Is from the local filesystem and may be edited
    pub is_local: bool,
}

impl PackageFolderRoot {
    pub fn source_dirs(&self) -> Vec<AbsPathBuf> {
        vec![
            self.content_root.join("sources"),
            self.content_root.join("tests"),
            self.content_root.join("scripts"),
        ]
    }

    pub fn build_dir(&self) -> AbsPathBuf {
        self.content_root.join("build")
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PackageKind {
    Local,
    Git,
}

#[derive(Clone, Eq, PartialEq)]
pub struct AptosPackage {
    content_root: AbsPathBuf,
    deps: Vec<AptosPackage>,
    sourced_from: PackageKind,
}

impl fmt::Debug for AptosPackage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("AptosPackage")
            .field("content_root", &self.content_root().to_string())
            .field("sourced_from", &self.sourced_from)
            .field("deps", &self.deps)
            .finish()
    }
}

impl AptosPackage {
    pub fn new(
        manifest_path: &ManifestPath,
        sourced_from: PackageKind,
        deps: Vec<AptosPackage>,
    ) -> Self {
        AptosPackage {
            content_root: manifest_path.content_root(),
            sourced_from,
            deps,
        }
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

    /// Returns the roots for the current `AptosPackage`
    /// The return type contains the path and whether or not
    /// the root is a member of the current workspace
    pub fn package_and_deps_folder_roots(&self) -> Vec<PackageFolderRoot> {
        self.package_and_deps()
            .into_iter()
            .map(|it| it.to_folder_root())
            .collect()
    }

    pub fn to_folder_root(&self) -> PackageFolderRoot {
        PackageFolderRoot {
            content_root: self.content_root.to_path_buf(),
            is_local: self.sourced_from == PackageKind::Local,
        }
    }

    pub fn contains_file(&self, file_path: &AbsPath) -> bool {
        file_path.starts_with(self.content_root())
    }
}
