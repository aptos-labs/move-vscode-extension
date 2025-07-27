// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::fixtures::{SourceFiles, parse_files_from_source};
use crate::testdir;
use ide::{Analysis, AnalysisHost};
use paths::{Utf8Path, Utf8PathBuf};
use project_model::DiscoveredManifest;
use project_model::aptos_package::load_from_fs;
use std::fs;
use vfs::{AbsPathBuf, FileId, Vfs, VfsPath};

pub fn prepare_directories(ws_root: &Utf8Path, test_packages: Vec<TestPackageFiles>) {
    let _ = fs::create_dir(ws_root);

    for test_package in test_packages {
        let package_root_dir = ws_root.join(test_package.root_dir);
        fs::create_dir(&package_root_dir).expect(&format!("Package root {package_root_dir} exists"));

        let move_toml_file = package_root_dir.join("Move.toml");
        let move_toml_contents = test_package.move_toml;
        fs::write(&move_toml_file, move_toml_contents).unwrap();

        let sources_dir = package_root_dir.join("sources");
        fs::create_dir(&sources_dir).unwrap();
        let tests_dir = package_root_dir.join("tests");
        fs::create_dir(&tests_dir).unwrap();

        let SourceFiles { sources, tests } = parse_files_from_source(&test_package.source_files);
        for (fpath, file_text) in sources {
            let fpath = sources_dir.join(fpath);
            fs::write(&fpath, file_text).unwrap();
        }
        for (fpath, file_text) in tests {
            let fpath = tests_dir.join(fpath);
            fs::write(&fpath, file_text).unwrap();
        }
    }
}

pub fn from_multiple_files_on_tmpfs(test_packages: Vec<TestPackageFiles>) -> TestState {
    let tmp = testdir::TestDir::new();

    let ws_root = tmp.path().to_path_buf();
    prepare_directories(&ws_root, test_packages);

    let discovered_manifests = DiscoveredManifest::discover_all(&[AbsPathBuf::assert(ws_root.clone())]);
    let all_packages = load_from_fs::load_aptos_packages(discovered_manifests)
        .into_iter()
        .filter_map(|it| it.ok())
        .collect::<Vec<_>>();

    let (db, vfs) = ide_db::load::load_db(all_packages.as_slice()).unwrap();

    let analysis_host = AnalysisHost::with_database(db);
    TestState { ws_root, vfs, analysis_host }
}

#[derive(Debug)]
pub struct TestPackageFiles {
    move_toml: String,
    root_dir: String,
    source_files: String,
}

pub fn named(name: &str, source_files: impl Into<String>) -> TestPackageFiles {
    // language=TOML
    TestPackageFiles {
        root_dir: name.to_string(),
        move_toml: stdx::trim_indent(&format!(
            r#"
[package]
name = "{name}"
version = "0.1.0"
        "#
        )),
        source_files: source_files.into(),
    }
}

pub fn raw(package_name: &str, root_dir: &str, source_files: &str) -> TestPackageFiles {
    // language=TOML
    TestPackageFiles {
        root_dir: root_dir.to_string(),
        move_toml: stdx::trim_indent(&format!(
            r#"
[package]
name = "{package_name}"
version = "0.1.0"
        "#
        )),
        source_files: source_files.to_string(),
    }
}

pub fn named_with_deps(name: &str, deps: &str, source_files: &str) -> TestPackageFiles {
    // language=TOML
    TestPackageFiles {
        root_dir: name.to_string(),
        move_toml: stdx::trim_indent(&format!(
            r#"
[package]
name = "{name}"
version = "0.1.0"

{deps}
        "#
        )),
        source_files: source_files.to_string(),
    }
}

pub struct TestState {
    pub ws_root: Utf8PathBuf,
    vfs: Vfs,
    analysis_host: AnalysisHost,
}

impl TestState {
    pub fn analysis(&self) -> Analysis {
        self.analysis_host.analysis()
    }

    pub fn relpath(&self, file_id: FileId) -> String {
        let file_path = self.vfs.file_path(file_id);
        if file_path.name_and_extension() == Some(("builtins", Some("move"))) {
            return "/builtins.move".into();
        }
        let file_path = file_path.as_path().unwrap();
        let package_metadata = self.analysis().package_metadata(file_id).unwrap().unwrap();
        let package_name = package_metadata.package_name.unwrap();
        let sources_dir = self.ws_root.join(package_name).join("sources");
        let relpath = file_path
            .strip_prefix(AbsPathBuf::assert(sources_dir).as_path())
            .unwrap()
            .as_str();
        format!("/{relpath}")
    }

    pub fn all_move_files(&self) -> Vec<(FileId, String)> {
        let analysis = self.analysis_host.analysis();
        let mut res = vec![];
        for (file_id, file_path) in self.vfs.iter() {
            if file_path.name_and_extension() == Some(("builtins", Some("move"))) {
                continue;
            }
            let file_text = analysis.file_text(file_id).unwrap().to_string();
            if analysis.package_metadata(file_id).unwrap().is_none() {
                // builtin
                continue;
            }
            if file_path
                .name_and_extension()
                .is_none_or(|(_, ext)| ext.is_none_or(|it| it != "move"))
            {
                continue;
            }
            res.push((file_id, file_text));
        }
        res
    }

    pub fn file_path(&self, file_id: FileId) -> &VfsPath {
        self.vfs.file_path(file_id)
    }

    pub fn file_with_caret(&self, caret: &str) -> (FileId, String) {
        let analysis = self.analysis_host.analysis();
        for (file_id, _) in self.vfs.iter() {
            let file_text = analysis.file_text(file_id).unwrap().to_string();
            if file_text.contains(caret) {
                return (file_id, file_text);
            }
        }
        panic!("file with {caret} is missing");
    }
}
