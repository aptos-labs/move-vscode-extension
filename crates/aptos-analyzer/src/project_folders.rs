use base_db::package_root::PackageRoot;
use paths::AbsPathBuf;
use project_model::aptos_package::{AptosPackage, PackageFolderRoot};
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fmt::Formatter;
use std::{fmt, mem};
use stdx::itertools::Itertools;
use vfs::VfsPath;
use vfs::file_set::FileSetConfig;

#[derive(Default)]
pub struct PackageRootConfig {
    pub fsc: FileSetConfig,
    pub local_filesets: Vec<u64>,
}

impl PackageRootConfig {
    pub fn partition_into_roots(&self, vfs: &vfs::Vfs) -> Vec<PackageRoot> {
        let package_file_sets = self.fsc.partition(vfs);
        package_file_sets
            .into_iter()
            .enumerate()
            .map(|(idx, package_file_set)| {
                let is_local = self.local_filesets.contains(&(idx as u64));
                if is_local {
                    PackageRoot::new_local(package_file_set)
                } else {
                    PackageRoot::new_library(package_file_set)
                }
            })
            .collect()
    }
}

impl fmt::Debug for PackageRootConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("PackageRootConfig")
            .field("fsc", &self.fsc)
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
    pub fn new(main_packages: &[AptosPackage]) -> ProjectFolders {
        let mut folders = ProjectFolders::default();
        let mut fsc = FileSetConfig::builder();
        let mut local_filesets = vec![];

        let mut folder_roots = main_packages
            .iter()
            .flat_map(|pkg| pkg.to_folder_roots())
            // .update(|root| root.include.sort())
            // .sorted_by(|a, b| a.include.cmp(&b.include))
            .collect::<Vec<_>>();
        folder_roots.dedup();

        for package_folder_root in folder_roots {
            for dir_entry in folder_root_to_dir_entries(package_folder_root.clone()) {
                if package_folder_root.is_local {
                    folders.watch.push(folders.load.len());
                }
                folders.load.push(dir_entry);
            }

            if package_folder_root.is_local {
                local_filesets.push(fsc.len() as u64);
            }
            let file_set_root = VfsPath::from(package_folder_root.content_root.clone());
            fsc.add_file_set(vec![file_set_root])
        }

        let fsc = fsc.build();
        folders.package_root_config = PackageRootConfig { fsc, local_filesets };

        folders
    }
}

fn folder_root_to_dir_entries(folder_root: PackageFolderRoot) -> Vec<vfs::loader::Entry> {
    let mut toml_dirs = vfs::loader::Directories::default();
    toml_dirs.extensions.push("toml".into());
    toml_dirs.include.extend(vec![folder_root.content_root.clone()]);

    let mut move_dirs = vfs::loader::Directories::default();
    move_dirs.extensions.push("move".into());
    move_dirs.include.extend(folder_root.source_dirs());
    vec![
        vfs::loader::Entry::Directories(toml_dirs),
        vfs::loader::Entry::Directories(move_dirs),
    ]
}
