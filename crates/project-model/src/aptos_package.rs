use crate::aptos_workspace::PackageRoot;
use crate::manifest_path::ManifestPath;
use crate::move_toml::MoveToml;
use anyhow::Context;
use paths::AbsPath;
use std::fs;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AptosPackage {
    content_root: paths::AbsPathBuf,
    move_toml: MoveToml,
}

impl AptosPackage {
    pub fn load(manifest_path: ManifestPath) -> anyhow::Result<Self> {
        let file = fs::read_to_string(&manifest_path)
            .with_context(|| format!("Failed to read Move.toml file {manifest_path}"))?;
        let move_toml = MoveToml::from_str(file.as_str())
            .with_context(|| format!("Failed to deserialize Move.toml file {manifest_path}"))?;
        let package_root = manifest_path.parent().to_path_buf();

        let package = AptosPackage {
            move_toml,
            content_root: package_root,
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

    pub fn to_root(&self) -> PackageRoot {
        let sources = self.content_root.join("sources");
        let tests = self.content_root.join("tests");
        let scripts = self.content_root.join("scripts");
        PackageRoot {
            is_local: true,
            include: vec![sources, tests, scripts],
            exclude: vec![],
        }
    }

    pub fn contains_file(&self, file_path: &AbsPath) -> bool {
        self.to_root()
            .include
            .iter()
            .any(|source_folder| file_path.starts_with(source_folder))
    }
}
