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
use std::io::Read;

#[derive(Debug, Clone)]
pub struct ManifestEntry {
    package_name: Option<String>,
    kind: PackageKind,
    declared_deps: Vec<(ManifestPath, PackageKind)>,
    resolve_deps: bool,
}

pub fn load_aptos_packages(ws_manifests: Vec<DiscoveredManifest>) -> Vec<anyhow::Result<AptosPackage>> {
    let mut all_reachable_manifests = HashMap::new();
    for ws_manifest in ws_manifests.clone() {
        let manifest_path = ManifestPath::new(ws_manifest.move_toml_file.to_path_buf());
        let mut visited_manifests = HashSet::new();
        collect_reachable_manifests(
            manifest_path,
            &mut all_reachable_manifests,
            PackageKind::Local,
            &mut visited_manifests,
            ws_manifest.resolve_deps,
        );
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
            Ok(ManifestEntry {
                package_name,
                kind,
                declared_deps,
                resolve_deps,
            }) => {
                // for every reachable package in the workspace, we need to build it's dependencies
                let mut collected_deps = vec![];
                let mut visited_dep_manifests = HashSet::new();
                let mut visited_dep_names = HashSet::new();
                for dep in declared_deps {
                    collect_transitive_deps(
                        dep.clone(),
                        &mut collected_deps,
                        &valid_manifests,
                        &mut visited_dep_manifests,
                        &mut visited_dep_names,
                    );
                }
                all_reachable_aptos_packages.push(Ok(AptosPackage::new(
                    package_name.clone(),
                    &manifest_path,
                    kind.clone(),
                    collected_deps,
                    resolve_deps.clone(),
                )));
            }
            Err(manifest_parse_error) => {
                all_reachable_aptos_packages.push(Err(manifest_parse_error));
            }
        }
    }

    all_reachable_aptos_packages
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

/// reads package and dependencies, collect all (Root, Vec<Root>, SourcedFrom) for all the reachable packages
fn collect_reachable_manifests(
    manifest_path: ManifestPath,
    manifest_entries: &mut HashMap<ManifestPath, anyhow::Result<ManifestEntry>>,
    package_kind: PackageKind,
    visited_manifests: &mut HashSet<ManifestPath>,
    resolve_deps: bool,
) {
    if visited_manifests.contains(&manifest_path) {
        return;
    }
    visited_manifests.insert(manifest_path.clone());

    match read_manifest_from_fs(&manifest_path) {
        Err(err) => {
            manifest_entries.insert(manifest_path, Err(err));
        }
        Ok(move_toml) => {
            let package_name = move_toml.package.as_ref().and_then(|it| it.name.clone());
            if !resolve_deps {
                manifest_entries.insert(
                    manifest_path,
                    Ok(ManifestEntry {
                        package_name,
                        kind: package_kind,
                        declared_deps: vec![],
                        resolve_deps: false,
                    }),
                );
                return;
            }
            let dep_manifests =
                read_dependencies(manifest_path.content_root(), &move_toml, package_kind);
            manifest_entries.insert(
                manifest_path,
                Ok(ManifestEntry {
                    package_name,
                    kind: package_kind,
                    declared_deps: dep_manifests.clone(),
                    resolve_deps: true,
                }),
            );
            for (dep_manifest, dep_kind) in dep_manifests.clone() {
                collect_reachable_manifests(
                    dep_manifest,
                    manifest_entries,
                    dep_kind,
                    visited_manifests,
                    true,
                );
            }
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
    let all_toml_deps = move_toml
        .dependencies
        .iter()
        .chain(move_toml.dev_dependencies.iter());
    for toml_dep in all_toml_deps {
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
