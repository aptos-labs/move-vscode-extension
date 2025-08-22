// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::aptos_package::{AptosPackage, VfsLoader};
use base_db::change::{ManifestFileId, PackageGraph};
use base_db::inputs::PackageMetadata;
use paths::AbsPath;
use std::sync::Arc;
use vfs::Vfs;

pub fn collect_initial(packages: &[AptosPackage], vfs: &mut Vfs) -> Option<PackageGraph> {
    let mut load = |path: &AbsPath| {
        let contents = std::fs::read(path).ok();
        let path = vfs::VfsPath::from(path.to_path_buf());
        vfs.set_file_contents(path.clone(), contents);
        vfs.file_id(&path)
            .and_then(|(file_id, excluded)| (excluded == vfs::FileExcluded::No).then_some(file_id))
    };
    collect(packages, &mut load)
}

pub fn collect(aptos_packages: &[AptosPackage], load: VfsLoader<'_>) -> Option<PackageGraph> {
    let _p = tracing::info_span!("dep_graph::collect").entered();

    let mut global_dep_graph = PackageGraph::default();

    for aptos_package in aptos_packages.iter() {
        let (package_file_id, dep_ids) = aptos_package.dep_graph_entry(load)?;
        global_dep_graph.insert(
            package_file_id,
            PackageMetadata {
                package_name: aptos_package.package_name.clone(),
                dep_manifest_ids: Arc::new(dep_ids),
                resolve_deps: aptos_package.resolve_deps,
                named_addresses: aptos_package.named_addresses.clone(),
            },
        );
    }

    Some(global_dep_graph)
}

impl AptosPackage {
    fn dep_graph_entry(&self, load: VfsLoader<'_>) -> Option<(ManifestFileId, Vec<ManifestFileId>)> {
        let package_file_id = load_package_file_id(self.content_root(), load)?;

        let mut dep_ids = vec![];
        for (dep_root, _) in self.dep_roots() {
            let dep_file_id = load_package_file_id(dep_root, load)?;
            dep_ids.push(dep_file_id);
        }

        Some((package_file_id, dep_ids))
    }
}

fn load_package_file_id(dep_root: &AbsPath, load_from_vfs: VfsLoader<'_>) -> Option<ManifestFileId> {
    let move_toml_file = dep_root.join("Move.toml");
    match load_from_vfs(&move_toml_file) {
        Some(file_id) => Some(file_id),
        None => {
            tracing::info!(?move_toml_file, "cannot load from filesystem");
            None
        }
    }
}
