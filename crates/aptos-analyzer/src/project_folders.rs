use base_db::input::SourceRoot;
use paths::AbsPathBuf;
use project_model::AptosWorkspace;
use project_model::aptos_workspace::PackageRoot;
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
    pub fn partition(&self, vfs: &vfs::Vfs) -> Vec<SourceRoot> {
        self.fsc
            .partition(vfs)
            .into_iter()
            .enumerate()
            .map(|(idx, file_set)| {
                let is_local = self.local_filesets.contains(&(idx as u64));
                if is_local {
                    SourceRoot::new_local(file_set)
                } else {
                    SourceRoot::new_library(file_set)
                }
            })
            .collect()
    }

    // /// Maps local source roots to their parent source roots by bytewise comparing of root paths .
    // /// If a `SourceRoot` doesn't have a parent and is local then it is not contained in this mapping but it can be asserted that it is a root `SourceRoot`.
    // pub fn source_root_parent_map(&self) -> FxHashMap<SourceRootId, SourceRootId> {
    //     let roots = self.fsc.roots();
    //
    //     let mut map = FxHashMap::default();
    //
    //     // See https://github.com/rust-lang/rust-analyzer/issues/17409
    //     //
    //     // We can view the connections between roots as a graph. The problem is
    //     // that this graph may contain cycles, so when adding edges, it is necessary
    //     // to check whether it will lead to a cycle.
    //     //
    //     // Since we ensure that each node has at most one outgoing edge (because
    //     // each SourceRoot can have only one parent), we can use a disjoint-set to
    //     // maintain the connectivity between nodes. If an edge’s two nodes belong
    //     // to the same set, they are already connected.
    //     let mut dsu = FxHashMap::default();
    //     fn find_parent(dsu: &mut FxHashMap<u64, u64>, id: u64) -> u64 {
    //         if let Some(&parent) = dsu.get(&id) {
    //             let parent = find_parent(dsu, parent);
    //             dsu.insert(id, parent);
    //             parent
    //         } else {
    //             id
    //         }
    //     }
    //
    //     for (idx, (root, root_id)) in roots.iter().enumerate() {
    //         if !self.local_filesets.contains(root_id) || map.contains_key(&SourceRootId(*root_id as u32))
    //         {
    //             continue;
    //         }
    //
    //         for (root2, root2_id) in roots[..idx].iter().rev() {
    //             if self.local_filesets.contains(root2_id)
    //                 && root_id != root2_id
    //                 && root.starts_with(root2)
    //             {
    //                 // check if the edge will create a cycle
    //                 if find_parent(&mut dsu, *root_id) != find_parent(&mut dsu, *root2_id) {
    //                     map.insert(SourceRootId(*root_id as u32), SourceRootId(*root2_id as u32));
    //                     dsu.insert(*root_id, *root2_id);
    //                 }
    //
    //                 break;
    //             }
    //         }
    //     }
    //
    //     map
    // }
}

#[derive(Default)]
pub struct ProjectFolders {
    pub load: Vec<vfs::loader::Entry>,
    pub watch: Vec<usize>,
    pub source_root_config: SourceRootConfig,
}

impl ProjectFolders {
    pub fn new(workspaces: &[AptosWorkspace], global_excludes: &[AbsPathBuf]) -> ProjectFolders {
        let mut res = ProjectFolders::default();
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

        let mut roots: Vec<_> = workspaces
            .iter()
            .flat_map(|ws| ws.to_roots())
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
            for (idx, root) in roots.iter().enumerate().filter(|(_, it)| !it.include.is_empty()) {
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
                        &mut roots[v],
                        PackageRoot {
                            is_local: false,
                            include: vec![],
                            exclude: vec![],
                        },
                    );
                    roots[k].is_local |= r.is_local;
                    roots[k].include.extend(r.include);
                    roots[k].exclude.extend(r.exclude);
                }
                roots[k].include.sort();
                roots[k].exclude.sort();
                roots[k].include.dedup();
                roots[k].exclude.dedup();
            }
        }

        for root in roots.into_iter().filter(|it| !it.include.is_empty()) {
            let file_set_roots: Vec<VfsPath> = root.include.iter().cloned().map(VfsPath::from).collect();

            let entry = {
                let mut dirs = vfs::loader::Directories::default();
                dirs.extensions.push("move".into());
                dirs.extensions.push("toml".into());
                dirs.include.extend(root.include);
                dirs.exclude.extend(root.exclude);
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

            if root.is_local {
                res.watch.push(res.load.len());
            }
            res.load.push(entry);

            if root.is_local {
                local_filesets.push(fsc.len() as u64);
            }
            fsc.add_file_set(file_set_roots)
        }

        let fsc = fsc.build();
        res.source_root_config = SourceRootConfig { fsc, local_filesets };

        res
    }
}
