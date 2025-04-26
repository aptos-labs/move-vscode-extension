use crate::aptos_package::AptosPackage;
use crate::manifest_path::ManifestPath;
use anyhow::Context;
use base_db::change::PackageGraph;
use paths::{AbsPath, AbsPathBuf};
use vfs::FileId;

pub type FileLoader<'a> = &'a mut dyn for<'b> FnMut(&'b AbsPath) -> Option<FileId>;

/// `PackageRoot` describes a package root folder.
/// Which may be an external dependency, or a member of
/// the current workspace.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct PackageFolderRoot {
    /// Is from the local filesystem and may be edited
    pub is_local: bool,
    /// Directories to include
    pub include: Vec<AbsPathBuf>,
    /// Directories to exclude
    pub exclude: Vec<AbsPathBuf>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AptosWorkspace {
    main_package: AptosPackage,
}

impl AptosWorkspace {
    pub fn load(manifest: ManifestPath) -> anyhow::Result<AptosWorkspace> {
        let _p = tracing::info_span!("load ws", "{:?}", manifest.file.as_path().to_string()).entered();
        AptosWorkspace::load_inner(manifest.clone())
            .with_context(|| format!("Failed to load the project at {manifest}"))
    }

    fn load_inner(root_manifest: ManifestPath) -> anyhow::Result<AptosWorkspace> {
        // todo: run `aptos metadata` (see rust-analyzer for error handling and progress reporting)

        let main_package = AptosPackage::load(root_manifest, false)?;

        // todo: fetch package dependencies
        // todo: fetch declared named addresses

        Ok(AptosWorkspace { main_package })
    }

    pub fn workspace_root(&self) -> &AbsPath {
        self.main_package.content_root()
    }

    pub fn manifest_path(&self) -> ManifestPath {
        self.main_package.manifest()
    }

    pub fn manifest(&self) -> Option<ManifestPath> {
        self.main_package.manifest().into()
    }

    /// Returns the roots for the current `AptosWorkspace`
    /// The return type contains the path and whether or not
    /// the root is a member of the current workspace
    pub fn to_folder_roots(&self) -> Vec<PackageFolderRoot> {
        self.all_package_refs()
            .into_iter()
            .map(|it| it.to_folder_root())
            .collect()
    }

    pub fn to_package_graph(&self, load: FileLoader<'_>) -> Option<PackageGraph> {
        tracing::info!(
            "loading aptos workspace at {:?} into PackageGraph",
            self.main_package.content_root()
        );

        let mut package_graph = PackageGraph::default();
        for package_ref in self.all_package_refs() {
            let main_file_id = package_ref.load_manifest_file_id(load)?;
            let mut dep_ids = vec![];
            for dep_package_ref in package_ref.deps() {
                let dep_file_id = dep_package_ref.load_manifest_file_id(load)?;
                dep_ids.push(dep_file_id);
            }
            package_graph.insert(main_file_id, dep_ids);
        }

        Some(package_graph)
    }

    pub fn all_package_refs(&self) -> Vec<&AptosPackage> {
        package_refs(&self.main_package)
    }

    pub fn contains_file(&self, file_path: &AbsPath) -> bool {
        self.all_package_refs()
            .iter()
            .any(|pkg| pkg.contains_file(file_path))
    }
}

fn package_refs(package: &AptosPackage) -> Vec<&AptosPackage> {
    let mut refs = vec![package];
    for dep in package.deps() {
        refs.extend(package_refs(dep));
    }
    refs
}
