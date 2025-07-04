// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::init_tracing_for_test;
use paths::AbsPathBuf;
use project_model::DiscoveredManifest;
use project_model::aptos_package::load_from_fs::load_aptos_packages;
use std::env::current_dir;
use std::path::PathBuf;

fn load_deps_dir() -> PathBuf {
    current_dir().unwrap().join("resources").join("load_deps")
}

#[test]
fn test_circular_dependencies() {
    init_tracing_for_test();

    let manifest = DiscoveredManifest {
        move_toml_file: AbsPathBuf::assert_utf8(
            load_deps_dir().join("circular_dependencies").join("Move.toml"),
        ),
        resolve_deps: true,
    };
    load_aptos_packages(vec![manifest]);
}

#[test]
fn test_if_inside_aptos_core_only_load_deps_from_aptos_move() {
    init_tracing_for_test();

    let root = load_deps_dir().join("aptos-core");
    let ws_roots = vec![AbsPathBuf::assert_utf8(root)];
    let discovered_manifests = DiscoveredManifest::discover_all(&ws_roots);

    let packages = load_aptos_packages(discovered_manifests)
        .into_iter()
        .map(|it| it.unwrap())
        .collect::<Vec<_>>();
    assert_eq!(packages.len(), 4);

    let other_movestdlib_package = packages
        .iter()
        .find(|it| it.content_root().to_string().contains("other-move"))
        .unwrap();
    assert_eq!(other_movestdlib_package.dep_roots().len(), 0);

    let aptos_stdlib = packages
        .iter()
        .find(|it| it.content_root().to_string().contains("aptos-stdlib"))
        .unwrap();
    assert_eq!(aptos_stdlib.dep_roots().len(), 1);

    let my_package = packages
        .iter()
        .find(|it| it.content_root().to_string().contains("my-package"))
        .unwrap();
    assert_eq!(my_package.dep_roots().len(), 1);
}
