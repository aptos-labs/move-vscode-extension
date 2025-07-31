// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::DiscoveredManifest;
use crate::aptos_package::{AptosPackage, PackageKind};
use crate::manifest_path::ManifestPath;
use crate::move_toml::{MoveToml, MoveTomlDependency};
use anyhow::Context;
use paths::AbsPathBuf;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;

#[derive(Debug, Clone)]
pub struct ManifestEntry {
    package_name: Option<String>,
    kind: PackageKind,
    declared_deps: Vec<(ManifestPath, PackageKind)>,
    resolve_deps: bool,
    missing_dependencies: Vec<String>,
}

#[derive(Debug)]
pub enum LoadedPackage {
    /// package is still valid, but some of the dependencies is not
    Package(AptosPackage),
    PackageWithMissingDeps(AptosPackage, Vec<String>),
    ManifestParseError(anyhow::Error),
}

#[derive(Debug)]
pub struct LoadedPackages {
    pub packages: Vec<LoadedPackage>,
}

impl LoadedPackages {
    pub fn valid_packages(&self) -> Vec<AptosPackage> {
        self.packages
            .iter()
            .filter_map(|it| match it {
                LoadedPackage::Package(package) => Some(package.clone()),
                LoadedPackage::PackageWithMissingDeps(package, _) => Some(package.clone()),
                LoadedPackage::ManifestParseError(_) => None,
            })
            .collect()
    }

    pub fn display_error_for_tracing(&self) -> Option<String> {
        let mut buf = String::new();
        let packages_from_fs = &self.packages;
        if packages_from_fs.is_empty() {
            stdx::format_to!(buf, "aptos-language-server failed to find any packages");
        } else {
            for package_from_fs in packages_from_fs {
                match package_from_fs {
                    LoadedPackage::ManifestParseError(load_err) => {
                        stdx::format_to!(buf, "aptos-language-server error: {:#}\n", load_err);
                    }
                    // LoadedPackage::MissingDependencies(aptos_package, missing_dependencies) => {
                    //     let package_name = aptos_package.package_name.clone().unwrap_or_default();
                    //     if let Some(missing_dep) = missing_dependencies.first().cloned() {
                    //         stdx::format_to!(
                    //             buf,
                    //             "aptos-language-server error: missing dependencies {missing_dep:?} for package {package_name:?}\n",
                    //         );
                    //     }
                    // }
                    _ => (),
                }
            }
        }
        if buf.is_empty() {
            return None;
        }
        Some(buf)
    }
}

pub fn load_aptos_packages(ws_manifests: Vec<DiscoveredManifest>) -> LoadedPackages {
    let mut all_reachable_manifests = HashMap::new();
    for ws_manifest in ws_manifests.clone() {
        let reachable_manifests = collect_reachable_manifests_with_queue(&ws_manifest);
        all_reachable_manifests.extend(reachable_manifests);
    }

    let valid_manifests = all_reachable_manifests
        .iter()
        .filter_map(|(path, manifest_entry)| match manifest_entry {
            Ok(manifest_entry) => Some((path.clone(), manifest_entry.clone())),
            Err(_) => None,
        })
        .collect();

    let mut all_reachable_aptos_packages = vec![];
    for (manifest_path, manifest_entry) in all_reachable_manifests {
        match manifest_entry {
            Err(manifest_error) => {
                all_reachable_aptos_packages.push(LoadedPackage::ManifestParseError(manifest_error));
            }
            Ok(manifest_entry) => {
                // for every reachable package in the workspace, we need to build it's dependencies
                let loaded_package =
                    collect_package_from_manifest_entry(manifest_path, manifest_entry, &valid_manifests);
                all_reachable_aptos_packages.push(loaded_package);
            }
        }
    }

    LoadedPackages {
        packages: all_reachable_aptos_packages,
    }
}

fn collect_package_from_manifest_entry(
    manifest_path: ManifestPath,
    manifest_entry: ManifestEntry,
    valid_manifest_entries: &HashMap<ManifestPath, ManifestEntry>,
) -> LoadedPackage {
    let ManifestEntry {
        package_name,
        kind,
        declared_deps,
        resolve_deps,
        missing_dependencies,
    } = manifest_entry;
    // for every reachable package in the workspace, we need to build it's dependencies
    let mut collected_deps = vec![];
    let mut visited_dep_manifests = HashSet::new();
    let mut visited_dep_names = HashSet::new();
    for dep in declared_deps {
        collect_transitive_deps(
            dep.clone(),
            &mut collected_deps,
            &valid_manifest_entries,
            &mut visited_dep_manifests,
            &mut visited_dep_names,
        );
    }
    let aptos_package = AptosPackage::new(
        package_name.clone(),
        &manifest_path,
        kind.clone(),
        collected_deps,
        resolve_deps.clone(),
    );
    if !missing_dependencies.is_empty() {
        LoadedPackage::PackageWithMissingDeps(aptos_package, missing_dependencies)
    } else {
        LoadedPackage::Package(aptos_package)
    }
}

