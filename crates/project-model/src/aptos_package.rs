// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::manifest_path::ManifestPath;
use base_db::inputs::AddressPair;
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
    pub manifest_file: AbsPathBuf,
    /// Is from the local filesystem and may be edited
    pub is_local: bool,
}

impl PackageFolderRoot {
    pub fn content_root(&self) -> &AbsPath {
        self.manifest_file.parent().unwrap()
    }

    pub fn source_dirs(&self) -> Vec<AbsPathBuf> {
        vec![
            self.content_root().join("sources"),
            self.content_root().join("tests"),
            self.content_root().join("scripts"),
        ]
    }

    pub fn build_dir(&self) -> AbsPathBuf {
        self.content_root().join("build")
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PackageKind {
    Local,
    Git,
}

#[derive(Clone)]
pub struct AptosPackage {
    pub package_name: Option<String>,
    manifest_path: AbsPathBuf,
    kind: PackageKind,
    transitive_dep_roots: Vec<(AbsPathBuf, PackageKind)>,
    pub resolve_deps: bool,
    pub named_addresses: Vec<AddressPair>,
    pub missing_dependencies: Vec<String>,
}

impl fmt::Debug for AptosPackage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("AptosPackage")
            .field("package_name", &self.package_name)
            .field("manifest_path", &self.manifest_path.to_string())
            .field("sourced_from", &self.kind)
            .field("named_addresses", &self.named_addresses)
            .field("deps", &self.transitive_dep_roots)
            .field("resolve_deps", &self.resolve_deps)
            .field("missing_dependencies", &self.missing_dependencies)
            .finish()
    }
}

impl AptosPackage {
    pub fn new(
        package_name: Option<String>,
        manifest_path: &ManifestPath,
        kind: PackageKind,
        dep_roots: Vec<(ManifestPath, PackageKind)>,
        resolve_deps: bool,
        named_addresses: Vec<AddressPair>,
        missing_dependencies: Vec<String>,
    ) -> Self {
        AptosPackage {
            package_name,
            manifest_path: manifest_path.file.clone(),
            kind,
            transitive_dep_roots: dep_roots
                .into_iter()
                .map(|(manifest, kind)| (manifest.content_root(), kind))
                .collect(),
            resolve_deps,
            named_addresses,
            missing_dependencies,
        }
    }

    pub fn content_root(&self) -> &AbsPath {
        self.manifest_path
            .parent()
            .expect("manifest always has a parent dir")
    }

    pub fn display_root(&self) -> String {
        self.content_root().to_string()
    }

    pub fn dep_roots(&self) -> &[(AbsPathBuf, PackageKind)] {
        &self.transitive_dep_roots
    }

    pub fn manifest_path(&self) -> &AbsPath {
        self.manifest_path.as_path()
    }

    pub fn is_local(&self) -> bool {
        self.kind == PackageKind::Local
    }

    pub fn to_folder_root(&self) -> PackageFolderRoot {
        PackageFolderRoot {
            manifest_file: self.manifest_path.clone(),
            is_local: self.kind == PackageKind::Local,
        }
    }

    pub fn contains_file(&self, file_path: &AbsPath) -> bool {
        file_path.starts_with(self.content_root())
    }
}
