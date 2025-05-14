use crate::aptos_package::load_from_fs::read_dependencies;
use crate::aptos_package::{AptosPackage, PackageKind};
use crate::manifest_path::ManifestPath;
use crate::move_toml::MoveToml;
use std::collections::HashMap;
use std::fs;

type PackageEntries = HashMap<ManifestPath, (PackageKind, Vec<(ManifestPath, PackageKind)>)>;

fn load_all_packages(starting_manifest: &ManifestPath) -> Vec<AptosPackage> {
    let mut entries = HashMap::new();
    load_entries(&mut entries, starting_manifest.clone(), PackageKind::Local);

    // let mut deps = vec![];
    let mut packages = vec![];
    for (manifest, (kind, deps)) in entries.clone() {
        let mut transitive_deps = vec![];
        for dep in deps {
            collect_transitive_deps(dep, &mut transitive_deps, &entries);
        }
        // packages.push(AptosPackage::new(&manifest, transitive_deps, kind));
    }

    packages
}

fn collect_transitive_deps(
    current_dep: (ManifestPath, PackageKind),
    transitive_deps: &mut Vec<(ManifestPath, PackageKind)>,
    package_entries: &PackageEntries,
) -> Option<()> {
    transitive_deps.push(current_dep.clone());

    let (_, deps) = package_entries.get(&current_dep.0)?.clone();
    for dep in deps {
        collect_transitive_deps(dep, transitive_deps, package_entries);
    }
    Some(())
}

/// reads package and dependencies, collect all (Root, Vec<Root>, SourcedFrom) for all the reachable packages
fn load_entries(
    entries: &mut PackageEntries,
    manifest_path: ManifestPath,
    kind: PackageKind,
) -> Option<()> {
    let dep_manifests = read_dependencies(&manifest_path, kind).ok()?;
    entries.insert(manifest_path, (kind, dep_manifests.clone()));

    for (dep_manifest, dep_kind) in dep_manifests.clone() {
        load_entries(entries, dep_manifest, dep_kind);
    }
    Some(())
}

fn read_manifest_from_fs(path: &ManifestPath) -> Option<MoveToml> {
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => Some(contents),
        Err(err) => {
            tracing::error!(?path, ?err, "cannot read Move.toml file");
            None
        }
    }?;
    match MoveToml::from_str(contents.as_str()) {
        Ok(move_toml) => Some(move_toml),
        Err(err) => {
            tracing::error!(?path, ?err, "cannot deserialize Move.toml file");
            None
        }
    }
}
