// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::aptos_package::PackageFolderRoot;
use crate::aptos_package::load_from_fs::try_find_move_toml_at_root;
use paths::AbsPathBuf;
use std::fs;

pub mod aptos_package;
pub mod dep_graph;
pub mod manifest_path;
pub mod move_toml;
pub mod project_folders;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct DiscoveredManifest {
    pub move_toml_file: AbsPathBuf,
    pub resolve_deps: bool,
}

impl DiscoveredManifest {
    pub fn new(path: AbsPathBuf, resolve_deps: bool) -> Self {
        DiscoveredManifest {
            move_toml_file: path,
            resolve_deps,
        }
    }

    pub fn discover_all(ws_roots: &[AbsPathBuf]) -> Vec<DiscoveredManifest> {
        let mut all_manifests = vec![];
        for ws_root in ws_roots {
            let manifests = walk_and_discover_manifests(ws_root);
            all_manifests.extend(manifests);
        }
        all_manifests.sort();
        all_manifests.dedup();
        all_manifests
    }

    pub fn discover_for_file(fpath: &AbsPathBuf) -> Option<DiscoveredManifest> {
        let mut candidate_dir = fpath.parent()?;
        let candidate_manifest = loop {
            if let Some(move_toml_file) = try_find_move_toml_at_root(candidate_dir) {
                break DiscoveredManifest::new(move_toml_file, true);
            }
            candidate_dir = candidate_dir.parent()?;
        };
        let folder_root = PackageFolderRoot {
            manifest_file: candidate_manifest.move_toml_file.clone(),
            is_local: true,
        };
        if folder_root
            .source_dirs()
            .iter()
            .any(|source_dir| fpath.starts_with(source_dir))
        {
            return Some(candidate_manifest);
        }
        None
    }

    pub fn content_root(&self) -> AbsPathBuf {
        self.move_toml_file.parent().unwrap().to_path_buf()
    }

    pub fn display_root(&self) -> String {
        self.move_toml_file
            .parent()
            .map(|it| it.to_string())
            .expect("Move.toml file should have a parent")
    }
}

fn walk_and_discover_manifests(ws_root: &AbsPathBuf) -> Vec<DiscoveredManifest> {
    let candidate = ws_root.join("aptos-move").join("framework");
    let aptos_core_dirs = match fs::exists(&candidate) {
        Ok(true) => {
            let aptos_core_dirs = vec![
                ws_root.join("aptos-move").join("framework"),
                ws_root.join("aptos-move").join("move-examples"),
                ws_root
                    .join("testsuite")
                    .join("module-publish")
                    .join("src")
                    .join("packages"),
            ];
            let dirs_to_resolve = aptos_core_dirs
                .clone()
                .into_iter()
                .map(|it| it.to_string())
                .collect::<Vec<_>>();
            tracing::error!(
                "aptos-core repository detected, dependency resolution is restricted to {:#?}",
                dirs_to_resolve,
            );
            Some(aptos_core_dirs)
        }
        _ => None,
    };

    let mut manifests = vec![];
    for entry in walkdir::WalkDir::new(ws_root)
        .into_iter()
        .filter_map(|it| it.ok())
    {
        let path = AbsPathBuf::assert_utf8(entry.into_path());
        let resolve_deps = aptos_core_dirs
            .as_ref()
            .is_none_or(|dirs| dirs.iter().any(|dir| path.starts_with(dir)));
        if let Some(move_toml_file) = try_find_move_toml_at_root(path.as_path()) {
            manifests.push(DiscoveredManifest::new(move_toml_file, resolve_deps));
        }
    }
    manifests
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum InvocationStrategy {
    Once,
    #[default]
    PerWorkspace,
}
