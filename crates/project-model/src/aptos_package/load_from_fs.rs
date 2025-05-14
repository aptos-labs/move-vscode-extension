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
    kind: PackageKind,
    recurse_into_deps: bool,
    visited_manifests: &mut HashSet<ManifestPath>,
) -> anyhow::Result<AptosPackage> {
    if !recurse_into_deps {
        return Ok(AptosPackage::new(manifest_path, kind, vec![]));
    }

    let dep_roots = read_dependencies(manifest_path, kind)?;
    tracing::info!("dep_roots = {:#?}", dep_roots);

    let mut deps = vec![];
    for (dep_manifest, dep_kind) in read_dependencies(manifest_path, kind)? {
        if visited_manifests.contains(&dep_manifest) {
            tracing::info!("dep already visited {:?}", dep_manifest);
            continue;
        }
        visited_manifests.insert(dep_manifest.clone());

        let _p =
            tracing::info_span!("load dep package at", "{:?}", dep_manifest.canonical_root()).entered();
        if let Some(dep_package) = load_inner(&dep_manifest, dep_kind, true, visited_manifests).ok() {
            deps.push(dep_package);
        }
    }

    Ok(AptosPackage::new(manifest_path, kind, deps))
}

/// goes into dependencies and loads them too
pub(crate) fn read_dependencies(
    manifest_path: &ManifestPath,
    outer_kind: PackageKind,
) -> anyhow::Result<Vec<(ManifestPath, PackageKind)>> {
    let move_toml = read_manifest_from_fs(&manifest_path)?;

    let mut dep_manifests = vec![];
    let package_root = manifest_path.content_root();

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

    Ok(dep_manifests)
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
