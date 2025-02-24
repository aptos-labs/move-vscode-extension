use la_arena::{Arena, Idx, RawIdx};
use rustc_hash::{FxHashMap, FxHashSet};
use std::{fmt, mem, ops};
use vfs::file_set::FileSet;
use vfs::{AnchoredPath, FileId, VfsPath};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SourceRootId(pub u32);

/// Files are grouped into source roots. A source root is a directory on the
/// file systems which is watched for changes. Typically it corresponds to a
/// Aptos package. Source roots *might* be nested: in this case, a file belongs to
/// the nearest enclosing source root. Paths to files are always relative to a
/// source root, and the analyzer does not know the root path of the source root at
/// all. So, a file from one source root can't refer to a file in another source
/// root by path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceRoot {
    /// Sysroot or crates.io library.
    ///
    /// Libraries are considered mostly immutable, this assumption is used to
    /// optimize salsa's query structure
    pub is_library: bool,
    file_set: FileSet,
}

impl SourceRoot {
    pub fn new_local(file_set: FileSet) -> SourceRoot {
        SourceRoot {
            is_library: false,
            file_set,
        }
    }

    pub fn new_library(file_set: FileSet) -> SourceRoot {
        SourceRoot {
            is_library: true,
            file_set,
        }
    }

    pub fn path_for_file(&self, file: &FileId) -> Option<&VfsPath> {
        self.file_set.path_for_file(file)
    }

    pub fn file_for_path(&self, path: &VfsPath) -> Option<&FileId> {
        self.file_set.file_for_path(path)
    }

    pub fn resolve_path(&self, path: AnchoredPath<'_>) -> Option<FileId> {
        self.file_set.resolve_path(path)
    }

    pub fn iter(&self) -> impl Iterator<Item = FileId> + '_ {
        self.file_set.iter()
    }
}

/// `CrateGraph` is a bit of information which turns a set of text files into a
/// number of Rust crates.
///
/// Each crate is defined by the `FileId` of its root module, the set of enabled
/// `cfg` flags and the set of dependencies.
///
/// Note that, due to cfg's, there might be several crates for a single `FileId`!
///
/// For the purposes of analysis, a crate does not have a name. Instead, names
/// are specified on dependency edges. That is, a crate might be known under
/// different names in different dependent crates.
///
/// Note that `CrateGraph` is build-system agnostic: it's a concept of the Rust
/// language proper, not a concept of the build system. In practice, we get
/// `CrateGraph` by lowering `cargo metadata` output.
///
/// `CrateGraph` is `!Serialize` by design, see
/// <https://github.com/rust-lang/rust-analyzer/blob/master/docs/dev/architecture.md#serialization>
#[derive(Clone, Default)]
pub struct CrateGraph {
    arena: Arena<CrateData>,
}

impl fmt::Debug for CrateGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map()
            .entries(
                self.arena
                    .iter()
                    .map(|(id, data)| (u32::from(id.into_raw()), data)),
            )
            .finish()
    }
}

pub type CrateId = Idx<CrateData>;

