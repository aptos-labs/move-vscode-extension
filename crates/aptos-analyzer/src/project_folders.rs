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
        tracing::info!("partition with {:?}", self.fsc);
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
    pub fn new(packages: &[AptosPackage], global_excludes: &[AbsPathBuf]) -> ProjectFolders {
        let mut folders = ProjectFolders::default();
        let mut fsc = FileSetConfig::builder();
        let mut local_filesets = vec![];

        let mut folder_roots: Vec<_> = packages
            .iter()
            .flat_map(|pkg| pkg.to_folder_roots())
            .update(|root| root.include.sort())
            .sorted_by(|a, b| a.include.cmp(&b.include))
            .collect();

        // map that tracks indices of overlapping roots
        let mut overlap_map = HashMap::<_, Vec<_>>::default();
        let mut done = false;

        while !mem::replace(&mut done, true) {
            // maps include paths to indices of the corresponding root
            let mut include_to_idx: HashMap<&AbsPathBuf, usize> = HashMap::default();
            // Find and note down the indices of overlapping roots
            for (idx, root) in folder_roots
                .iter()
                .enumerate()
                .filter(|(_, it)| !it.include.is_empty())
            {
                for include in &root.include {
                    match include_to_idx.entry(include) {
                        Entry::Occupied(e) => {
                            overlap_map.entry(*e.get()).or_default().push(idx);
                        }
                        Entry::Vacant(e) => {
                            e.insert(idx);
                        }
                    }
                }
            }
            for (k, v) in overlap_map.drain() {
                done = false;
                for v in v {
                    let r = mem::replace(
                        &mut folder_roots[v],
                        PackageFolderRoot {
                            is_local: false,
                            include: vec![],
                            exclude: vec![],
                        },
                    );
                    folder_roots[k].is_local |= r.is_local;
                    folder_roots[k].include.extend(r.include);
                    folder_roots[k].exclude.extend(r.exclude);
                }

                folder_roots[k].include.sort();
                folder_roots[k].include.dedup();

                folder_roots[k].exclude.sort();
                folder_roots[k].exclude.dedup();
            }
        }

        for folder_root in folder_roots.into_iter().filter(|it| !it.include.is_empty()) {
            let file_set_roots: Vec<VfsPath> =
                folder_root.include.iter().cloned().map(VfsPath::from).collect();

            let entry = {
                let mut dirs = vfs::loader::Directories::default();
                dirs.extensions.push("move".into());
                dirs.extensions.push("toml".into());
                dirs.include.extend(folder_root.include);
                dirs.exclude.extend(folder_root.exclude);
                for excl in global_excludes {
                    if dirs
                        .include
                        .iter()
                        .any(|incl| incl.starts_with(excl) || excl.starts_with(incl))
                    {
                        dirs.exclude.push(excl.clone());
                    }
                }

                vfs::loader::Entry::Directories(dirs)
            };

            if folder_root.is_local {
                folders.watch.push(folders.load.len());
            }
            folders.load.push(entry);

            if folder_root.is_local {
                local_filesets.push(fsc.len() as u64);
            }
            fsc.add_file_set(file_set_roots)
        }

        let fsc = fsc.build();
        folders.package_root_config = PackageRootConfig { fsc, local_filesets };

        folders
    }
}
