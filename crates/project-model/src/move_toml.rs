use serde_derive::Deserialize;
use std::collections::HashMap;

#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct MoveToml {
    package: Option<Package>,
    dependencies: Vec<MoveTomlDependency>,
}

impl MoveToml {
    pub fn from_str(file_contents: &str) -> anyhow::Result<Self> {
        let mut move_toml = MoveToml::default();

        let deserialized = toml::from_str::<HashMap<String, toml::Value>>(file_contents)?;
        if let Some(package_table) = deserialized.get("package") {
            move_toml.package = package_table.to_owned().try_into().ok();
        }

        if let Some(deps_table) = deserialized.get("dependencies").and_then(|d| d.as_table()) {
            for (name, value) in deps_table {
                let table = value
                    .as_table()
                    .and_then(|table| table.to_owned().try_into::<'_, DependencyTable>().ok());
                if let Some(table) = table {
                    if let Some(local) = table.local {
                        move_toml
                            .dependencies
                            .push(MoveTomlDependency::Local(LocalDependency {
                                name: name.to_owned(),
                                path: local,
                            }));
                        continue;
                    }
                    if let Some(git) = table.git {
                        move_toml
                            .dependencies
                            .push(MoveTomlDependency::Git(GitDependency {
                                name: name.to_owned(),
                                git,
                                rev: table.rev,
                                subdir: table.subdir,
                            }));
                    }
                }
            }
        }

        Ok(move_toml)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LocalDependency {
    name: String,
    path: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GitDependency {
    name: String,
    git: String,
    rev: Option<String>,
    subdir: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum MoveTomlDependency {
    Local(LocalDependency),
    Git(GitDependency),
}

impl MoveTomlDependency {
    pub fn local(self) -> Option<LocalDependency> {
        match self {
            MoveTomlDependency::Local(local) => Some(local),
            _ => None,
        }
    }
    pub fn git(self) -> Option<GitDependency> {
        match self {
            MoveTomlDependency::Git(git) => Some(git),
            _ => None,
        }
    }
}

#[derive(Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Package {
    name: Option<String>,
    version: Option<String>,
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
            .find_map(|dep| dep.clone().local())
            .unwrap();
        assert_eq!(local_dep.name, "MoveStdlib");

        let git_dep = move_toml
            .dependencies
            .iter()
            .find_map(|dep| dep.clone().git())
            .unwrap();
        assert_eq!(git_dep.name, "AptosFramework");
    }
}
