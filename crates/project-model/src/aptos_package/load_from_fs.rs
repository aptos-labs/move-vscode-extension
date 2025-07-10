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
use std::collections::{HashMap, HashSet};
use std::fs;

#[derive(Debug, Clone)]
pub struct PackageEntry {
    package_name: Option<String>,
    kind: PackageKind,
    deps: Vec<(ManifestPath, PackageKind)>,
}

type PackageEntriesWithErrors = HashMap<ManifestPath, anyhow::Result<PackageEntry>>;

pub fn load_aptos_packages(manifests: Vec<DiscoveredManifest>) -> Vec<anyhow::Result<AptosPackage>> {
    let mut visited_package_roots = HashSet::new();
    let mut dedup = vec![];
    for manifest in manifests {
        let manifest_path = ManifestPath::new(manifest.move_toml_file.to_path_buf());
        let packages = load_reachable_aptos_packages(&manifest_path, manifest.resolve_deps);
        for package in packages {
            if let Ok(package) = &package {
                // dedup based on package root
                if visited_package_roots.contains(&package.content_root) {
                    continue;
                }
                visited_package_roots.insert(package.content_root.clone());
            }
            dedup.push(package);
        }
    }
    dedup
}

fn load_reachable_aptos_packages(
    starting_manifest: &ManifestPath,
    resolve_deps: bool,
) -> Vec<anyhow::Result<AptosPackage>> {
    let mut package_entries_with_errors = HashMap::new();
    let mut visited_manifests = HashSet::new();
    load_package_entries(
        &mut package_entries_with_errors,
        starting_manifest.clone(),
        PackageKind::Local,
        &mut visited_manifests,
        resolve_deps,
    );

    let valid_package_entries = package_entries_with_errors
        .iter()
        // error means that Move.toml is incorrect
        // todo: notification?
        .filter_map(|(k, entry)| {
            if !entry.is_ok() {
                return None;
            }
            let package_entry = entry.as_ref().cloned().unwrap();
            Some((k.clone(), package_entry))
        })
        .collect::<HashMap<_, _>>();

    let mut packages: Vec<anyhow::Result<AptosPackage>> = vec![];
    for (manifest, res) in package_entries_with_errors {
        match res {
            Err(err) => {
                packages.push(Err(err));
            }
            Ok(PackageEntry {
                package_name: name,
                kind,
                deps,
            }) => {
                let mut transitive_deps = vec![];
                let mut visited_manifests = HashSet::new();
                visited_manifests.insert(manifest.clone());

                let mut visited_transitive_manifests = HashSet::new();
                let mut visited_dep_names = HashSet::new();
                for dep in deps {
                    collect_transitive_deps(
                        dep.clone(),
                        &mut transitive_deps,
                        &valid_package_entries,
                        &mut visited_transitive_manifests,
                        &mut visited_dep_names,
                    );
                }
                packages.push(Ok(AptosPackage::new(
                    name,
                    &manifest,
                    kind,
                    transitive_deps,
                    resolve_deps,
                )));
            }
        }
    }

    packages
}

fn collect_transitive_deps(
    current_dep: (ManifestPath, PackageKind),
    transitive_deps: &mut Vec<(ManifestPath, PackageKind)>,
    package_entries: &HashMap<ManifestPath, PackageEntry>,
    visited_manifests: &mut HashSet<ManifestPath>,
    visited_dep_names: &mut HashSet<String>,
) {
    if visited_manifests.contains(&current_dep.0) {
        return;
    }
    visited_manifests.insert(current_dep.0.clone());

    match package_entries.get(&current_dep.0) {
        Some(PackageEntry {
            package_name: dep_package_name,
            deps,
            ..
        }) => {
            if let Some(dep_package_name) = dep_package_name {
                if visited_dep_names.contains(dep_package_name) {
                    return;
                }
                visited_dep_names.insert(dep_package_name.clone());
            }
            transitive_deps.push(current_dep.clone());
            for dep in deps {
                collect_transitive_deps(
                    dep.clone(),
                    transitive_deps,
                    package_entries,
                    visited_manifests,
                    visited_dep_names,
                );
            }
        }
        None => {
            tracing::error!(
                "cannot collect deps due to invalid Move.toml file: {}",
                current_dep.0
            );
            transitive_deps.push(current_dep.clone());
            return;
        }
    }
}

/// reads package and dependencies, collect all (Root, Vec<Root>, SourcedFrom) for all the reachable packages
fn load_package_entries(
    package_entries: &mut PackageEntriesWithErrors,
    manifest_path: ManifestPath,
    kind: PackageKind,
    visited_manifests: &mut HashSet<ManifestPath>,
    resolve_deps: bool,
) {
    if visited_manifests.contains(&manifest_path) {
        return;
    }
    visited_manifests.insert(manifest_path.clone());

    match read_manifest_from_fs(&manifest_path) {
        Ok(move_toml) => {
            let package_name = move_toml.package.as_ref().and_then(|it| it.name.clone());
            let dep_manifests = if resolve_deps {
                read_dependencies(manifest_path.content_root(), &move_toml, kind)
            } else {
                vec![]
            };
            package_entries.insert(
                manifest_path,
                Ok(PackageEntry {
                    package_name,
                    kind,
                    deps: dep_manifests.clone(),
                }),
            );
            for (dep_manifest, dep_kind) in dep_manifests.clone() {
                load_package_entries(package_entries, dep_manifest, dep_kind, visited_manifests, true);
            }
        }
        Err(manifest_parse_error) => {
            package_entries.insert(manifest_path, Err(manifest_parse_error));
        }
    }
}

/// goes into dependencies and loads them too
fn read_dependencies(
    package_root: AbsPathBuf,
    move_toml: &MoveToml,
    outer_kind: PackageKind,
) -> Vec<(ManifestPath, PackageKind)> {
    let mut dep_manifests = vec![];
    for toml_dep in move_toml.dependencies.clone() {
        if let Some(dep_root) = toml_dep.dep_root(&package_root) {
            let Some(move_toml_path) = find_move_toml_at(dep_root) else {
                continue;
            };

            let manifest_path = ManifestPath::new(move_toml_path);
            let dep_kind = match toml_dep {
                MoveTomlDependency::Git(_) => PackageKind::Git,
                MoveTomlDependency::Local(_) => PackageKind::Local,
            };
            dep_manifests.push((
                manifest_path,
                if outer_kind == PackageKind::Git {
                    outer_kind
                } else {
                    dep_kind
                },
            ));
        }
    }

    dep_manifests
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