pub type CrateName = String;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrateData {
    pub root_file_id: FileId,
    /// The dependencies of this crate.
    ///
    /// Note that this may contain more dependencies than the crate actually uses.
    /// A common example is the test crate which is included but only actually is active when
    /// declared in source via `extern crate test`.
    pub dependencies: Vec<Dependency>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Dependency {
    pub name: CrateName,
    pub crate_id: CrateId,
}

impl Dependency {
    pub fn new(name: CrateName, crate_id: CrateId) -> Self {
        Self { name, crate_id }
    }
}

impl CrateGraph {
    pub fn add_crate_root(&mut self, root_file_id: FileId) {
        let data = CrateData {
            root_file_id,
            dependencies: Vec::new(),
        };
        self.arena.alloc(data);
    }

    pub fn add_dep(&mut self, from: CrateId, dep: Dependency) {
        let _p = tracing::info_span!("add_dep").entered();

        // // Check if adding a dep from `from` to `to` creates a cycle. To figure
        // // that out, look for a  path in the *opposite* direction, from `to` to
        // // `from`.
        // if let Some(path) = self.find_path(&mut FxHashSet::default(), dep.crate_id, from) {
        //     let path = path
        //         .into_iter()
        //         .map(|it| (it, self[it].display_name.clone()))
        //         .collect();
        //     let err = CyclicDependenciesError { path };
        //     assert!(err.from().0 == from && err.to().0 == dep.crate_id);
        //     return Err(err);
        // }

        self.arena[from].add_dep(dep);
    }

    pub fn is_empty(&self) -> bool {
        self.arena.is_empty()
    }

    pub fn len(&self) -> usize {
        self.arena.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = CrateId> + '_ {
        self.arena.iter().map(|(idx, _)| idx)
    }

    // FIXME: used for fixing up the toolchain sysroot, should be removed and done differently
    #[doc(hidden)]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (CrateId, &mut CrateData)> + '_ {
        self.arena.iter_mut()
    }

    /// Returns an iterator over all transitive dependencies of the given crate,
    /// including the crate itself.
    pub fn transitive_deps(&self, of: CrateId) -> impl Iterator<Item = CrateId> {
        let mut worklist = vec![of];
        let mut deps = FxHashSet::default();

        while let Some(krate) = worklist.pop() {
            if !deps.insert(krate) {
                continue;
            }

            worklist.extend(self[krate].dependencies.iter().map(|dep| dep.crate_id));
        }

        deps.into_iter()
    }

    /// Returns all transitive reverse dependencies of the given crate,
    /// including the crate itself.
    pub fn transitive_rev_deps(&self, of: CrateId) -> impl Iterator<Item = CrateId> {
        let mut worklist = vec![of];
        let mut rev_deps = FxHashSet::default();
        rev_deps.insert(of);

        let mut inverted_graph = FxHashMap::<_, Vec<_>>::default();
        self.arena.iter().for_each(|(krate, data)| {
            data.dependencies
                .iter()
                .for_each(|dep| inverted_graph.entry(dep.crate_id).or_default().push(krate))
        });

        while let Some(krate) = worklist.pop() {
            if let Some(krate_rev_deps) = inverted_graph.get(&krate) {
                krate_rev_deps
                    .iter()
                    .copied()
                    .filter(|&rev_dep| rev_deps.insert(rev_dep))
                    .for_each(|rev_dep| worklist.push(rev_dep));
            }
        }

        rev_deps.into_iter()
    }

    /// Returns all crates in the graph, sorted in topological order (ie. dependencies of a crate
    /// come before the crate itself).
    pub fn crates_in_topological_order(&self) -> Vec<CrateId> {
        let mut res = Vec::new();
        let mut visited = FxHashSet::default();

        for crate_ in self.iter() {
            go(self, &mut visited, &mut res, crate_);
        }

        return res;

        fn go(
            graph: &CrateGraph,
            visited: &mut FxHashSet<CrateId>,
            res: &mut Vec<CrateId>,
            source: CrateId,
        ) {
            if !visited.insert(source) {
                return;
            }
            for dep in graph[source].dependencies.iter() {
                go(graph, visited, res, dep.crate_id)
            }
            res.push(source)
        }
    }

    /// Extends this crate graph by adding a complete second crate
    /// graph and adjust the ids in the [`ProcMacroPaths`] accordingly.
    ///
    /// This will deduplicate the crates of the graph where possible.
    /// Furthermore dependencies are sorted by crate id to make deduplication easier.
    ///
    /// Returns a map mapping `other`'s IDs to the new IDs in `self`.
    pub fn extend(&mut self, mut other: CrateGraph) -> FxHashMap<CrateId, CrateId> {
        // Sorting here is a bit pointless because the input is likely already sorted.
        // However, the overhead is small and it makes the `extend` method harder to misuse.
        self.arena
            .iter_mut()
            .for_each(|(_, data)| data.dependencies.sort_by_key(|dep| dep.crate_id));

        let m = self.len();
        let topo = other.crates_in_topological_order();
        let mut id_map: FxHashMap<CrateId, CrateId> = FxHashMap::default();
        for topo in topo {
            let crate_data = &mut other.arena[topo];

            crate_data
                .dependencies
                .iter_mut()
                .for_each(|dep| dep.crate_id = id_map[&dep.crate_id]);
            crate_data.dependencies.sort_by_key(|dep| dep.crate_id);

            let find = self
                .arena
                .iter()
                .take(m)
                .find_map(|(k, v)| (v == crate_data).then_some(k));
            let new_id = find.unwrap_or_else(|| self.arena.alloc(crate_data.clone()));
            id_map.insert(topo, new_id);
        }

        id_map
    }

    fn find_path(
        &self,
        visited: &mut FxHashSet<CrateId>,
        from: CrateId,
        to: CrateId,
    ) -> Option<Vec<CrateId>> {
        if !visited.insert(from) {
            return None;
        }

        if from == to {
            return Some(vec![to]);
        }

        for dep in &self[from].dependencies {
            let crate_id = dep.crate_id;
            if let Some(mut path) = self.find_path(visited, crate_id, to) {
                path.push(from);
                return Some(path);
            }
        }

        None
    }

    /// Removes all crates from this crate graph except for the ones in `to_keep` and fixes up the dependencies.
    /// Returns a mapping from old crate ids to new crate ids.
    pub fn remove_crates_except(&mut self, to_keep: &[CrateId]) -> Vec<Option<CrateId>> {
        let mut id_map = vec![None; self.arena.len()];
        self.arena = mem::take(&mut self.arena)
            .into_iter()
            .filter_map(|(id, data)| {
                if to_keep.contains(&id) {
                    Some((id, data))
                } else {
                    None
                }
            })
            .enumerate()
            .map(|(new_id, (id, data))| {
                id_map[id.into_raw().into_u32() as usize] =
                    Some(CrateId::from_raw(RawIdx::from_u32(new_id as u32)));
                data
            })
            .collect();
        for (_, data) in self.arena.iter_mut() {
            data.dependencies.iter_mut().for_each(|dep| {
                dep.crate_id =
                    id_map[dep.crate_id.into_raw().into_u32() as usize].expect("crate was filtered")
            });
        }
        id_map
    }

    pub fn shrink_to_fit(&mut self) {
        self.arena.shrink_to_fit();
    }
}

impl ops::Index<CrateId> for CrateGraph {
    type Output = CrateData;
    fn index(&self, crate_id: CrateId) -> &CrateData {
        &self.arena[crate_id]
    }
}

impl CrateData {
    /// Add a dependency to `self` without checking if the dependency
    /// is existent among `self.dependencies`.
    fn add_dep(&mut self, dep: Dependency) {
        self.dependencies.push(dep)
    }
}
