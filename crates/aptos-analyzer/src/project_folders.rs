use base_db::package::PackageRoot;
use paths::AbsPathBuf;
use project_model::AptosWorkspace;
use project_model::aptos_workspace::PackageFolderRoot;
use rustc_hash::FxHashMap;
use std::collections::hash_map::Entry;
use std::mem;
use stdx::itertools::Itertools;
use vfs::VfsPath;
use vfs::file_set::FileSetConfig;

#[derive(Default, Debug)]
pub struct SourceRootConfig {
    pub fsc: FileSetConfig,
    pub local_filesets: Vec<u64>,
}

impl SourceRootConfig {
    pub fn partition_into_roots(&self, vfs: &vfs::Vfs) -> Vec<PackageRoot> {
        tracing::info!("partition with {:?}", self.fsc);
        self.fsc
            .partition(vfs)
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

#[derive(Default)]
pub struct ProjectFolders {
    pub load: Vec<vfs::loader::Entry>,
    pub watch: Vec<usize>,
    pub source_root_config: SourceRootConfig,
}

impl ProjectFolders {
    pub fn new(workspaces: &[AptosWorkspace], global_excludes: &[AbsPathBuf]) -> ProjectFolders {
        let mut folders = ProjectFolders::default();
        let mut fsc = FileSetConfig::builder();
        let mut local_filesets = vec![];

        // Dedup source roots
        // Depending on the project setup, we can have duplicated source roots, or for example in
        // the case of the rustc workspace, we can end up with two source roots that are almost the
        // same but not quite, like:
        // PackageRoot { is_local: false, include: [AbsPathBuf(".../rust/src/tools/miri/cargo-miri")], exclude: [] }
        // PackageRoot {
        //     is_local: true,
        //     include: [AbsPathBuf(".../rust/src/tools/miri/cargo-miri"), AbsPathBuf(".../rust/build/x86_64-pc-windows-msvc/stage0-tools/x86_64-pc-windows-msvc/release/build/cargo-miri-85801cd3d2d1dae4/out")],
        //     exclude: [AbsPathBuf(".../rust/src/tools/miri/cargo-miri/.git"), AbsPathBuf(".../rust/src/tools/miri/cargo-miri/target")]
        // }
        //
        // The first one comes from the explicit rustc workspace which points to the rustc workspace itself
        // The second comes from the rustc workspace that we load as the actual project workspace
        // These `is_local` differing in this kind of way gives us problems, especially when trying to filter diagnostics as we don't report diagnostics for external libraries.
        // So we need to deduplicate these, usually it would be enough to deduplicate by `include`, but as the rustc example shows here that doesn't work,
        // so we need to also coalesce the includes if they overlap.

        let mut folder_roots: Vec<_> = workspaces
            .iter()
            .flat_map(|ws| ws.to_folder_roots())
            .update(|root| root.include.sort())
            .sorted_by(|a, b| a.include.cmp(&b.include))
            .collect();

        // map that tracks indices of overlapping roots
        let mut overlap_map = FxHashMap::<_, Vec<_>>::default();
        let mut done = false;

        while !mem::replace(&mut done, true) {
            // maps include paths to indices of the corresponding root
            let mut include_to_idx = FxHashMap::default();
            // Find and note down the indices of overlapping roots
            for (idx, root) in folder_roots.iter().enumerate().filter(|(_, it)| !it.include.is_empty()) {
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
                folder_roots[k].exclude.sort();
                folder_roots[k].include.dedup();
                folder_roots[k].exclude.dedup();
            }
        }

        for folder_root in folder_roots.into_iter().filter(|it| !it.include.is_empty()) {
            let file_set_roots: Vec<VfsPath> = folder_root.include.iter().cloned().map(VfsPath::from).collect();

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
        folders.source_root_config = SourceRootConfig { fsc, local_filesets };

        folders
    }
}
