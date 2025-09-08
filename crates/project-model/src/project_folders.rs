// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file contains code originally from rust-analyzer, licensed under Apache License 2.0.
// Modifications have been made to the original code.

use crate::aptos_package::{AptosPackage, PackageFolderRoot};
use base_db::package_root::{PackageKind, PackageRoot};
use std::fmt;
use std::fmt::Formatter;
use vfs::file_set::FileSetConfig;
use vfs::{FileId, VfsPath};

#[derive(Default)]
pub struct PackageRootConfig {
    pub fileset_root_manifests: Vec<VfsPath>,
    pub fsc: FileSetConfig,
    pub local_filesets: Vec<u64>,
}

impl PackageRootConfig {
    pub fn partition_into_package_roots(&self, vfs: &vfs::Vfs) -> Vec<PackageRoot> {
        let package_file_sets = self.fsc.partition(vfs);
        let mut package_roots = vec![];
        for (idx, package_file_set) in package_file_sets.into_iter().enumerate() {
            let is_local = self.local_filesets.contains(&(idx as u64));
            let kind = if is_local {
                PackageKind::Local
            } else {
                PackageKind::Library
            };
            let mut package_manifest_file_id: Option<FileId> = None;
            for candidate_manifest in self.fileset_root_manifests.iter() {
                if let Some(manifest_file_id) = package_file_set.file_for_path(&candidate_manifest) {
                    package_manifest_file_id = Some(*manifest_file_id);
                }
            }
            package_roots.push(PackageRoot::new(package_file_set, kind, package_manifest_file_id))
        }
        package_roots
    }
}

impl fmt::Debug for PackageRootConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("PackageRootConfig")
            .field("root_dirs", &self.fileset_root_manifests)
            .finish()
    }
}

#[derive(Default)]
pub struct ProjectFolders {
    pub load: Vec<vfs::loader::Entry>,
    pub watch: Vec<usize>,
    pub package_root_config: PackageRootConfig,
}

impl ProjectFolders {
    pub fn new(all_packages: &[AptosPackage]) -> ProjectFolders {
        let mut folders = ProjectFolders::default();
        let mut fsc = FileSetConfig::builder();
        let mut local_filesets = vec![];

        let mut all_folder_roots = all_packages
            .into_iter()
            .map(|pkg| pkg.to_folder_root())
            .collect::<Vec<_>>();
        // all_reachable_folder_roots.dedup_by(|a, b| a.canonical_form().eq(&b.canonical_form()));

        all_folder_roots.sort();
        all_folder_roots.dedup();

        let mut fileset_root_manifests = vec![];
        for package_folder_root in all_folder_roots {
            for dir_entry in folder_root_to_dir_entries(package_folder_root.clone()) {
                if package_folder_root.is_local {
                    folders.watch.push(folders.load.len());
                }
                folders.load.push(dir_entry);
            }

            if package_folder_root.is_local {
                local_filesets.push(fsc.len() as u64);
            }

            let fileset_root_manifest = VfsPath::from(package_folder_root.manifest_file.clone());
            fileset_root_manifests.push(fileset_root_manifest.clone());

            let fileset_root = fileset_root_manifest.parent().unwrap();
            fsc.add_file_set(vec![fileset_root])
        }

        let fsc = fsc.build();
        folders.package_root_config = PackageRootConfig {
            fileset_root_manifests,
            fsc,
            local_filesets,
        };

        folders
    }
}

fn folder_root_to_dir_entries(folder_root: PackageFolderRoot) -> Vec<vfs::loader::Entry> {
    let mut toml_dirs = vfs::loader::Directories::default();
    toml_dirs.extensions.push("toml".into());
    toml_dirs
        .include
        .extend(vec![folder_root.content_root().to_path_buf()]);

    let mut move_dirs = vfs::loader::Directories::default();
    move_dirs.extensions.push("move".into());
    move_dirs.include.extend(folder_root.source_dirs());
    vec![
        vfs::loader::Entry::Directories(toml_dirs),
        vfs::loader::Entry::Directories(move_dirs),
    ]
}
