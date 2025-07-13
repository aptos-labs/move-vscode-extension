// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

pub mod test_state;

pub use test_state::{TestState, from_multiple_files_on_tmpfs, prepare_directories};

use base_db::change::FileChanges;
use base_db::package_root::{PackageKind, PackageRoot};
use ide::{Analysis, AnalysisHost};
use lang::builtins_file::BUILTINS_FILE;
use regex::Regex;
use std::cell::Cell;
use std::collections::HashSet;
use vfs::file_set::FileSet;
use vfs::{FileId, VfsPath};

const BUILTINS_FILE_ID: FileId = FileId::from_raw(0);

pub fn from_single_file(text: impl Into<String>) -> (Analysis, FileId) {
    let text = text.into();
    let mut test_package = TestPackage::new();

    let mut changes = FileChanges::new();

    let mut file_set = FileSet::default();
    let file_id = test_package.new_file_id();
    file_set.insert(file_id, VfsPath::new_virtual_path("/main.move".to_owned()));

    changes.set_package_roots(vec![PackageRoot::new(file_set, PackageKind::Local, None)]);
    changes.change_file(file_id, Some(text));

    test_package.apply_changes(changes);

    (test_package.analysis(), file_id)
}

pub struct TestPackage {
    pub(crate) analysis_host: AnalysisHost,
    pub(crate) files: HashSet<FileId>,
    next_file_id: Cell<u32>,
}

impl TestPackage {
    pub fn new() -> TestPackage {
        let mut changes = FileChanges::new();
        changes.add_builtins_file(BUILTINS_FILE_ID, BUILTINS_FILE.to_string());

        let mut host = AnalysisHost::new();
        host.apply_change(changes);

        let mut files = HashSet::new();
        files.insert(BUILTINS_FILE_ID);

        TestPackage {
            analysis_host: host,
            files,
            next_file_id: Cell::new(1),
        }
    }

    pub fn file_with_caret(&self, caret: &str) -> (FileId, String) {
        for file_id in &self.files {
            let file_text = self.file_text(*file_id);
            if file_text.contains(caret) {
                return (*file_id, file_text);
            }
        }
        panic!("file with {caret} is missing");
    }

    pub fn analysis(&self) -> Analysis {
        self.analysis_host.analysis()
    }

    pub fn file_text(&self, file_id: FileId) -> String {
        self.analysis().file_text(file_id).unwrap().to_string()
    }

    pub(crate) fn apply_changes(&mut self, changes: FileChanges) {
        for (file_id, _) in &changes.files_changed {
            self.files.insert(file_id.to_owned());
        }
        self.analysis_host.apply_change(changes);
    }

    pub(crate) fn new_file_id(&self) -> FileId {
        let new_id = self.next_file_id.get();
        self.next_file_id.set(new_id + 1);
        FileId::from_raw(new_id)
    }
}

#[derive(Debug, Default)]
pub(crate) struct SourceFiles {
    sources: Vec<(String, String)>,
    tests: Vec<(String, String)>,
}

impl SourceFiles {
    pub(crate) fn add_file(&mut self, fpath: String, contents: String) {
        if fpath.starts_with("/sources/") {
            self.sources
                .push((fpath.trim_start_matches("/sources/").to_string(), contents));
            return;
        }
        if fpath.starts_with("/tests/") {
            self.tests
                .push((fpath.trim_start_matches("/tests/").to_string(), contents));
            return;
        }
        self.sources
            .push((fpath.trim_start_matches("/").to_string(), contents));
    }
}

fn parse_files_from_source(files_source: &str) -> SourceFiles {
    let files_source = stdx::trim_indent(files_source);

    let file_sep = Regex::new(r#"^\s*//- (\S+)\s*$"#).unwrap();

    let mut files = SourceFiles::default();
    let mut file_contents = vec![];
    let mut current_file_name: Option<String> = None;
    for line in files_source.lines() {
        let re = file_sep.captures(line);
        if let Some(groups) = re {
            if current_file_name.is_some() {
                files.add_file(current_file_name.unwrap().clone(), file_contents.join("\n"));
                file_contents = vec![];
            }
            current_file_name = groups.get(1).map(|it| it.as_str().to_string());
            continue;
        }
        if current_file_name.is_some() {
            file_contents.push(line);
        }
    }
    if current_file_name.is_some() {
        files.add_file(current_file_name.unwrap().clone(), file_contents.join("\n"))
    }

    files
}
