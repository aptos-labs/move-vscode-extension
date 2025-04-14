use crate::aptos_workspace::{FileLoader, PackageFolderRoot};
use crate::manifest_path::ManifestPath;
use crate::move_toml::MoveToml;
use anyhow::Context;
use base_db::change::ManifestFileId;
use paths::AbsPath;
use std::fs;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AptosPackage {
    content_root: paths::AbsPathBuf,
    move_toml: MoveToml,
    is_dep: bool,
}

impl AptosPackage {
    pub fn load(manifest_path: ManifestPath, is_dep: bool) -> anyhow::Result<Self> {
        tracing::info!("load package at {:?}", manifest_path.file.to_string());

        let file_contents = fs::read_to_string(&manifest_path)
            .with_context(|| format!("Failed to read Move.toml file {manifest_path}"))?;

        let move_toml = MoveToml::from_str(file_contents.as_str())
            .with_context(|| format!("Failed to deserialize Move.toml file {manifest_path}"))?;
        let package_root = manifest_path.parent().to_path_buf();

        let package = AptosPackage {
            move_toml,
            content_root: package_root,
            is_dep,
        };
        Ok(package)
    }

    pub fn content_root(&self) -> &AbsPath {
        self.content_root.as_path()
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

    pub fn deps(&self) -> Vec<ManifestPath> {
        let mut deps = vec![];
        for toml_dep in self.move_toml.dependencies.clone() {
            if let Some(dep_root) = toml_dep.dep_root(&self.content_root) {
                let move_toml_path = dep_root.join("Move.toml");
                if fs::exists(&move_toml_path).is_ok() {
                    let manifest_path = ManifestPath::from_manifest_file(move_toml_path).unwrap();
                    deps.push(manifest_path);
                }
            }
        }
        deps
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
                tracing::info!("cannot load FileId for {:?}", manifest_file.as_path());
                None
            }
        }
    }
}
