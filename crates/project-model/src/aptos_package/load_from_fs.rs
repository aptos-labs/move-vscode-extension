// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::DiscoveredManifest;
use crate::aptos_package::{AptosPackage, PackageKind};
use crate::manifest_path::ManifestPath;
use crate::move_toml::{MoveToml, MoveTomlDependency};
use anyhow::Context;
use paths::{AbsPath, AbsPathBuf};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;

#[derive(Debug, Clone)]
pub struct ManifestEntry {
    package_name: Option<String>,
    kind: PackageKind,
    declared_deps: Vec<(ManifestPath, PackageKind)>,
    resolve_deps: bool,
    missing_dependencies: Vec<String>,
    named_addresses: Vec<String>,
}

#[derive(Debug)]
pub enum LoadedPackage {
    /// package is still valid, but some of the dependencies is not
    Package(AptosPackage),
    // PackageWithMissingDeps(AptosPackage, Vec<String>),
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
                // LoadedPackage::PackageWithMissingDeps(package, _) => Some(package.clone()),
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
        let reachable_manifests = collect_reachable_manifests(&ws_manifest);
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
        named_addresses,
    } = manifest_entry;
    // for every reachable package in the workspace, we need to build it's dependencies
    let collected_deps = collect_deps_transitively(declared_deps, valid_manifest_entries);

    let aptos_package = AptosPackage::new(
        package_name,
        &manifest_path,
        kind,
        collected_deps,
        resolve_deps,
        named_addresses,
        missing_dependencies,
    );
    LoadedPackage::Package(aptos_package)
    // if !missing_dependencies.is_empty() {
    //     LoadedPackage::PackageWithMissingDeps(aptos_package, missing_dependencies)
    // } else {
    // }
}

fn collect_deps_transitively(
    declared_deps: Vec<(ManifestPath, PackageKind)>,
    valid_manifest_entries: &HashMap<ManifestPath, ManifestEntry>,
) -> Vec<(ManifestPath, PackageKind)> {
    let mut visited_dep_manifests = HashSet::new();
    let mut visited_dep_names = HashSet::new();

    let mut deps_queue = VecDeque::new();
    deps_queue.extend(declared_deps);

    let mut res = vec![];
    while let Some(current_dep) = deps_queue.pop_front() {
        if visited_dep_manifests.contains(&current_dep.0) {
            continue;
        }
        visited_dep_manifests.insert(current_dep.0.clone());

        match valid_manifest_entries.get(&current_dep.0) {
            Some(ManifestEntry {
                package_name: current_dep_package_name,
                declared_deps: transitive_declared_deps,
                ..
            }) => {
                if let Some(current_dep_package_name) = current_dep_package_name {
                    if visited_dep_names.contains(current_dep_package_name) {
                        continue;
                    }
                    visited_dep_names.insert(current_dep_package_name.clone());
                }
                res.push(current_dep);
                for transitive_dep in transitive_declared_deps {
                    deps_queue.push_back(transitive_dep.clone());
                }
            }
            None => {
                tracing::error!(
                    "cannot collect deps due to invalid Move.toml file: {}",
                    current_dep.0
                );
                res.push(current_dep);
            }
        }
    }
    res
}

fn collect_reachable_manifests(
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
                            named_addresses: move_toml.declared_named_addresses(),
                        }),
                    );
                    continue;
                }
                let mut missing_dependencies = vec![];
                let package_root = manifest_path.content_root();
                let mut dep_manifests = vec![];
                for declared_toml_dep in move_toml.declared_dependencies() {
                    if let Some(dep_root) = declared_toml_dep.dep_root(&package_root) {
                        let dep_manifest_path = match try_find_move_toml_at_root(dep_root.as_path()) {
                            Some(move_toml_path) => ManifestPath::new(move_toml_path),
                            None => {
                                tracing::error!(
                                    ?dep_root,
                                    "cannot find Move.toml file in dependency root",
                                );
                                // record that fact to show notification to the user
                                missing_dependencies.push(declared_toml_dep.name());
                                continue;
                            }
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
                        named_addresses: move_toml.declared_named_addresses(),
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

pub fn try_find_move_toml_at_root(dep_root: &AbsPath) -> Option<AbsPathBuf> {
    // handles case-insensitivity
    let raw_move_toml_path = dep_root.join("Move.toml").normalize();
    if fs::metadata(&raw_move_toml_path).is_ok() {
        Some(raw_move_toml_path)
    } else {
        None
    }
}

fn read_manifest_from_fs(path: &ManifestPath) -> anyhow::Result<MoveToml> {
    let contents =
        fs::read_to_string(&path).with_context(|| format!("Failed to read Move.toml file {path}"))?;
    MoveToml::from_str(contents.as_str())
        .with_context(|| format!("Failed to deserialize Move.toml file {path}"))
}
