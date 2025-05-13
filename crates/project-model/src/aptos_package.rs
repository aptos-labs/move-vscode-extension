use crate::manifest_path::ManifestPath;
use crate::move_toml::{MoveToml, MoveTomlDependency};
use anyhow::Context;
use base_db::change::{DepGraph, ManifestFileId};
use paths::{AbsPath, AbsPathBuf};
use std::collections::HashSet;
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
    pub fn load(root_manifest: &ManifestPath, resolve_deps: bool) -> anyhow::Result<AptosPackage> {
        let _p =
            tracing::info_span!("load package at", "{:?}", root_manifest.canonical_root()).entered();
        let mut visited = HashSet::new();
        AptosPackage::load_inner(root_manifest, false, resolve_deps, &mut visited)
            .with_context(|| format!("Failed to load the project at {root_manifest}"))
    }

    fn load_inner(
        manifest_path: &ManifestPath,
        is_git: bool,
        resolve_deps: bool,
        visited: &mut HashSet<AbsPathBuf>,
    ) -> anyhow::Result<Self> {
        let file_contents = fs::read_to_string(&manifest_path)
            .with_context(|| format!("Failed to read Move.toml file {manifest_path}"))?;
        let move_toml = MoveToml::from_str(file_contents.as_str())
            .with_context(|| format!("Failed to deserialize Move.toml file {manifest_path}"))?;

        let package_root = manifest_path.root();

        let mut dep_roots = vec![];
        let mut dep_manifests = vec![];
        if resolve_deps {
            for toml_dep in move_toml.dependencies.clone() {
                if let Some(dep_root) = toml_dep.dep_root(&package_root) {
                    let move_toml_path = dep_root.join("Move.toml");

                    let canonical_path = fs::canonicalize(&move_toml_path);
                    match canonical_path {
                        Ok(path) => {
                            let path = AbsPathBuf::assert_utf8(path);
                            if visited.contains(&path) {
                                // visited already, circular dependency
                                tracing::error!("circular dependency in {:?}", path);
                                break;
                            }
                            visited.insert(path);
                        }
                        Err(_) => {
                            tracing::error!(
                                "dependency resolution error: cannot canonicalize path {:?}",
                                move_toml_path
                            );
                            break;
                        }
                    }

                    if fs::exists(&move_toml_path).is_ok_and(|it| it) {
                        let manifest_path = ManifestPath::new(move_toml_path);
                        dep_roots.push(manifest_path.canonical_root());
                        let is_git = matches!(toml_dep, MoveTomlDependency::Git(_));
                        dep_manifests.push((manifest_path, is_git));
                    } else {
                        tracing::warn!(?move_toml_path, "invalid dependency: manifest does not exist");
                    }
                }
            }
            tracing::info!("dep_roots = {:#?}", dep_roots);
        }

        let deps = dep_manifests
            .into_iter()
            .filter_map(|(manifest, is_git)| {
                let _p = tracing::info_span!("load dep package at", "{:?}", manifest.canonical_root())
                    .entered();
                AptosPackage::load_inner(&manifest, is_git, resolve_deps, visited).ok()
            })
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
            is_local: !self.is_git,
        }
    }

    pub fn dependency_manifests(&self) -> Vec<ManifestPath> {
        let mut manifests = vec![];
        for toml_dep in self.move_toml.dependencies.clone() {
            if let Some(dep_root) = toml_dep.dep_root(&self.content_root) {
                let move_toml_path = dep_root.join("Move.toml");
                if fs::exists(&move_toml_path).is_ok() {
                    let manifest_path = ManifestPath::new(move_toml_path);
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
