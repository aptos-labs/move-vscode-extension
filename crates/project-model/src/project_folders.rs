use crate::aptos_package::{AptosPackage, PackageFolderRoot};
use base_db::package_root::PackageRoot;
use std::fmt;
use std::fmt::Formatter;
use std::path::PathBuf;
use vfs::VfsPath;
use vfs::file_set::FileSetConfig;

#[derive(Default)]
pub struct PackageRootConfig {
    pub fsc: FileSetConfig,
    pub local_filesets: Vec<u64>,
}

impl PackageRootConfig {
    pub fn partition_into_package_roots(&self, vfs: &vfs::Vfs) -> Vec<PackageRoot> {
        let package_file_sets = self.fsc.partition(vfs);
        let mut package_roots = vec![];
        for (idx, package_file_set) in package_file_sets.into_iter().enumerate() {
            let root_dir = self.package_dir(idx);
            let is_local = self.local_filesets.contains(&(idx as u64));
            let package_root = if is_local {
                PackageRoot::new_local(package_file_set, root_dir)
            } else {
                PackageRoot::new_library(package_file_set, root_dir)
            };
            package_roots.push(package_root);
        }
        package_roots
    }

    fn package_dir(&self, idx: usize) -> Option<String> {
        let root_bytes = self.fsc.roots().get(idx)?.0.clone();
        let root = String::from_utf8(root_bytes)
            .ok()?
            .trim_start_matches("\0")
            .to_string();
        PathBuf::from(root)
            .file_name()
            .map(|it| it.to_string_lossy().to_string())
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
    pub fn new(ws_packages: &[AptosPackage]) -> ProjectFolders {
        let mut folders = ProjectFolders::default();
        let mut fsc = FileSetConfig::builder();
        let mut local_filesets = vec![];

        let mut all_reachable_folder_roots = ws_packages
            .iter()
            .flat_map(|pkg| pkg.package_and_deps_folder_roots())
            .collect::<Vec<_>>();
        // all_reachable_folder_roots.dedup_by(|a, b| a.canonical_form().eq(&b.canonical_form()));

        all_reachable_folder_roots.sort();
        all_reachable_folder_roots.dedup();

        for package_folder_root in all_reachable_folder_roots {
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
