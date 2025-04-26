use crate::aptos_workspace::{FileLoader, PackageFolderRoot};
use crate::manifest_path::ManifestPath;
use crate::move_toml::MoveToml;
use anyhow::Context;
use base_db::change::ManifestFileId;
use paths::{AbsPath, AbsPathBuf};
use std::fmt::Formatter;
use std::{fmt, fs};

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
    pub fn load(manifest_path: ManifestPath, is_dep: bool) -> anyhow::Result<Self> {
        tracing::info!("load package at {:?}", fs::canonicalize(&manifest_path.file)?);

        let file_contents = fs::read_to_string(&manifest_path)
            .with_context(|| format!("Failed to read Move.toml file {manifest_path}"))?;

        let move_toml = MoveToml::from_str(file_contents.as_str())
            .with_context(|| format!("Failed to deserialize Move.toml file {manifest_path}"))?;
        let content_root = manifest_path.parent().to_path_buf();

        let mut dep_manifests = vec![];
        for toml_dep in move_toml.dependencies.clone() {
            if let Some(dep_root) = toml_dep.dep_root(&content_root) {
                let move_toml_path = dep_root.join("Move.toml");
                if fs::exists(&move_toml_path).is_ok_and(|it| it) {
                    let manifest_path = ManifestPath::from_manifest_file(move_toml_path).unwrap();
                    dep_manifests.push(manifest_path);
                } else {
                    tracing::warn!(?move_toml_path, "invalid dependency: manifest does not exist");
                }
            }
        }
        let deps = dep_manifests
            .into_iter()
            .filter_map(|it| AptosPackage::load(it, true).ok())
            .collect();

        Ok(AptosPackage {
            content_root,
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

    pub fn manifest(&self) -> ManifestPath {
        let file = self.content_root.join("Move.toml");
        ManifestPath { file }
    }

    pub fn to_folder_root(&self) -> PackageFolderRoot {
        // let sources = self.content_root.join("sources");
        // let tests = self.content_root.join("tests");
        // let scripts = self.content_root.join("scripts");
        // let manifest = self.content_root.join("Move.toml");
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
        let manifest_file = self.manifest().file;
        match load(manifest_file.as_path()) {
            Some(file_id) => Some(file_id),
            None => {
                tracing::info!("cannot load {:?} from filesystem", manifest_file.as_path());
                None
            }
        }
    }
}
