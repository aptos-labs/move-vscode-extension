// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use paths::{AbsPathBuf, Utf8PathBuf};
use serde_derive::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MoveToml {
    pub contents: String,
    pub package: Option<Package>,
    pub dependencies: Vec<MoveTomlDependency>,
    pub dev_dependencies: Vec<MoveTomlDependency>,
}

impl MoveToml {
    pub fn from_str(file_contents: &str) -> anyhow::Result<Self> {
        let mut move_toml = MoveToml {
            contents: file_contents.to_string(),
            package: None,
            dependencies: vec![],
            dev_dependencies: vec![],
        };

        let deserialized = toml::from_str::<HashMap<String, toml::Value>>(file_contents)?;
        if let Some(package_table) = deserialized.get("package") {
            move_toml.package = package_table.to_owned().try_into().ok();
        }

        // covers both [dependencies] table with inner tables and [dependencies.AptosFramework]
        if let Some(deps_table) = deserialized.get("dependencies").and_then(|d| d.as_table()) {
            for (dep_name, deps_inner_table) in deps_table {
                if let Some(dep) =
                    Self::parse_dependency_table(dep_name.to_string(), deps_inner_table.to_owned())
                {
                    move_toml.dependencies.push(dep);
                }
            }
        }

        // covers both [dev-dependencies] table with inner tables and [dev-dependencies.AptosFramework]
        if let Some(deps_table) = deserialized.get("dev-dependencies").and_then(|d| d.as_table()) {
            for (dep_name, deps_inner_table) in deps_table {
                if let Some(dep) =
                    Self::parse_dependency_table(dep_name.to_string(), deps_inner_table.to_owned())
                {
                    move_toml.dev_dependencies.push(dep);
                }
            }
        }

        Ok(move_toml)
    }

    fn parse_dependency_table(name: String, dep_table: toml::Value) -> Option<MoveTomlDependency> {
        let table = dep_table
            .as_table()?
            .to_owned()
            .try_into::<'_, DependencyTable>()
            .ok()?;
        if let Some(local) = table.local {
            return Some(MoveTomlDependency::Local(LocalDependency {
                name: name.to_owned(),
                path: local,
            }));
        }
        if let Some(git) = table.git {
            return Some(MoveTomlDependency::Git(GitDependency {
                name: name.to_owned(),
                git,
                rev: table.rev,
                subdir: table.subdir,
            }));
        }
        None
    }

    pub(crate) fn declared_dependencies(&self) -> impl Iterator<Item = &MoveTomlDependency> {
        self.dependencies.iter().chain(self.dev_dependencies.iter())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LocalDependency {
    name: String,
    path: String,
}

impl LocalDependency {
    pub fn dep_root(&self, move_toml_root: &AbsPathBuf) -> Option<AbsPathBuf> {
        let root = move_toml_root.join(self.path.clone());
        if !std::fs::metadata(&root).is_ok() {
            tracing::warn!("Dependency content root does not exist: {:?}", root.to_string());
            return None;
        }
        Some(root)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GitDependency {
    name: String,
    git: String,
    rev: Option<String>,
    subdir: Option<String>,
}

impl GitDependency {
    pub fn dep_root(&self) -> Option<AbsPathBuf> {
        let home_dir = dirs::home_dir()?;
        let move_home_dir = home_dir.join(".move");
        let sanitized_repo_name: String = self
            .git
            .chars()
            .map(|ch| match ch {
                '/' | ':' | '.' | '@' => '_',
                _ => ch,
            })
            .collect();
        let rev_name = self.rev.clone()?.replace("/", "_");
        let dep_dir_name = format!("{sanitized_repo_name}_{rev_name}");
        let dep_root = move_home_dir
            .join(dep_dir_name)
            .join(self.subdir.clone().unwrap_or_default());
        let abs_dep_root = AbsPathBuf::try_from(Utf8PathBuf::from_path_buf(dep_root).ok()?).ok()?;
        Some(abs_dep_root)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum MoveTomlDependency {
    Local(LocalDependency),
    Git(GitDependency),
}

impl MoveTomlDependency {
    pub fn name(&self) -> String {
        match self {
            MoveTomlDependency::Local(local) => local.name.clone(),
            MoveTomlDependency::Git(git) => git.name.clone(),
        }
    }

    pub fn into_local(self) -> Option<LocalDependency> {
        match self {
            MoveTomlDependency::Local(local) => Some(local),
            _ => None,
        }
    }
    pub fn into_git(self) -> Option<GitDependency> {
        match self {
            MoveTomlDependency::Git(git) => Some(git),
            _ => None,
        }
    }

    pub fn dep_root(&self, current_pkg_root: &AbsPathBuf) -> Option<AbsPathBuf> {
        match self {
            MoveTomlDependency::Git(git_dep) => git_dep.dep_root(),
            MoveTomlDependency::Local(local_dep) => local_dep.dep_root(current_pkg_root),
        }
    }
}

#[derive(Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Package {
    pub name: Option<String>,
    pub version: Option<String>,
}

#[derive(Deserialize, Debug, Clone, Eq, PartialEq)]
struct DependencyTable {
    local: Option<String>,
    git: Option<String>,
    rev: Option<String>,
    subdir: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use paths::RelPath;

    #[test]
    fn test_parse_basic_move_toml_with_dependencies() {
        // language=Toml
        let source = r#"
[package]
name = "AptosStdlib"
version = "1.5.0"

[dependencies]
MoveStdlib = { local = "../move-stdlib" }

[dependencies.AptosFramework]
git = "https://github.com/aptos-labs/move-stdlib.git"
rev = "main"
        "#;

        let move_toml = MoveToml::from_str(source).unwrap();

        assert_eq!(move_toml.package.unwrap().name.unwrap(), "AptosStdlib");
        assert_eq!(move_toml.dependencies.len(), 2);

        let local_dep = move_toml
            .dependencies
            .iter()
            .find_map(|dep| dep.clone().into_local())
            .unwrap();
        assert_eq!(local_dep.name, "MoveStdlib");

        let git_dep = move_toml
            .dependencies
            .iter()
            .find_map(|dep| dep.clone().into_git())
            .unwrap();
        assert_eq!(git_dep.name, "AptosFramework");

        assert!(git_dep.dep_root().unwrap().ends_with(RelPath::new_unchecked(
            ".move/https___github_com_aptos-labs_move-stdlib_git_main/".into()
        )));
    }

    #[test]
    fn test_sibling_local_dependency() {
        // language=Toml
        let source = r#"
[dependencies.LiquidswapInit]
local = "./liquidswap_init/"
        "#;

        let move_toml = MoveToml::from_str(source).unwrap();

        let local_dep = move_toml
            .dependencies
            .iter()
            .find_map(|dep| dep.clone().into_local())
            .unwrap();
        assert_eq!(local_dep.name, "LiquidswapInit");
        assert_eq!(local_dep.path, "./liquidswap_init/")
    }
}
