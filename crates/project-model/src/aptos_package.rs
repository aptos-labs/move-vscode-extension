use crate::manifest_path::ManifestPath;
use paths::{AbsPath, AbsPathBuf};
use std::fmt;
use std::fmt::Formatter;
use vfs::FileId;

pub mod load_from_fs;

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
    kind: PackageKind,
    transitive_dep_roots: Vec<(AbsPathBuf, PackageKind)>,
}

impl fmt::Debug for AptosPackage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("AptosPackage")
            .field("content_root", &self.content_root().to_string())
            .field("sourced_from", &self.kind)
            .field("deps", &self.transitive_dep_roots)
            .finish()
    }
}

impl AptosPackage {
    pub fn new(
        manifest_path: &ManifestPath,
        kind: PackageKind,
        dep_roots: Vec<(ManifestPath, PackageKind)>,
    ) -> Self {
        AptosPackage {
            content_root: manifest_path.content_root(),
            kind,
            transitive_dep_roots: dep_roots
                .into_iter()
                .map(|(manifest, kind)| (manifest.content_root(), kind))
                .collect(),
        }
    }

    pub fn content_root(&self) -> &AbsPath {
        self.content_root.as_path()
    }

    pub fn dep_roots(&self) -> &[(AbsPathBuf, PackageKind)] {
        &self.transitive_dep_roots
    }

    pub fn manifest_path(&self) -> ManifestPath {
        let file = self.content_root.join("Move.toml");
        ManifestPath { file }
    }

    pub fn is_local(&self) -> bool {
        self.kind == PackageKind::Local
    }

    pub fn to_folder_root(&self) -> PackageFolderRoot {
        PackageFolderRoot {
            content_root: self.content_root.to_path_buf(),
            is_local: self.kind == PackageKind::Local,
        }
    }

    pub fn contains_file(&self, file_path: &AbsPath) -> bool {
        file_path.starts_with(self.content_root())
    }
}