fn collect_transitive_deps(
    current_dep: (ManifestPath, PackageKind),
    collected_transitive_deps: &mut Vec<(ManifestPath, PackageKind)>,
    package_entries: &HashMap<ManifestPath, ManifestEntry>,
    visited_dep_manifests: &mut HashSet<ManifestPath>,
    visited_package_names: &mut HashSet<String>,
) {
    if visited_dep_manifests.contains(&current_dep.0) {
        return;
    }
    visited_dep_manifests.insert(current_dep.0.clone());

    match package_entries.get(&current_dep.0) {
        Some(ManifestEntry {
            package_name: current_dep_package_name,
            declared_deps,
            ..
        }) => {
            if let Some(current_dep_package_name) = current_dep_package_name {
                if visited_package_names.contains(current_dep_package_name) {
                    return;
                }
                visited_package_names.insert(current_dep_package_name.clone());
            }
            collected_transitive_deps.push(current_dep.clone());
            for dep in declared_deps {
                collect_transitive_deps(
                    dep.clone(),
                    collected_transitive_deps,
                    package_entries,
                    visited_dep_manifests,
                    visited_package_names,
                );
            }
        }
        None => {
            tracing::error!(
                "cannot collect deps due to invalid Move.toml file: {}",
                current_dep.0
            );
            collected_transitive_deps.push(current_dep.clone());
            return;
        }
    }
}

fn collect_reachable_manifests_with_queue(
    ws_manifest: &DiscoveredManifest,
) -> HashMap<ManifestPath, anyhow::Result<ManifestEntry>> {
    let mut packages_queue = VecDeque::new();
    packages_queue.push_back((
        ManifestPath::new(ws_manifest.move_toml_file.to_path_buf()),
        PackageKind::Local,
        ws_manifest.resolve_deps,
    ));
    let mut res = HashMap::new();
    while let Some((manifest_path, outer_package_kind, resolve_deps)) = packages_queue.pop_front() {
        if res.contains_key(&manifest_path) {
            continue;
        }
        match read_manifest_from_fs(&manifest_path) {
            Err(invalid_toml_err) => {
                res.insert(manifest_path, Err(invalid_toml_err));
            }
            Ok(move_toml) => {
                let package_name = move_toml.package.as_ref().and_then(|it| it.name.clone());
                if !resolve_deps {
                    res.insert(
                        manifest_path,
                        Ok(ManifestEntry {
                            package_name,
                            kind: outer_package_kind,
                            declared_deps: vec![],
                            resolve_deps: false,
                            missing_dependencies: vec![],
                        }),
                    );
                    continue;
                }
                let mut missing_dependencies = vec![];
                let package_root = manifest_path.content_root();
                let mut dep_manifests = vec![];
                for declared_toml_dep in move_toml.declared_dependencies() {
                    if let Some(dep_root) = declared_toml_dep.dep_root(&package_root) {
                        let dep_manifest_path = match find_move_toml_at(dep_root) {
                            None => {
                                // record that fact to show notification to the user
                                missing_dependencies.push(declared_toml_dep.name());
                                continue;
                            }
                            Some(move_toml_path) => ManifestPath::new(move_toml_path),
                        };
                        // local dependency of remote git package is still a git dependency
                        let dep_package_kind = match declared_toml_dep {
                            MoveTomlDependency::Git(_) => PackageKind::Git,
                            MoveTomlDependency::Local(_) if outer_package_kind == PackageKind::Git => {
                                PackageKind::Git
                            }
                            MoveTomlDependency::Local(_) => PackageKind::Local,
                        };
                        dep_manifests.push((dep_manifest_path, dep_package_kind));
                    }
                }
                res.insert(
                    manifest_path,
                    Ok(ManifestEntry {
                        package_name,
                        kind: outer_package_kind,
                        declared_deps: dep_manifests.clone(),
                        resolve_deps: true,
                        missing_dependencies,
                    }),
                );
                for (dep_manifest, dep_kind) in dep_manifests.clone() {
                    packages_queue.push_back((dep_manifest, dep_kind, true));
                }
            }
        }
    }
    res
}

fn find_move_toml_at(dep_root: AbsPathBuf) -> Option<AbsPathBuf> {
    let raw_move_toml_path = dep_root.join("Move.toml");
    let move_toml_path = match fs::canonicalize(&raw_move_toml_path) {
        Ok(path) => Some(AbsPathBuf::assert_utf8(path)),
        Err(_) => {
            tracing::error!(
                ?raw_move_toml_path,
                "dependency resolution error: path does not exist",
            );
            return None;
        }
    };
    move_toml_path
}

fn read_manifest_from_fs(path: &ManifestPath) -> anyhow::Result<MoveToml> {
    let contents =
        fs::read_to_string(&path).with_context(|| format!("Failed to read Move.toml file {path}"))?;
    MoveToml::from_str(contents.as_str())
        .with_context(|| format!("Failed to deserialize Move.toml file {path}"))
}
