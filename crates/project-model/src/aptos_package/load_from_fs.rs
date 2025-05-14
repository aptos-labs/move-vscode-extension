use crate::aptos_package::{AptosPackage, PackageKind};
use crate::manifest_path::ManifestPath;
use crate::move_toml::{MoveToml, MoveTomlDependency};
use crate::{DiscoveredManifest, move_toml};
use anyhow::Context;
use paths::AbsPathBuf;
use std::collections::HashSet;
use std::fs;

pub fn load_aptos_packages(manifests: Vec<DiscoveredManifest>) -> Vec<anyhow::Result<AptosPackage>> {
    manifests
        .into_iter()
        .flat_map(|it| {
            let manifest_path = ManifestPath::new(it.move_toml_file.to_path_buf());
            load_from_manifest(&manifest_path, it.resolve_deps)
        })
        .collect::<Vec<_>>()
}

/// goes into dependencies and loads them too
pub fn load_from_manifest(
    root_manifest: &ManifestPath,
    recurse_into_deps: bool,
) -> Vec<anyhow::Result<AptosPackage>> {
    let _p = tracing::info_span!("load package at", "{:?}", root_manifest.canonical_root()).entered();
    let mut visited = HashSet::new();
    let package = load_inner(root_manifest, PackageKind::Local, recurse_into_deps, &mut visited)
        .with_context(|| format!("Failed to load the project at {root_manifest}"));
    vec![package]
}

fn load_inner(
    manifest_path: &ManifestPath,
    sourced_from: PackageKind,
    recurse_into_deps: bool,
    visited_manifests: &mut HashSet<AbsPathBuf>,
) -> anyhow::Result<AptosPackage> {
    let move_toml = read_manifest_from_fs(&manifest_path)?;

    let mut dep_roots = vec![];
    let mut dep_manifests = vec![];
    let package_root = manifest_path.root();

    if recurse_into_deps {
        for toml_dep in move_toml.dependencies.clone() {
            if let Some(dep_root) = toml_dep.dep_root(&package_root) {
                let Some(move_toml_path) = find_move_toml_at(dep_root) else {
                    continue;
                };

                if visited_manifests.contains(&move_toml_path) {
                    tracing::info!("dep already visited {:?}", move_toml_path);
                    continue;
                }
                visited_manifests.insert(move_toml_path.clone());

                let manifest_path = ManifestPath::new(move_toml_path);
                dep_roots.push(manifest_path.canonical_root());

                let sourced_from = match toml_dep {
                    MoveTomlDependency::Git(_) => PackageKind::Git,
                    MoveTomlDependency::Local(_) => PackageKind::Local,
                };
                dep_manifests.push((manifest_path, sourced_from));
            }
        }
        tracing::info!("dep_roots = {:#?}", dep_roots);
    }

    let deps = dep_manifests
        .into_iter()
        .filter_map(|(manifest, sourced_from)| {
            let _p =
                tracing::info_span!("load dep package at", "{:?}", manifest.canonical_root()).entered();
            load_inner(&manifest, sourced_from, recurse_into_deps, visited_manifests).ok()
        })
        .collect();

    Ok(AptosPackage {
        content_root: package_root,
        move_toml,
        sourced_from,
        deps: deps,
    })
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
